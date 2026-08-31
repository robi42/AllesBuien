use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::{
    config::LocationConfig,
    datetime::LocalTimestamp,
    notify::{Notifier, NotifyError},
    radar::{RadarApi, RadarFuture},
    weather::{FeedApi, FeedFuture, WeatherData},
};

struct Gate {
    open: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl Gate {
    fn new() -> Self {
        Self {
            open: AtomicBool::new(false),
            waker: Mutex::new(None),
        }
    }

    fn release(&self) {
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
    type Output = Result<Vec<u8>, allesbuien_core::radar::RadarError>;

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

struct FakeRadarApi {
    metadata_calls: AtomicUsize,
    stale_gate: Arc<Gate>,
    ordering: Mutex<Vec<String>>,
}

impl FakeRadarApi {
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

struct StubFeedApi;

impl FeedApi for StubFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: None,
                forecast: Vec::new(),
                now_line: String::new(),
                rain_line: String::new(),
                rain_soon: None,
                rain_series: Vec::new(),
            })
        })
    }
}

struct StubNotifier;

impl Notifier for StubNotifier {
    fn send(&self, _title: &str, _body: &str) -> Result<(), NotifyError> {
        Ok(())
    }
}

/// Asserts the frame caption is a wall-clock reading rather than the raw feed
/// stamp. The exact digits depend on the host timezone — including half-hour
/// zones — so only the shape is pinned here; `app/src/radar_clock.rs` owns the
/// host-independent assertions about the offset itself.
fn assert_local_clock_label(label: &str) {
    assert!(
        !label.contains('T'),
        "raw feed timestamp leaked into the frame label: {label}"
    );
    let Some((hours, minutes)) = label.split_once(':').and_then(|(hours, minutes)| {
        Some((hours.parse::<u32>().ok()?, minutes.parse::<u32>().ok()?))
    }) else {
        panic!("expected a HH:MM label, got {label}");
    };
    assert!(hours < 24 && minutes < 60, "not a wall clock: {label}");
}

#[test]
fn ignores_stale_generation_frame_and_refresh_started_resets_state() {
    // Given: one GUI application, deterministic radar generations, and injected timer intervals.
    let stale_gate = Arc::new(Gate::new());
    let radar = Arc::new(FakeRadarApi {
        metadata_calls: AtomicUsize::new(0),
        stale_gate: Arc::clone(&stale_gate),
        ordering: Mutex::new(Vec::new()),
    });
    let cache = std::env::temp_dir().join(format!("allesbuien-radar-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(StubFeedApi),
        radar: radar.clone(),
        notifier: Arc::new(StubNotifier),
        location: LocationConfig {
            latitude: 52.37,
            longitude: 4.84,
            label: String::from("Amsterdam-West"),
        },
        radar_state_dir: cache.clone(),
    });
    let intervals = TimerIntervals {
        radar_refresh: Duration::from_secs(300),
        radar_playback: Duration::from_millis(400),
        ..TimerIntervals::default()
    };

    // When: construction loads generation one, playback advances, and generation two is superseded.
    let mut booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        intervals,
    )
    .boot()
    .expect("fake-wired GUI should boot");
    for _ in 0..20_000 {
        booted.process_events();
        if booted
            .radar_controller()
            .is_some_and(|controller| *controller.frame_count() == 2)
        {
            break;
        }
        std::thread::yield_now();
    }
    let controller = booted.radar_controller().expect("controller should exist");
    assert_eq!(*controller.generation(), 1);
    assert_eq!(*controller.frame_count(), 2);
    assert_eq!(*controller.current_frame_index(), 0);
    assert!(*controller.playing());
    assert_eq!(*controller.refresh_interval_ms(), 300_000);
    assert_eq!(*controller.playback_interval_ms(), 400);
    assert_eq!(controller.timestamps().len(), 2);
    assert_local_clock_label(&controller.current_frame_label().to_string());
    assert_eq!(
        controller.current_frame_label().to_string(),
        LocalTimestamp::from_feed("2026-08-30T09:00:00").time_label,
        "the vendor's declared timeOffset was ignored in favour of a derivation"
    );
    assert_eq!(*controller.first_forecast_index(), 2);
    assert!(
        *controller.current_frame_observed(),
        "a run whose every frame is in the past has nothing to forecast"
    );
    assert_eq!(
        controller.current_frame_url().to_string(),
        "image://radar/1/0"
    );

    booted.tick_radar();
    assert_eq!(
        *booted
            .radar_controller()
            .expect("controller should exist")
            .current_frame_index(),
        1
    );
    booted.seek_radar(9);
    {
        let controller = booted.radar_controller().expect("controller should exist");
        assert_eq!(*controller.current_frame_index(), 1);
        assert!(!*controller.playing());
        assert_eq!(
            controller.current_frame_url().to_string(),
            "image://radar/1/1"
        );
    }
    booted.seek_radar(-3);
    {
        let controller = booted.radar_controller().expect("controller should exist");
        assert_eq!(*controller.current_frame_index(), 0);
        assert_eq!(
            controller.current_frame_url().to_string(),
            "image://radar/1/0"
        );
    }
    booted.refresh_radar();
    let controller = booted.radar_controller().expect("controller should exist");
    assert_eq!(*controller.generation(), 2);
    assert_eq!(*controller.frame_count(), 0);
    assert!(!*controller.playing());
    for _ in 0..20_000 {
        if radar.metadata_calls.load(Ordering::SeqCst) == 2 {
            break;
        }
        std::thread::yield_now();
    }
    booted.refresh_radar();
    for _ in 0..20_000 {
        booted.process_events();
        if booted
            .radar_controller()
            .is_some_and(|controller| *controller.frame_count() == 3)
        {
            break;
        }
        std::thread::yield_now();
    }
    stale_gate.release();
    for _ in 0..20_000 {
        booted.process_events();
        if radar
            .ordering
            .lock()
            .expect("ordering log lock should be healthy")
            .iter()
            .any(|event| event == "frame-fake://stale/0")
        {
            break;
        }
        std::thread::yield_now();
    }

    // Then: the stale completion ran after the fresh one but could not replace generation three.
    let controller = booted.radar_controller().expect("controller should exist");
    assert_eq!(*controller.generation(), 3);
    assert_eq!(*controller.frame_count(), 3);
    assert_eq!(controller.timestamps().len(), 3);
    assert_local_clock_label(&controller.current_frame_label().to_string());
    assert_eq!(
        controller.current_frame_url().to_string(),
        "image://radar/3/0"
    );
    let ordering = radar
        .ordering
        .lock()
        .expect("ordering log lock should be healthy");
    let fresh_position = ordering
        .iter()
        .position(|event| event == "frame-fake://fresh/2")
        .expect("fresh generation should finish");
    let stale_position = ordering
        .iter()
        .position(|event| event == "frame-fake://stale/0")
        .expect("stale generation should finish");
    assert!(fresh_position < stale_position);
    drop(ordering);
    std::fs::remove_dir_all(cache).expect("radar test cache should be removable");
}
