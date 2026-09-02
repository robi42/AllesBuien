use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use allesbuien::{
    AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals,
    weather_controller::ffi::ForecastRole,
};
use allesbuien_core::{
    datetime::{LocalTimestamp, SOURCE_TIMEZONE},
    rain::parse_series,
    weather::{CurrentConditions, FeedApi, FeedFuture, ForecastDay, RainSoon, WeatherData},
};
use allesbuien_testkit::{SETTLE_BUDGET, StubNotifier, StubRadarApi, amsterdam_west, pump_until};
use cxx_qt_lib::{QByteArray, QModelIndex, QString};

struct FakeFeedApi {
    calls: AtomicUsize,
    empty: AtomicBool,
}

impl FeedApi for FakeFeedApi {
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
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    // Deliberately unequal to the configured location label, so
                    // the header assertions below fail if the two are swapped.
                    station_name: String::from("Meetstation Schiphol"),
                    temperature: Some(19.5),
                    feel_temperature: Some(18.8),
                    wind_speed: Some(3.2),
                    wind_bft: Some(2),
                    humidity: Some(71.0),
                    condition: Some(String::from("Half bewolkt")),
                    condition_code: String::from("qq"),
                    observed_at: LocalTimestamp::from_feed_in(
                        "2026-08-31T17:50:00",
                        SOURCE_TIMEZONE,
                    ),
                    ..CurrentConditions::default()
                }),
                forecast: (1..=5)
                    .map(|day| ForecastDay {
                        day: format!("2026-09-0{day}T00:00:00"),
                        min_temperature_min: 10 + day,
                        min_temperature_max: 11 + day,
                        max_temperature_min: 18 + day,
                        max_temperature_max: 19 + day,
                        rain_chance: 20,
                        condition: String::from("Bewolkt"),
                        condition_code: String::from("f"),
                        day_at: LocalTimestamp::from_feed_in(
                            &format!("2026-09-0{day}T12:00:00"),
                            SOURCE_TIMEZONE,
                        ),
                        ..ForecastDay::default()
                    })
                    .collect(),
                rain_line: String::from("starts around 14:10"),
                rain_soon: Some(RainSoon {
                    minutes_until: 10,
                    time_label: String::from("14:10"),
                    max_mmh: 1.5,
                }),
                rain_series: parse_series("000|14:00\n000|14:05\n077|14:10\n090|14:15\n", 0).points,
            })
        })
    }
}

#[test]
fn populates_weather_properties_when_boot_triggers_initial_refresh() {
    // Given: one GUI application builder with a fake feed and a long recurring interval.
    let feed = Arc::new(FakeFeedApi {
        calls: AtomicUsize::new(0),
        empty: AtomicBool::new(false),
    });
    let services = CoreServices::new(CoreServiceConfig {
        feed: feed.clone(),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: PathBuf::from("/nonexistent/test-radar"),
    });
    let intervals = TimerIntervals {
        weather_refresh: Duration::from_secs(60),
        ..TimerIntervals::default()
    };

    // When: boot constructs the controller and its initial async refresh completes.
    let mut booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        intervals,
    )
    .boot()
    .expect("fake-wired GUI should boot");
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
        assert_eq!(
            controller.station_name().to_string(),
            "Meetstation Schiphol"
        );
        assert_eq!(controller.temperature().to_string(), "19.5 °C");
        assert_eq!(controller.feels_like().to_string(), "18.8 °C");
        assert_eq!(controller.wind().to_string(), "3.2 m/s");
        assert_eq!(controller.humidity().to_string(), "71%");
        assert_eq!(
            controller.condition().to_string(),
            "Heavily clouded with rain"
        );
        assert_eq!(
            controller.condition_icon_url().to_string(),
            "image://condition/QQ"
        );
        assert_eq!(controller.observed_label().to_string(), "Observed 17:50");
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
        assert_eq!(icon.to_string(), "image://condition/F");
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
    assert_eq!(controller.temperature().to_string(), "Unavailable");
    assert!(controller.feels_like().is_empty());
    assert!(controller.humidity().is_empty());
    assert_eq!(controller.row_count(&QModelIndex::default()), 0);
    assert!(controller.condition_icon_url().is_empty());
    assert!(controller.observed_label().is_empty());
    assert_eq!(controller.rain_levels().len(), 0);
    assert_eq!(controller.rain_axis_labels().len(), 0);
    assert!(!*controller.rain_wet());
}
