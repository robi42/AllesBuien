//! The scaffolding this crate's unit tests share: a polling executor, a fixed
//! environment, `weather.json` writers, fixed instants and errors, and a radar
//! fake.
//!
//! `allesbuien-core` is pure Rust with no async runtime of its own: reqwest
//! brings one for the application, and taking tokio as a dev-dependency to drive
//! futures that complete without a reactor would buy a thread pool nothing
//! here needs. Every future these tests await is hand-written against a fake and
//! completes synchronously, so polling it to completion is the whole
//! requirement — and one home for that loop is why the transport's tests did not
//! arrive carrying a second copy of it.

use std::{
    future::Future,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

use jiff::Timestamp;

use crate::{
    config::EnvSource,
    location::OMARCHY_WEATHER_SETTING,
    radar::{RadarApi, RadarError, RadarFuture},
};

/// A fixed environment table, so no test has to mutate the process's own.
pub(crate) struct FakeEnv<const N: usize>(pub(crate) [(&'static str, &'static str); N]);

impl<const N: usize> EnvSource for FakeEnv<N> {
    fn var(&self, key: &str) -> Option<String> {
        self.0
            .iter()
            .find_map(|(candidate, value)| (*candidate == key).then(|| (*value).to_owned()))
    }
}

/// Writes Omarchy's `weather.json` under a throwaway home directory.
pub(crate) fn write_weather(home: &Path, json: &str) {
    let path = home.join(OMARCHY_WEATHER_SETTING);
    std::fs::create_dir_all(path.parent().expect("weather parent")).expect("create settings");
    std::fs::write(path, json).expect("write weather fixture");
}

/// Removes it again, as `omarchy-weather-location --clear` does.
pub(crate) fn clear_weather(home: &Path) {
    std::fs::remove_file(home.join(OMARCHY_WEATHER_SETTING)).expect("remove weather fixture");
}

/// Drives a future to completion on the calling thread.
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 2 September 2026, 15:50 Amsterdam — the instant the live radar sample was taken.
///
/// Written as an offset-qualified instant rather than a zone name so the
/// assertions that read it mean the same thing in every zone the matrix runs.
pub(crate) fn sample_instant() -> Timestamp {
    "2026-09-02T15:50:00+02:00"
        .parse()
        .expect("a literal instant should parse")
}

/// A genuine `serde_json` decode failure, for error variants that carry one.
pub(crate) fn json_decode_error() -> serde_json::Error {
    serde_json::from_str::<u8>("not json").expect_err("fixture must be malformed")
}

/// A radar endpoint that answers every request from memory and counts downloads.
pub(crate) struct StaticRadarApi {
    metadata: &'static [u8],
    metadata_status: Option<u16>,
    downloads: AtomicUsize,
}

impl StaticRadarApi {
    /// Serves `metadata` and a placeholder PNG body for every frame.
    pub(crate) const fn serving(metadata: &'static [u8]) -> Self {
        Self {
            metadata,
            metadata_status: None,
            downloads: AtomicUsize::new(0),
        }
    }

    /// Refuses the metadata request with an HTTP `status`, as a failing endpoint does.
    pub(crate) const fn failing_metadata(status: u16) -> Self {
        Self {
            metadata: b"",
            metadata_status: Some(status),
            downloads: AtomicUsize::new(0),
        }
    }

    /// How many frame bodies have been requested so far.
    pub(crate) fn downloads(&self) -> usize {
        self.downloads.load(Ordering::Relaxed)
    }
}

impl RadarApi for StaticRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async move {
            match self.metadata_status {
                Some(status) => Err(RadarError::HttpStatus { status }),
                None => Ok(self.metadata.to_vec()),
            }
        })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async move {
            self.downloads.fetch_add(1, Ordering::Relaxed);
            Ok(b"png".to_vec())
        })
    }
}
