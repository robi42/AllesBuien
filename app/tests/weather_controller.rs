use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use allesbuien::{TimerIntervals, weather_controller::ffi::ForecastRole};
use allesbuien_core::weather::{FeedApi, FeedFuture, WeatherData};
use allesbuien_harness::HarnessBuilder;
use allesbuien_testkit::{SETTLE_BUDGET, fixtures::five_day_weather, pump_until};
use cxx_qt_lib::{QByteArray, QModelIndex, QString};

/// A feed that answers with the shared five-day fixture, or with nothing at all
/// once switched empty, and counts how often it was asked.
struct FiveDayOrEmptyFeedApi {
    calls: AtomicUsize,
    empty: AtomicBool,
}

impl FeedApi for FiveDayOrEmptyFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let empty = self.empty.load(Ordering::SeqCst);
        Box::pin(async move {
            if empty {
                return Ok(WeatherData {
                    current: None,
                    forecast: Vec::new(),
                    rain_line: String::from("next 2h: dry"),
                    rain_soon: None,
                    rain_series: Vec::new(),
                });
            }
            Ok(five_day_weather())
        })
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this one-process GUI regression keeps bootstrap, async settling, property checks, model roles, and refresh replacement in one ordered narrative"
)]
#[expect(
    clippy::float_cmp,
    reason = "the encoded dry rain sample must map to the exact zero-height baseline asserted by this regression test"
)]
fn populates_weather_properties_when_boot_triggers_initial_refresh() {
    // Given: one GUI application builder with a fake feed and a long recurring interval.
    let feed = Arc::new(FiveDayOrEmptyFeedApi {
        calls: AtomicUsize::new(0),
        empty: AtomicBool::new(false),
    });
    let intervals = TimerIntervals {
        weather_refresh: Duration::from_secs(60),
        ..TimerIntervals::default()
    };

    // When: boot constructs the controller and its initial async refresh completes.
    let mut booted = HarnessBuilder::with_feed(feed.clone())
        .intervals(intervals)
        .boot();
    pump_until("the boot refresh fills five rows", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .weather_controller()
            .expect("controller should exist")
            .row_count(&QModelIndex::default())
            == 5
    });

    // Then: the initial fetch ran once and populated conditions, rain, and five rows.
    {
        let controller = booted
            .weather_controller()
            .expect("controller should exist");
        assert_eq!(feed.calls.load(Ordering::SeqCst), 1);
        assert_eq!(controller.location_name().to_string(), "Amsterdam-West");
        assert_eq!(controller.station_name().to_string(), "Schiphol");
        assert_eq!(controller.temperature().to_string(), "19.5 °C");
        assert_eq!(controller.feels_like().to_string(), "18.8 °C");
        assert_eq!(controller.wind().to_string(), "3.2 m/s");
        assert_eq!(controller.humidity().to_string(), "71%");
        assert_eq!(
            controller.condition().to_string(),
            "Heavily clouded with rain"
        );
        // The harness's default icon cache is disabled, so a provider URL it can
        // never serve is withheld; `weather_icons.rs` proves icon delivery.
        assert!(controller.condition_icon_url().is_empty());
        assert_eq!(controller.observed_label().to_string(), "17:50");
        assert_eq!(controller.rain_summary().to_string(), "starts around 14:10");
        assert_eq!(controller.row_count(&QModelIndex::default()), 5);
        assert_eq!(*controller.refresh_interval_ms(), 60_000);
        assert!(controller.status().is_empty());

        let levels: Vec<f64> = controller.rain_levels().iter().copied().collect();
        assert_eq!(levels.len(), 4);
        assert_eq!(levels[0], 0.0);
        assert!(levels[2] > 0.0, "wet sample flattened to {}", levels[2]);
        assert!(levels[3] > levels[2], "bars ignore intensity: {levels:?}");
        assert!(*controller.rain_wet());
        let captions: Vec<String> = controller
            .rain_axis_labels()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(captions, ["14:00", "", "", "14:15"]);

        let roles = controller.role_names();
        assert_eq!(
            roles.get(&ForecastRole::Weekday.repr),
            Some(QByteArray::from("weekday"))
        );
        assert_eq!(
            roles.get(&ForecastRole::IconUrl.repr),
            Some(QByteArray::from("iconUrl"))
        );
        let first_row = controller.index(0, 0, &QModelIndex::default());
        let weekday = controller
            .data(&first_row, ForecastRole::Weekday.repr)
            .value::<QString>()
            .expect("weekday role should carry a string");
        assert_eq!(weekday.to_string(), "Tue");
        let condition = controller
            .data(&first_row, ForecastRole::Condition.repr)
            .value::<QString>()
            .expect("condition role should carry a string");
        assert_eq!(condition.to_string(), "Partly cloudy with light rain");
        let icon = controller
            .data(&first_row, ForecastRole::IconUrl.repr)
            .value::<QString>()
            .expect("icon role should carry a string");
        assert!(icon.is_empty());
    }

    // The rain cursor tracks the radar frame QML is showing, and reports itself
    // absent for frames the four-sample fixture window does not cover.
    booted.track_radar_frame("14:05");
    {
        let controller = booted
            .weather_controller()
            .expect("controller should exist");
        assert!(*controller.rain_cursor_visible());
        assert!(
            (*controller.rain_cursor_fraction() - 1.5 / 4.0).abs() < 1e-9,
            "cursor at {}",
            *controller.rain_cursor_fraction()
        );
    }
    booted.track_radar_frame("13:55");
    assert!(
        !*booted
            .weather_controller()
            .expect("controller should exist")
            .rain_cursor_visible(),
        "an observed frame before the series must hide the cursor"
    );

    feed.empty.store(true, Ordering::SeqCst);
    booted.refresh_weather();
    pump_until("the empty refresh reports itself", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .weather_controller()
            .expect("controller should exist")
            .status()
            .to_string()
            == "Current conditions unavailable"
    });
    let controller = booted
        .weather_controller()
        .expect("controller should exist");
    assert_eq!(feed.calls.load(Ordering::SeqCst), 2);
    assert_eq!(controller.station_name().to_string(), "Unavailable");
    assert_eq!(controller.temperature().to_string(), "—");
    assert!(controller.feels_like().is_empty());
    assert!(controller.humidity().is_empty());
    assert_eq!(controller.row_count(&QModelIndex::default()), 0);
    assert!(controller.condition_icon_url().is_empty());
    assert!(controller.observed_label().is_empty());
    assert_eq!(controller.rain_levels().len(), 0);
    assert_eq!(controller.rain_axis_labels().len(), 0);
    assert!(!*controller.rain_wet());
}
