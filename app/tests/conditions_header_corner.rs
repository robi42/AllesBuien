//! The header's right corner has to disappear, not merely go blank.
//!
//! Both corner captions bind `visible: text.length > 0`, and their parent
//! `ColumnLayout` binds its own visibility to theirs. Asserting those bindings
//! would prove nothing about the thing that actually goes wrong: a `RowLayout`
//! keeps its `spacing` in front of an item it still lays out, so a corner that
//! is blank rather than absent leaves the location name ending a gap early for
//! no stated reason. That is a layout outcome, so this reads it off the
//! rendered tree - the left column has to end exactly where the header row
//! does.

use std::{
    ffi::{CString, c_char},
    path::PathBuf,
    ptr::NonNull,
    sync::Arc,
};

use allesbuien::{AppBuilder, BootedApp, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::weather::{CurrentConditions, FeedApi, FeedFuture, WeatherData};
use allesbuien_testkit::{SETTLE_BUDGET, StubNotifier, StubRadarApi, amsterdam_west, pump_until};
use cxx_qt_lib::{QString, QVariant};

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

    #[link_name = "_ZN7QObject16staticMetaObjectE"]
    static QOBJECT_STATIC_META_OBJECT: QMetaObject;
}

/// A station that reports readings but names neither itself nor a time.
///
/// The feed does not promise either field, and this is the shape that empties
/// both corner captions at once: `station_name` is what `stationLabel` reads,
/// and a default `observed_at` carries no time label for `observedLabel`.
struct NamelessStationFeedApi;

impl FeedApi for NamelessStationFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    temperature: Some(11.5),
                    ..CurrentConditions::default()
                }),
                forecast: Vec::new(),
                rain_line: String::new(),
                rain_soon: None,
                rain_series: Vec::new(),
            })
        })
    }
}

fn boot_over_the_nameless_station() -> BootedApp {
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(NamelessStationFeedApi),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: std::env::temp_dir()
            .join(format!("allesbuien-header-corner-{}", std::process::id())),
    });
    AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        TimerIntervals::default(),
    )
    .boot()
    .expect("fake-wired GUI should boot")
}

#[test]
fn a_feed_naming_no_station_leaves_no_gap_beside_the_location() {
    // Given: a booted window whose feed answers with readings but no station
    // name and no observation time.
    let booted = boot_over_the_nameless_station();

    // When: that answer has reached the controller and the layout has settled.
    pump_until(
        "the nameless station reaches the header",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            booted.weather_controller().is_some_and(|controller| {
                controller.station_name().is_empty()
                    && controller.temperature().to_string() == "11.5 °C"
            })
        },
    );

    // SAFETY: Category 8 - Qt owns the focused QQuickWindow and QObject tree for
    // `booted`'s lifetime; null pointers are rejected and calls stay on the GUI thread.
    unsafe {
        let window = NonNull::new(focus_window()).expect("root QQuickWindow should be focused");
        let child = |name: &str| {
            NonNull::new(find_child(
                window.as_ptr().cast::<QObject>(),
                &QString::from(name),
                &QOBJECT_STATIC_META_OBJECT,
                1,
            ))
            .unwrap_or_else(|| panic!("{name} should exist"))
        };
        let corner = child("headerCorner");
        let header_row = child("headerRow");
        let location_source = child("locationSource");
        let visible = CString::new("visible").expect("static property name is valid");
        let width = CString::new("width").expect("static property name is valid");
        let read = |object: NonNull<QObject>, name: &CString| {
            object_property(object.as_ptr(), name.as_ptr())
        };

        // Then: the corner is gone rather than blank, and the left column runs
        // the full width of the header, so nothing was reserved for it.
        assert_eq!(
            read(corner, &visible).value::<bool>(),
            Some(false),
            "the corner should collapse when it has nothing to say"
        );
        let row_width = read(header_row, &width)
            .value::<f64>()
            .expect("the header row should have a width");
        let left_width = read(location_source, &width)
            .value::<f64>()
            .expect("the location column should have a width");
        assert!(row_width > 0.0, "the header row should have been laid out");
        assert!(
            (row_width - left_width).abs() < f64::EPSILON,
            "the collapsed corner still reserved {} px of the header",
            row_width - left_width
        );
    }
}
