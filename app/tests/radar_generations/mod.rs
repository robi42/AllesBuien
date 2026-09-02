//! The radar port `radar_controller` scripts across three consecutive runs.
//!
//! The harness it serves is about one thing: a frame that arrives late must not
//! be allowed to overwrite a newer generation. Proving that needs a radar whose
//! second run can be held open while a third overtakes it, so [`FakeRadarApi`]
//! answers differently on every metadata call — two frames, then one behind a
//! [`Gate`], then three — and logs the order every completion actually landed
//! in, which is the ordering the test reads back.
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
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use allesbuien_core::radar::{RadarApi, RadarError, RadarFuture};

/// A one-way switch that a pending future parks its waker on.
pub(crate) struct Gate {
    open: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl Gate {
    pub(crate) fn new() -> Self {
        Self {
            open: AtomicBool::new(false),
            waker: Mutex::new(None),
        }
    }

    pub(crate) fn release(&self) {
        self.open.store(true, Ordering::SeqCst);
        if let Ok(mut slot) = self.waker.lock()
            && let Some(waker) = slot.take()
        {
            waker.wake();
        }
    }
}

struct GatedBytes {
    gate: Arc<Gate>,
    bytes: Option<Vec<u8>>,
}

impl Future for GatedBytes {
    type Output = Result<Vec<u8>, RadarError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.gate.open.load(Ordering::SeqCst) {
            return Poll::Ready(Ok(self.bytes.take().unwrap_or_default()));
        }
        if let Ok(mut slot) = self.gate.waker.lock() {
            *slot = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

pub(crate) struct FakeRadarApi {
    metadata_calls: AtomicUsize,
    stale_gate: Arc<Gate>,
    ordering: Mutex<Vec<String>>,
}

impl FakeRadarApi {
    pub(crate) fn new(stale_gate: Arc<Gate>) -> Self {
        Self {
            metadata_calls: AtomicUsize::new(0),
            stale_gate,
            ordering: Mutex::new(Vec::new()),
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
            2 => Box::pin(GatedBytes {
                gate: Arc::clone(&self.stale_gate),
                bytes: Some(Self::metadata("202608300905", "stale", 1)),
            }),
            _ => Box::pin(async { Ok(Self::metadata("202608300910", "fresh", 3)) }),
        }
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        self.record(format!("frame-{url}"));
        Box::pin(async { Ok(allesbuien::BASEMAP_PNG.to_vec()) })
    }
}
