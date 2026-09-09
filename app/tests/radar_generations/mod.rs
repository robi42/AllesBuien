//! The radar port `radar_controller` scripts across three consecutive runs.
//!
//! The harness it serves is about one thing: a replacement refresh must cancel
//! and finish dropping its predecessor before touching the shared cache. The
//! second metadata fetch therefore stays pending forever and records its own
//! cancellation, while the third records when replacement work actually starts.
//!
//! This lives beside its one consumer rather than in `allesbuien-testkit`.
//! Nothing else wants a radar that contradicts itself on purpose, and a double
//! shared by a single test is a double whose expectations have drifted out of
//! sight of the assertions they exist to serve.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use allesbuien_core::radar::{RadarApi, RadarError, RadarFuture};

struct PendingBytes {
    ordering: Arc<Mutex<Vec<String>>>,
}

impl Future for PendingBytes {
    type Output = Result<Vec<u8>, RadarError>;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl Drop for PendingBytes {
    fn drop(&mut self) {
        self.ordering
            .lock()
            .expect("ordering log lock should be healthy")
            .push("metadata-stale-cancelled".into());
    }
}

pub(crate) struct FakeRadarApi {
    metadata_calls: AtomicUsize,
    ordering: Arc<Mutex<Vec<String>>>,
}

impl FakeRadarApi {
    pub(crate) fn new() -> Self {
        Self {
            metadata_calls: AtomicUsize::new(0),
            ordering: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn metadata_calls(&self) -> usize {
        self.metadata_calls.load(Ordering::SeqCst)
    }

    pub(crate) fn recorded(&self) -> Vec<String> {
        self.ordering
            .lock()
            .expect("ordering log lock should be healthy")
            .clone()
    }

    fn metadata(run: &str, label: &str, count: usize) -> Vec<u8> {
        let times = (0..count)
            .map(|index| {
                let minute = 5 * index;
                format!(
                    r#"{{"timestamp":"2026-08-30T09:{minute:02}:00","url":"fake://{label}/{index}"}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        // A zero vendor offset is deliberately *not* what deriving these August
        // stamps would produce, so an ignored `timeOffset` shifts the caption.
        format!(r#"{{"timestamp":"{run}","timeOffset":0,"times":[{times}]}}"#).into_bytes()
    }

    fn record(&self, event: impl Into<String>) {
        self.ordering
            .lock()
            .expect("ordering log lock should be healthy")
            .push(event.into());
    }
}

impl RadarApi for FakeRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        let call = self.metadata_calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.record(format!("metadata-{call}"));
        match call {
            1 => Box::pin(async { Ok(Self::metadata("202608300900", "initial", 2)) }),
            2 => Box::pin(PendingBytes {
                ordering: Arc::clone(&self.ordering),
            }),
            _ => Box::pin(async { Ok(Self::metadata("202608300910", "fresh", 3)) }),
        }
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        self.record(format!("frame-{url}"));
        Box::pin(async { Ok(allesbuien::BASEMAP_PNG.to_vec()) })
    }
}
