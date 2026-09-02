use std::{
    ffi::{CString, c_char},
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::Arc,
};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::{
    location::LocationConfig,
    notify::{Notifier, NotifyError},
    radar::{RadarApi, RadarError, RadarFuture},
    weather::{CurrentConditions, FeedApi, FeedFuture, ForecastDay, RainSoon, WeatherData},
};
use cxx_qt_lib::{QImage, QString, QVariant};

#[repr(C)]
struct QObject {
    _private: [u8; 0],
}

#[repr(C)]
struct QMetaObject {
    _private: [u8; 0],
}

#[repr(C)]
struct QQuickWindow {
    _private: [u8; 0],
}

unsafe extern "C" {
    #[link_name = "_ZN15QGuiApplication11focusWindowEv"]
    fn focus_window() -> *mut QQuickWindow;
    #[link_name = "_Z20qt_qFindChild_helperPK7QObjectRK7QStringRK11QMetaObject6QFlagsIN2Qt15FindChildOptionEE"]
    fn find_child(
        parent: *const QObject,
        name: &QString,
        meta_object: &QMetaObject,
        options: i32,
    ) -> *mut QObject;
    #[link_name = "_ZNK7QObject8propertyEPKc"]
    fn object_property(object: *const QObject, name: *const c_char) -> QVariant;
    #[link_name = "_ZN12QQuickWindow10grabWindowEv"]
    fn grab_window(window: *mut QQuickWindow) -> QImage;
    #[link_name = "_ZNK6QImage4saveERK7QStringPKci"]
    fn save_image(image: &QImage, path: &QString, format: *const c_char, quality: i32) -> bool;

    #[link_name = "_ZN7QObject16staticMetaObjectE"]
    static QOBJECT_STATIC_META_OBJECT: QMetaObject;
}

struct FakeFeedApi;

impl FeedApi for FakeFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    station_name: String::from("Amsterdam-West"),
                    temperature: Some(19.5),
                    feel_temperature: Some(18.8),
                    wind_speed: Some(3.2),
                    wind_bft: Some(2),
                    humidity: Some(71.0),
                    condition: Some(String::from("Half bewolkt")),
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
                        ..ForecastDay::default()
                    })
                    .collect(),
                rain_line: String::from("starts around 14:10"),
                rain_soon: Some(RainSoon {
                    minutes_until: 10,
                    time_label: String::from("14:10"),
                    max_mmh: 1.5,
                }),
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
        Box::pin(async { unreachable!("forecast view test must not fetch radar frames") })
    }
}

struct StubNotifier;

impl Notifier for StubNotifier {
    fn send(&self, _title: &str, _body: &str) -> Result<(), NotifyError> {
        Ok(())
    }
}

#[test]
fn shows_five_forecast_rows_and_visible_attribution_when_feed_is_available() {
    // Given: the real QML window backed by the donor's five-day fake feed.
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(FakeFeedApi),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: LocationConfig {
            latitude: 52.37,
            longitude: 4.84,
            label: String::from("Amsterdam-West"),
        },
        radar_state_dir: PathBuf::from("/nonexistent/test-radar"),
    });

    // When: boot and Qt event processing populate and render the forecast view.
    let booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        TimerIntervals::default(),
    )
    .boot()
    .expect("fake-wired GUI should boot");
    for _ in 0..10_000 {
        booted.process_events();
        if booted
            .weather_controller()
            .expect("controller should exist")
            .row_count(&cxx_qt_lib::QModelIndex::default())
            == 5
        {
            break;
        }
        std::thread::yield_now();
    }
    booted.process_events();

    // SAFETY: Category 8 — Qt owns the focused QQuickWindow and its QObject tree for
    // `booted`'s lifetime. Null pointers are rejected before use, ABI types come from
    // cxx-qt-lib, and all calls stay on the Qt GUI thread.
    unsafe {
        let window = NonNull::new(focus_window()).expect("root QQuickWindow should be focused");
        let root = window.as_ptr().cast::<QObject>();
        let forecast = NonNull::new(find_child(
            root,
            &QString::from("forecastView"),
            &QOBJECT_STATIC_META_OBJECT,
            1,
        ))
        .expect("forecastView should exist");
        let attribution = NonNull::new(find_child(
            root,
            &QString::from("attribution"),
            &QOBJECT_STATIC_META_OBJECT,
            1,
        ))
        .expect("attribution should exist");

        // Then: the rendered object tree exposes five rows, and the attribution
        // carries what Buienradar's free-data terms actually require — the
        // visible domain, hyperlinked to the root domain. Reading both off the
        // rendered Label is what makes this a licence check rather than a pin
        // on whatever sentence the UI happens to word it with today.
        let count_name = CString::new("count").expect("static property name is valid");
        let visible_name = CString::new("visible").expect("static property name is valid");
        let text_name = CString::new("text").expect("static property name is valid");
        let link_name = CString::new("link").expect("static property name is valid");
        assert_eq!(
            object_property(forecast.as_ptr(), count_name.as_ptr()).value::<i32>(),
            Some(5)
        );
        assert_eq!(
            object_property(attribution.as_ptr(), visible_name.as_ptr()).value::<bool>(),
            Some(true)
        );
        assert_eq!(
            object_property(attribution.as_ptr(), text_name.as_ptr())
                .value::<QString>()
                .map(|text| text.to_string())
                .as_deref(),
            Some("Buienradar.nl")
        );
        assert_eq!(
            object_property(attribution.as_ptr(), link_name.as_ptr())
                .value::<QString>()
                .map(|link| link.to_string())
                .as_deref(),
            Some("https://www.buienradar.nl")
        );

        let image = grab_window(window.as_ptr());
        assert!(
            !image.is_null(),
            "offscreen window grab should contain pixels"
        );
        let evidence_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.omo/evidence/task-18-omarchy-weather-app.png");
        if evidence_path.parent().is_some_and(Path::exists) {
            let evidence_path = QString::from(
                evidence_path
                    .to_str()
                    .expect("evidence path should be valid UTF-8"),
            );
            let png = CString::new("PNG").expect("static image format is valid");
            assert!(
                save_image(&image, &evidence_path, png.as_ptr(), -1),
                "offscreen window grab should save as PNG"
            );
        }
    }
}
