//! Regression coverage for persistent, fail-fast, bounded frame materialisation.

use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use tempfile::tempdir;

use crate::{
    radar::{GenerationId, RadarApi, RadarClient, RadarError, RadarFuture},
    test_support::block_on,
};

const ONE_FRAME_METADATA: &[u8] = br#"{
    "timestamp":"2026-08-29T21:15:00",
    "times":[
        {"timestamp":"2026-08-29T21:15:00","url":"fake://frame-0"}
    ]
}"#;
const OLDEST_RUN_METADATA: &[u8] = br#"{
    "timestamp":"2020-01-01T00:00:00",
    "times":[
        {"timestamp":"2020-01-01T00:00:00","url":"fake://frame-0"}
    ]
}"#;
const OUT_OF_ORDER_METADATA: &[u8] = br#"{
    "timestamp":"2026-08-29T21:15:00",
    "times":[
        {"timestamp":"2026-08-29T21:15:00","url":"fake://frame-0"},
        {"timestamp":"2026-08-29T21:20:00","url":"fake://frame-1"},
        {"timestamp":"2026-08-29T21:25:00","url":"fake://frame-2"},
        {"timestamp":"2026-08-29T21:30:00","url":"fake://frame-3"},
        {"timestamp":"2026-08-29T21:35:00","url":"fake://frame-4"},
        {"timestamp":"2026-08-29T21:40:00","url":"fake://frame-5"}
    ]
}"#;

struct StaticRadarApi {
    metadata: &'static [u8],
    downloads: AtomicUsize,
}

impl StaticRadarApi {
    const fn serving(metadata: &'static [u8]) -> Self {
        Self {
            metadata,
            downloads: AtomicUsize::new(0),
        }
    }
}

impl RadarApi for StaticRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async { Ok(self.metadata.to_vec()) })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async move {
            self.downloads.fetch_add(1, Ordering::Relaxed);
            Ok(b"png".to_vec())
        })
    }
}

struct DelayedRadarApi {
    active: AtomicUsize,
    max_active: AtomicUsize,
    completions: Mutex<Vec<usize>>,
}

impl DelayedRadarApi {
    const fn new() -> Self {
        Self {
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
            completions: Mutex::new(Vec::new()),
        }
    }
}

impl RadarApi for DelayedRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async { Ok(OUT_OF_ORDER_METADATA.to_vec()) })
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        let index = url
            .rsplit('-')
            .next()
            .expect("fixture URL has an index")
            .parse()
            .expect("fixture index is numeric");
        Box::pin(DelayedFrame {
            api: self,
            index,
            polls_remaining: 4 - index % 4,
            active: false,
        })
    }
}

struct DelayedFrame<'a> {
    api: &'a DelayedRadarApi,
    index: usize,
    polls_remaining: usize,
    active: bool,
}

impl Future for DelayedFrame<'_> {
    type Output = Result<Vec<u8>, RadarError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let frame = self.get_mut();
        if !frame.active {
            let active = frame.api.active.fetch_add(1, Ordering::Relaxed) + 1;
            frame.api.max_active.fetch_max(active, Ordering::Relaxed);
            frame.active = true;
        }
        if frame.polls_remaining > 0 {
            frame.polls_remaining -= 1;
            context.waker().wake_by_ref();
            return Poll::Pending;
        }
        frame
            .api
            .completions
            .lock()
            .expect("completion lock")
            .push(frame.index);
        frame.api.active.fetch_sub(1, Ordering::Relaxed);
        frame.active = false;
        Poll::Ready(Ok(frame.index.to_string().into_bytes()))
    }
}

impl Drop for DelayedFrame<'_> {
    fn drop(&mut self) {
        if self.active {
            self.api.active.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

fn delayed_client(state: &Path) -> (Arc<DelayedRadarApi>, RadarClient) {
    let api = Arc::new(DelayedRadarApi::new());
    let client = RadarClient::new(api.clone(), state.to_path_buf());
    (api, client)
}

#[test]
fn refresh_keeps_metadata_order_when_downloads_finish_out_of_order() {
    // Given
    let state = tempdir().expect("temp state");
    let (api, client) = delayed_client(state.path());

    // When
    let refresh = block_on(client.refresh(GenerationId(1))).expect("refresh succeeds");

    // Then
    let timestamps: Vec<&str> = refresh
        .frames
        .iter()
        .map(|frame| frame.timestamp.as_str())
        .collect();
    assert_eq!(
        timestamps,
        [
            "2026-08-29T21:15:00",
            "2026-08-29T21:20:00",
            "2026-08-29T21:25:00",
            "2026-08-29T21:30:00",
            "2026-08-29T21:35:00",
            "2026-08-29T21:40:00",
        ]
    );
    assert_ne!(
        *api.completions.lock().expect("completion lock"),
        [0, 1, 2, 3, 4, 5],
        "the fake must prove completion really differed from metadata order"
    );
}

#[test]
fn refresh_keeps_exactly_four_frame_requests_in_flight() {
    // Given
    let state = tempdir().expect("temp state");
    let (api, client) = delayed_client(state.path());

    // When
    block_on(client.refresh(GenerationId(1))).expect("refresh succeeds");

    // Then
    assert_eq!(api.max_active.load(Ordering::Relaxed), 4);
}

#[test]
fn refresh_replaces_an_interrupted_temporary_frame_atomically() {
    // Given
    let state = tempdir().expect("temp state");
    let run_dir = state.path().join("runs/202608292115");
    std::fs::create_dir_all(&run_dir).expect("run directory");
    let pending = run_dir.join("frame-0.png.tmp");
    std::fs::write(&pending, b"partial").expect("interrupted frame");
    let api = Arc::new(StaticRadarApi::serving(ONE_FRAME_METADATA));
    let client = RadarClient::new(api, state.path().to_path_buf());

    // When
    block_on(client.refresh(GenerationId(1))).expect("refresh succeeds");

    // Then
    assert_eq!(
        std::fs::read(run_dir.join("frame-0.png")).expect("committed frame"),
        b"png"
    );
    assert!(!pending.exists(), "temporary frame should be renamed");
}

#[test]
fn refresh_never_prunes_the_run_it_just_materialised() {
    // Given
    let state = tempdir().expect("temp state");
    let runs = state.path().join("runs");
    for run_id in ["202601010000", "202601020000", "202601030000"] {
        std::fs::create_dir_all(runs.join(run_id)).expect("cached run");
    }
    let api = Arc::new(StaticRadarApi::serving(OLDEST_RUN_METADATA));
    let client = RadarClient::new(api, state.path().to_path_buf());

    // When
    block_on(client.refresh(GenerationId(1))).expect("refresh succeeds");

    // Then
    assert!(
        runs.join("202001010000/frame-0.png").exists(),
        "the current run must be excluded from pruning candidates"
    );
}
