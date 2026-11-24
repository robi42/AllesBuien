//! A radar run: the frames Buienradar published together, and the cache holding them.
//!
//! Buienradar publishes the animation as metadata plus one PNG per timestamp, so
//! a "refresh" is a small download storm rather than a single request. Frames are
//! written under a directory named for the run, which makes the cache trivially
//! resumable — a second refresh of the same run downloads nothing — and makes
//! pruning a matter of dropping whole directories rather than tracking files.
//!
//! [`GenerationId`] is the app's answer to the other half of that: a refresh
//! carries the generation it was started for, so a reply that arrives after the
//! viewer has moved on can be recognised and dropped instead of repainting the
//! window with frames from a run nobody is looking at any more.
//!
//! The network itself lives in [`crate::radar_transport`], behind [`RadarApi`].

use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

use serde::Deserialize;

use crate::{bounded_body::BodyError, radar_cache::RadarCache};

/// Runs kept on disk once a run has downloaded. A run holds at most
/// [`MAX_RUN_FRAMES`] frames, so at the 94 KiB frames on record the cache comes
/// to about 9 MiB, and at the transport's 2 MiB frame limit to at most 192 MiB.
/// Pruning runs only after a run completes, so a run that fails is not counted
/// here until a later one does.
pub const MAX_CACHED_RUNS: usize = 3;

/// The most frames one run accepts: the window the preferred metadata query
/// asks for, two observed steps and thirty predicted.
///
/// A longer `times[]` is cut to its first this-many entries in metadata order
/// before any frame is requested, so a vendor that lists more degrades the
/// radar rather than blanking it, and one run accepts at most 64 MiB under the
/// transport's frame limit. The stale-run fallback asks for a smaller window,
/// and one cap serves both: the vendor counts a window its own way, the run's
/// own frame included or not (`history=0&forecast=24` once answered with 21
/// frames), so a cap read off the fallback's parameters could cut the frame
/// furthest ahead, the one that lets the radar reach the rain graph's end.
pub const MAX_RUN_FRAMES: usize = 32;

