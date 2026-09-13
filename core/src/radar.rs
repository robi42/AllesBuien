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

use crate::radar_cache::RadarCache;

/// Runs kept on disk. Frames measure 94 KiB and a run holds 26 of them, so this
/// bounds the cache at roughly 5.5 MiB.
pub const MAX_CACHED_RUNS: usize = 3;

pub type RadarFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, RadarError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RadarFrame {
    pub timestamp: String,
    pub url: String,
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
    #[error("radar metadata is invalid: {0}")]
    Metadata(#[from] serde_json::Error),
    #[error("radar metadata has an invalid run timestamp: {timestamp}")]
    InvalidRunTimestamp { timestamp: String },
    #[error("radar cache operation failed: {0}")]
    Cache(#[from] std::io::Error),
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
    use std::sync::Arc;

    use tempfile::tempdir;

    use super::*;
    use crate::test_support::{StaticRadarApi, block_on};

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
            ("2026-08-29T21:15:00", "https://image.example/frame-a"),
            ("2026-08-29T21:20:00", "https://image.example/frame-b"),
            ("2026-08-29T21:25:00", "https://image.example/frame-c"),
        ];
        for (frame, expected) in refreshed.frames.iter().zip(expected) {
            assert_eq!((frame.timestamp.as_str(), frame.url.as_str()), expected);
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
}
