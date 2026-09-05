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
    notify::{Notifier, NotifyConfig, NotifyError},
    weather::{FeedApi, FeedFuture, RainSoon, WeatherData},
};
use allesbuien_testkit::{SETTLE_BUDGET, StubRadarApi, amsterdam_west, pump_until};

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
                rain_line: format!("refresh {call}"),
                rain_soon: rain,
                rain_series: Vec::new(),
            })
        })
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

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this single-process GUI scenario keeps setup, two refreshes, cooldown observation, and notifier assertions in one ordered narrative"
)]
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
            location: amsterdam_west(),
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
    pump_until("the boot refresh lands and notifies", SETTLE_BUDGET, || {
        booted.process_events();
        notifier.calls().len() == 1
            && booted
                .weather_controller()
                .is_some_and(|controller| controller.rain_summary().to_string() == "refresh 1")
    });
    booted.refresh_weather();
    pump_until("the second refresh reaches the view", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .weather_controller()
            .is_some_and(|controller| controller.rain_summary().to_string() == "refresh 2")
    });
    feed.rain.store(false, Ordering::SeqCst);
    booted.refresh_weather();
    pump_until("the dry refresh reaches the view", SETTLE_BUDGET, || {
        booted.process_events();
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
