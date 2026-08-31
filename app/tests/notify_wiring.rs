use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::{
    config::LocationConfig,
    notify::{Notifier, NotifyConfig, NotifyError},
    radar::{RadarApi, RadarError, RadarFuture},
    weather::{FeedApi, FeedFuture, RainSoon, WeatherData},
};

struct FakeFeedApi {
    calls: AtomicUsize,
    rain: AtomicBool,
}

impl FeedApi for FakeFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let rain = self.rain.load(Ordering::SeqCst).then(|| RainSoon {
            minutes_until: 10,
            time_label: String::from("14:10"),
            max_mmh: 1.5,
        });
        Box::pin(async move {
            Ok(WeatherData {
                current: None,
                forecast: Vec::new(),
                now_line: String::from("Now: unavailable"),
                rain_line: format!("refresh {call}"),
                rain_soon: rain,
                rain_series: Vec::new(),
            })
        })
    }
}

struct StubRadarApi;

impl RadarApi for StubRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async {
            Err(RadarError::InvalidRunTimestamp {
                timestamp: String::new(),
            })
        })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async { unreachable!("notification test must not fetch radar frames") })
    }
}

#[derive(Default)]
struct FakeNotifier {
    calls: Mutex<Vec<(String, String)>>,
}

impl FakeNotifier {
    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().expect("fake calls lock").clone()
    }
}

impl Notifier for FakeNotifier {
    fn send(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        self.calls
            .lock()
            .expect("fake calls lock")
            .push((title.to_owned(), body.to_owned()));
        Ok(())
    }
}

fn wait_for(booted: &allesbuien::BootedApp, predicate: impl Fn() -> bool) {
    for _ in 0..10_000 {
        booted.process_events();
        if predicate() {
            return;
        }
        std::thread::yield_now();
    }
    panic!("timed out waiting for background refresh");
}

#[test]
fn dispatches_once_when_same_rain_event_refreshes_within_cooldown() {
    // Given: one GUI application with rain inside the configured threshold and fake ports.
    let feed = Arc::new(FakeFeedApi {
        calls: AtomicUsize::new(0),
        rain: AtomicBool::new(true),
    });
    let notifier = Arc::new(FakeNotifier::default());
    let services = CoreServices::new_with_notify(
        CoreServiceConfig {
            feed: feed.clone(),
            radar: Arc::new(StubRadarApi),
            notifier: notifier.clone(),
            location: LocationConfig {
                latitude: 52.37,
                longitude: 4.84,
                label: String::from("Amsterdam-West"),
            },
            radar_state_dir: PathBuf::from("/nonexistent/test-radar"),
        },
        NotifyConfig {
            enabled: true,
            threshold_minutes: 30,
            cooldown: Duration::from_secs(3_600),
        },
    );
    let mut booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        TimerIntervals::default(),
    )
    .boot()
    .expect("fake-wired GUI should boot");

    // When: the initial refresh succeeds, then the same event refreshes again.
    wait_for(&booted, || {
        notifier.calls().len() == 1
            && booted
                .weather_controller()
                .is_some_and(|controller| controller.rain_summary().to_string() == "refresh 1")
    });
    booted.refresh_weather();
    wait_for(&booted, || {
        booted
            .weather_controller()
            .is_some_and(|controller| controller.rain_summary().to_string() == "refresh 2")
    });
    feed.rain.store(false, Ordering::SeqCst);
    booted.refresh_weather();
    wait_for(&booted, || {
        booted
            .weather_controller()
            .is_some_and(|controller| controller.rain_summary().to_string() == "refresh 3")
    });

    // Then: cooldown dedup suppresses the repeated event and dry data adds no dispatch.
    assert_eq!(
        notifier.calls(),
        [(
            String::from("Rain soon (Amsterdam-West)"),
            String::from("Starts around 14:10 (10 min), peak ~1.50 mm/h")
        )]
    );
}
