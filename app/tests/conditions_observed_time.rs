//! The corner timestamp says it is a measurement time, not a clock.
//!
//! `SCHIPHOL` over a bare `12:30` names an instrument and a number and leaves
//! the relation between them to be inferred; `@ 12:30` states it. The `@` is
//! added in `ConditionsView.qml` rather than in `conditions_view.rs`, because
//! it is a typographic relation glyph on a caption row this file's sibling
//! already reads off the rendered tree - the controller keeps carrying the
//! plain `hh:mm` a notification or a test can assert without a window.
//!
//! This is a second file rather than an extra assertion inside
//! `conditions_header_corner.rs` because the two scenarios contradict each
//! other in one process. That harness proves the corner COLLAPSES when the feed
//! names neither station nor time, which needs both captions empty; this one
//! needs both populated. `AGENTS.md` pins every file under `app/tests/` to one
//! GUI test and one `QGuiApplication`, so the populated case has to boot its own
//! window rather than share that one.

use std::{
    ffi::{CString, c_char},
    path::PathBuf,
    ptr::NonNull,
    sync::Arc,
};

use allesbuien::{AppBuilder, BootedApp, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::{
    datetime::LocalTimestamp,
    weather::{CurrentConditions, FeedApi, FeedFuture, WeatherData},
};
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

unsafe fn child(window: NonNull<QQuickWindow>, name: &str) -> NonNull<QObject> {
    // SAFETY: Category 8 - Qt owns the focused window's QObject tree for the
    // booted app's lifetime, and this call stays on the GUI thread.
    let found = unsafe {
        find_child(
            window.as_ptr().cast::<QObject>(),
            &QString::from(name),
            &QOBJECT_STATIC_META_OBJECT,
            1,
        )
    };
    NonNull::new(found).unwrap_or_else(|| panic!("{name} should exist"))
}

unsafe fn text_of(object: NonNull<QObject>) -> String {
    let name = CString::new("text").expect("static property name is valid");
    // SAFETY: Category 8 - `object` came from the tree Qt owns, and the name is
    // a NUL-terminated static.
    unsafe { object_property(object.as_ptr(), name.as_ptr()) }
        .value::<QString>()
        .expect("a Label should carry text")
        .to_string()
}

/// A station that names itself and the minute it read.
///
/// `17:50` is the same fixture `conditions_view.rs` formats against, so the
/// controller's plain string and the corner's decorated one can be compared to
/// one another rather than each to a number of its own.
struct TimestampedStationFeedApi;

impl FeedApi for TimestampedStationFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    station_name: String::from("Schiphol"),
                    temperature: Some(11.5),
                    observed_at: LocalTimestamp::from_feed_in(
                        "2026-08-31T17:50:00",
                        "Europe/Amsterdam",
                    ),
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

fn boot_over_the_timestamped_station() -> BootedApp {
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(TimestampedStationFeedApi),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: std::env::temp_dir()
            .join(format!("allesbuien-observed-time-{}", std::process::id())),
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
fn the_corner_time_is_rendered_as_a_measurement_rather_than_a_clock() {
    // Given: a booted window whose feed names its station and the minute it read.
    let booted = boot_over_the_timestamped_station();

    // When: that answer has reached the controller and the layout has settled.
    pump_until(
        "the timestamped station reaches the header and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let header_is_ready = booted
                .weather_controller()
                .is_some_and(|controller| controller.observed_label().to_string() == "17:50");
            // SAFETY: Category 8 - Qt owns the focus-window pointer and this call
            // only tests it for null on the Qt GUI thread.
            header_is_ready && unsafe { !focus_window().is_null() }
        },
    );

    // SAFETY: Category 8 - Qt owns the focused QQuickWindow and the QObject tree
    // under it for `booted`'s lifetime; null pointers are rejected and every
    // call stays on the GUI thread.
    unsafe {
        let window = NonNull::new(focus_window()).expect("root QQuickWindow should be focused");

        // Then: the time carries the relation, and the station beside it does
        // not. Reading both is what proves the corner was reached and that the
        // glyph belongs to the timestamp rather than to every caption in it.
        assert_eq!(text_of(child(window, "stationLabel")), "SCHIPHOL");
        assert_eq!(text_of(child(window, "observedLabel")), "@ 17:50");
    }
}