pub type RadarFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, RadarError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RadarFrame {
    pub timestamp: String,
    pub path: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RadarRefresh {
    pub generation: GenerationId,
    pub frames: Vec<RadarFrame>,
    /// Buienradar's own `timeOffset`: the UTC offset in whole hours it publishes
    /// its Dutch wall clock at, or [`None`] when the run's metadata omits it.
    pub time_offset_hours: Option<i32>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RadarError {
    #[error("radar endpoint returned HTTP {status}")]
    HttpStatus { status: u16 },
    #[error("radar request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("radar response exceeded its {limit}-byte limit")]
    BodyTooLarge { limit: usize },
    #[error("radar metadata is invalid: {0}")]
    Metadata(#[from] serde_json::Error),
    #[error("radar metadata has an invalid run timestamp: {timestamp}")]
    InvalidRunTimestamp { timestamp: String },
    #[error("radar cache operation failed: {0}")]
    Cache(#[from] std::io::Error),
}

impl From<BodyError> for RadarError {
    fn from(error: BodyError) -> Self {
        match error {
            BodyError::TooLarge { limit } => Self::BodyTooLarge { limit },
            BodyError::Transport(error) => Self::Request(error),
        }
    }
}

pub trait RadarApi: Send + Sync {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>>;
    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>>;
}

pub struct RadarClient {
    api: Arc<dyn RadarApi>,
    state_dir: PathBuf,
}

impl RadarClient {
    #[must_use]
    pub fn new(api: Arc<dyn RadarApi>, state_dir: PathBuf) -> Self {
        Self { api, state_dir }
    }

    /// Downloads one metadata-defined run and tags it with the caller's generation.
    ///
    /// # Errors
    /// Returns a typed transport, metadata, or cache error.
    pub async fn refresh(&self, generation: GenerationId) -> Result<RadarRefresh, RadarError> {
        let metadata: Metadata = serde_json::from_slice(&self.api.metadata().await?)?;
        let run_id = run_directory_id(&metadata.timestamp)?;

        let time_offset_hours = metadata.time_offset;
        let frames = RadarCache::new(&self.state_dir)
            .materialize(self.api.as_ref(), &run_id, metadata.times)
            .await?;
        Ok(RadarRefresh {
            generation,
            frames,
            time_offset_hours,
        })
    }
}

/// Names a run's cache directory after the first twelve digits of its timestamp.
///
/// `2026-08-29T21:15:00` becomes `202608292115`, which sorts the way the runs
/// were published; a timestamp without twelve digits cannot name a directory.
fn run_directory_id(timestamp: &str) -> Result<String, RadarError> {
    let run_id: String = timestamp
        .chars()
        .filter(char::is_ascii_digit)
        .take(12)
        .collect();
    if run_id.len() == 12 {
        Ok(run_id)
    } else {
        Err(RadarError::InvalidRunTimestamp {
            timestamp: timestamp.to_owned(),
        })
    }
}

#[derive(Deserialize)]
struct Metadata {
    timestamp: String,
    #[serde(rename = "timeOffset", default)]
    time_offset: Option<i32>,
    times: Vec<MetadataTime>,
}

#[derive(Deserialize)]
pub(crate) struct MetadataTime {
    pub(crate) timestamp: String,
    pub(crate) url: String,
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::tempdir;

    use super::*;
    use crate::test_support::{StaticRadarApi, block_on};

    /// Lists `count` frames five minutes apart from noon, and records the URL
    /// of every frame it is asked for.
    struct ListingRadarApi {
        metadata: Vec<u8>,
        requested: Mutex<Vec<String>>,
    }

    impl ListingRadarApi {
        fn listing(count: usize) -> Self {
            let times: Vec<String> = (0..count)
                .map(|index| {
                    let timestamp = frame_timestamp(index);
                    format!(
                        r#"{{"timestamp":"{timestamp}","url":"{}"}}"#,
                        frame_url(index)
                    )
                })
                .collect();
            let metadata = format!(
                r#"{{"timestamp":"2026-08-29T12:00:00","times":[{}]}}"#,
                times.join(",")
            );
            Self {
                metadata: metadata.into_bytes(),
                requested: Mutex::new(Vec::new()),
            }
        }
    }

    impl RadarApi for ListingRadarApi {
        fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
            Box::pin(async move { Ok(self.metadata.clone()) })
        }

        fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
            Box::pin(async move {
                self.requested
                    .lock()
                    .expect("request log")
                    .push(url.to_owned());
                Ok(b"png".to_vec())
            })
        }
    }

    fn frame_timestamp(index: usize) -> String {
        let minutes = 12 * 60 + 5 * index;
        format!("2026-08-29T{:02}:{:02}:00", minutes / 60, minutes % 60)
    }

    fn frame_url(index: usize) -> String {
        format!("fake://frame-{index}")
    }

    const METADATA: &[u8] = br#"{
        "timestamp":"2026-08-29T21:15:00",
        "times":[
            {"timestamp":"2026-08-29T21:15:00","url":"https://image.example/frame-a"},
            {"timestamp":"2026-08-29T21:20:00","url":"https://image.example/frame-b"},
            {"timestamp":"2026-08-29T21:25:00","url":"https://image.example/frame-c"}
        ]
    }"#;

    #[test]
    fn run_directories_are_named_by_the_first_twelve_timestamp_digits() {
        // Given / When / Then
        assert_eq!(
            run_directory_id("2026-08-29T21:15:00").expect("a full timestamp"),
            "202608292115"
        );
        assert!(matches!(
            run_directory_id("2026-08-29"),
            Err(RadarError::InvalidRunTimestamp { timestamp }) if timestamp == "2026-08-29"
        ));
    }

    #[test]
    fn refresh_returns_ordered_metadata_frames_when_metadata_succeeds() {
        // Given
        let state = tempdir().expect("temp state");
        let api = Arc::new(StaticRadarApi::serving(METADATA));
        let client = RadarClient::new(api, state.path().to_path_buf());

        // When
        let refreshed = block_on(client.refresh(GenerationId(7))).expect("refresh succeeds");

        // Then
        assert_eq!(refreshed.generation, GenerationId(7));
        let expected = [
            ("2026-08-29T21:15:00", "frame-0.png"),
            ("2026-08-29T21:20:00", "frame-1.png"),
            ("2026-08-29T21:25:00", "frame-2.png"),
        ];
        assert_eq!(refreshed.frames.len(), expected.len());
        for (frame, (timestamp, file)) in refreshed.frames.iter().zip(expected) {
            assert_eq!(frame.timestamp, timestamp);
            assert!(frame.path.ends_with(file), "{}", frame.path.display());
            assert!(
                frame.path.is_file(),
                "{} was never written",
                frame.path.display()
            );
        }
    }

    #[test]
    fn refresh_skips_downloads_when_frames_are_cached() {
        // Given
        let state = tempdir().expect("temp state");
        let api = Arc::new(StaticRadarApi::serving(METADATA));
        let client = RadarClient::new(api.clone(), state.path().to_path_buf());
        block_on(client.refresh(GenerationId(1))).expect("first refresh succeeds");

        // When
        block_on(client.refresh(GenerationId(2))).expect("second refresh succeeds");

        // Then
        assert_eq!(api.downloads(), 3);
    }

    #[test]
    fn refresh_surfaces_typed_error_and_leaves_cache_untouched_when_metadata_is_500() {
        // Given
        let state = tempdir().expect("temp state");
        let api = Arc::new(StaticRadarApi::failing_metadata(500));
        let client = RadarClient::new(api, state.path().to_path_buf());

        // When
        let error = block_on(client.refresh(GenerationId(1))).expect_err("refresh fails");

        // Then
        assert!(matches!(error, RadarError::HttpStatus { status: 500 }));
        assert!(!state.path().join("runs").exists());
    }

    #[test]
    fn refresh_cuts_an_overlong_listing_to_its_earliest_frames_before_downloading() {
        // Given: metadata listing three frames more than a run accepts.
        let state = tempdir().expect("temp state");
        let api = Arc::new(ListingRadarApi::listing(MAX_RUN_FRAMES + 3));
        let client = RadarClient::new(api.clone(), state.path().to_path_buf());

        // When
        let refreshed = block_on(client.refresh(GenerationId(1))).expect("refresh succeeds");

        // Then: the first frames in metadata order, and none past them asked for.
        let timestamps: Vec<String> = refreshed
            .frames
            .iter()
            .map(|frame| frame.timestamp.clone())
            .collect();
        let expected: Vec<String> = (0..MAX_RUN_FRAMES).map(frame_timestamp).collect();
        assert_eq!(timestamps, expected);
        let mut requested = api.requested.lock().expect("request log").clone();
        requested.sort_unstable();
        let mut first: Vec<String> = (0..MAX_RUN_FRAMES).map(frame_url).collect();
        first.sort_unstable();
        assert_eq!(requested, first);
        let run = state.path().join("runs/202608291200");
        assert!(run.join("frame-31.png").is_file());
        assert!(!run.join("frame-32.png").exists());
    }
}
