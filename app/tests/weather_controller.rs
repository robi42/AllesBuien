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
    config::LocationConfig,
    datetime::{LocalTimestamp, SOURCE_TIMEZONE},
    notify::{Notifier, NotifyError},
    radar::{RadarApi, RadarError, RadarFuture},
    rain::parse_series,
    weather::{CurrentConditions, FeedApi, FeedFuture, ForecastDay, RainSoon, WeatherData},
};
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
                    now_line: String::from("Now: unavailable"),
                    rain_line: String::from("next 2h: dry"),
                    rain_soon: None,
                    rain_series: Vec::new(),
                });
            }
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    station_name: String::from("Amsterdam-West"),
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
                now_line: String::from("Now: fixture"),
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
        Box::pin(async { unreachable!("weather test must not fetch radar frames") })
    }
}

struct StubNotifier;

impl Notifier for StubNotifier {
    fn send(&self, _title: &str, _body: &str) -> Result<(), NotifyError> {
        Ok(())
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
        location: LocationConfig {
            latitude: 52.37,
            longitude: 4.84,
            label: String::from("Amsterdam-West"),
        },
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
    for _ in 0..10_000 {
        booted.process_events();
        if booted
            .weather_controller()
            .expect("controller should exist")
            .row_count(&QModelIndex::default())
            == 5
        {
            break;
        }
        std::thread::yield_now();
    }

    // Then: the initial fetch ran once and populated conditions, rain, and five rows.
    {
        let controller = booted
            .weather_controller()
            .expect("controller should exist");
        assert_eq!(feed.calls.load(Ordering::SeqCst), 1);
        assert_eq!(controller.station_name().to_string(), "Amsterdam-West");
        assert_eq!(controller.temperature().to_string(), "19.5 °C");
        assert_eq!(
            controller.condition().to_string(),
            "Heavily clouded with rain"
        );
        assert!(
            controller
                .condition_icon_url()
                .to_string()
                .ends_with("/96x96/QQ.png"),
            "unexpected icon url {}",
            controller.condition_icon_url()
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
        assert!(icon.to_string().ends_with("/96x96/F.png"), "got {icon}");
    }

    feed.empty.store(true, Ordering::SeqCst);
    booted.refresh_weather();
    for _ in 0..10_000 {
        booted.process_events();
        if booted
            .weather_controller()
            .expect("controller should exist")
            .status()
            .to_string()
            == "Current conditions unavailable"
        {
            break;
        }
        std::thread::yield_now();
    }
    let controller = booted
        .weather_controller()
        .expect("controller should exist");
    assert_eq!(feed.calls.load(Ordering::SeqCst), 2);
    assert_eq!(controller.station_name().to_string(), "Unavailable");
    assert_eq!(controller.temperature().to_string(), "Unavailable");
    assert_eq!(controller.row_count(&QModelIndex::default()), 0);
    assert!(controller.condition_icon_url().is_empty());
    assert!(controller.observed_label().is_empty());
    assert_eq!(controller.rain_levels().len(), 0);
    assert_eq!(controller.rain_axis_labels().len(), 0);
    assert!(!*controller.rain_wet());
}
