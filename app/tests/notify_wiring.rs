use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use allesbuien_core::{
    notify::{Notifier, NotifyConfig, NotifyError},
    weather::{FeedApi, FeedFuture, WeatherData},
};
use allesbuien_harness::HarnessBuilder;
use allesbuien_testkit::{SETTLE_BUDGET, fixtures::rain_soon, pump_until};

/// A feed that numbers its answers and reports rain soon until switched dry.
struct RainToggleFeedApi {
    calls: AtomicUsize,
    rain: AtomicBool,
}

impl FeedApi for RainToggleFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let rain = self.rain.load(Ordering::SeqCst).then(rain_soon);
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
fn dispatches_once_when_same_rain_event_refreshes_within_cooldown() {
    // Given: one GUI application with rain inside the configured threshold and fake ports.
    let feed = Arc::new(RainToggleFeedApi {
        calls: AtomicUsize::new(0),
        rain: AtomicBool::new(true),
    });
    let notifier = Arc::new(FakeNotifier::default());
    let mut booted = HarnessBuilder::with_feed(feed.clone())
        .notifications(
            notifier.clone(),
            NotifyConfig {
                enabled: true,
                threshold_minutes: 30,
                cooldown: Duration::from_secs(3_600),
            },
        )
        .boot();

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
