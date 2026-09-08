//! Weather failure wording follows the data that remains visible.
//!
//! A failed refresh retains the previous forecast rows, so only the controller
//! pathway can prove whether the status describes those rows as stale or calls
//! them unavailable. This harness exercises both the cold and warm failure
//! states through one real Qt object.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::weather::{FeedApi, FeedFuture, ForecastDay, WeatherData, WeatherError};
use allesbuien_testkit::{SETTLE_BUDGET, StubNotifier, StubRadarApi, amsterdam_west, pump_until};
use cxx_qt_lib::QModelIndex;

struct SwitchableFeedApi {
    failing: AtomicBool,
}

impl FeedApi for SwitchableFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        let failing = self.failing.load(Ordering::SeqCst);
        Box::pin(async move {
            if failing {
                let error = reqwest::Client::new()
                    .get("not a URL")
                    .send()
                    .await
                    .expect_err("the fake request URL must be invalid");
                return Err(WeatherError::FeedRequest(error));
            }
            Ok(WeatherData {
                current: None,
                forecast: vec![ForecastDay::default()],
                rain_line: String::new(),
                rain_soon: None,
                rain_series: Vec::new(),
            })
        })
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this one-process GUI regression observes cold failure, success, and warm failure in order on the same controller"
)]
fn weather_failure_wording_tracks_whether_a_forecast_is_visible() {
    // Given: a GUI whose first feed request fails before any forecast exists.
    let feed = Arc::new(SwitchableFeedApi {
        failing: AtomicBool::new(true),
    });
    let services = CoreServices::new(CoreServiceConfig {
        feed: feed.clone(),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: std::env::temp_dir().join(format!(
            "allesbuien-weather-staleness-{}",
            std::process::id()
        )),
    });
    let mut booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        TimerIntervals {
            weather_refresh: Duration::from_secs(60),
            ..TimerIntervals::default()
        },
    )
    .boot()
    .expect("fake-wired GUI should boot");

    // When: the cold failure reaches the controller.
    pump_until("the cold failure reports its cause", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .weather_controller()
            .is_some_and(|controller| controller.status().to_string().contains(" — "))
    });

    // Then: with no prior success, the weather is genuinely unavailable.
    let cold_status = booted
        .weather_controller()
        .expect("controller should exist")
        .status()
        .to_string();
    assert!(
        cold_status.starts_with("Weather unavailable"),
        "{cold_status}"
    );

    // When: a successful refresh populates a row, then the next refresh fails.
    feed.failing.store(false, Ordering::SeqCst);
    booted.refresh_weather();
    pump_until("the successful refresh fills a row", SETTLE_BUDGET, || {
        booted.process_events();
        booted.weather_controller().is_some_and(|controller| {
            controller.row_count(&QModelIndex::default()) == 1
                && controller.status().to_string() == "Current conditions unavailable"
        })
    });
    feed.failing.store(true, Ordering::SeqCst);
    booted.refresh_weather();
    pump_until("the warm failure reports its cause", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .weather_controller()
            .is_some_and(|controller| controller.status().to_string().starts_with("Weather "))
    });

    // Then: the retained row is real but may be old, so it is not unavailable.
    let controller = booted
        .weather_controller()
        .expect("controller should exist");
    let warm_status = controller.status().to_string();
    assert!(
        warm_status.starts_with("Weather may be out of date"),
        "{warm_status}"
    );
    assert_eq!(controller.row_count(&QModelIndex::default()), 1);
}
