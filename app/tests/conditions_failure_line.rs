//! One failed fetch says so once, and the reading it could not fill does not
//! pretend to be a number that was cut short.
//!
//! Neither defect was reachable from the controller. The duplicate status was
//! two labels binding one `status` property, so every property-level assertion
//! passed while the window printed the sentence twice about 190 px apart; the
//! truncation was `Text.ElideRight` deciding at paint time, which no binding
//! reports. Both are outcomes of the rendered tree, so this reads them there:
//! the forecast list must contain no second copy of the sentence, and the hero
//! must ask for no more width than it was given.

use std::{
    ffi::{CString, c_char},
    path::PathBuf,
    ptr::NonNull,
    sync::Arc,
};

use allesbuien::{AppBuilder, BootedApp, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::weather::{FeedApi, FeedFuture, WeatherError};
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

unsafe fn find(window: NonNull<QQuickWindow>, name: &str) -> *mut QObject {
    // SAFETY: Category 8 - Qt owns the focused window's QObject tree for the
    // booted app's lifetime, and this call stays on the GUI thread.
    unsafe {
        find_child(
            window.as_ptr().cast::<QObject>(),
            &QString::from(name),
            &QOBJECT_STATIC_META_OBJECT,
            1,
        )
    }
}

unsafe fn child(window: NonNull<QQuickWindow>, name: &str) -> NonNull<QObject> {
    // SAFETY: Category 8 - as `find`, whose result is checked before use.
    NonNull::new(unsafe { find(window, name) }).unwrap_or_else(|| panic!("{name} should exist"))
}

unsafe fn property(object: NonNull<QObject>, name: &str) -> QVariant {
    let name = CString::new(name).expect("static property name is valid");
    // SAFETY: Category 8 - `object` came from the tree Qt owns, and the name is
    // a NUL-terminated static.
    unsafe { object_property(object.as_ptr(), name.as_ptr()) }
}

unsafe fn text_of(object: NonNull<QObject>) -> String {
    // SAFETY: Category 8 - as `property`.
    unsafe { property(object, "text") }
        .value::<QString>()
        .expect("a Label should carry text")
        .to_string()
}

unsafe fn number_of(object: NonNull<QObject>, name: &str) -> f64 {
    // SAFETY: Category 8 - as `property`.
    unsafe { property(object, name) }
        .value::<f64>()
        .unwrap_or_else(|| panic!("{name} should be readable"))
}

/// A feed that never answers, which is the state the window opens in offline.
struct UnreachableFeedApi;

impl FeedApi for UnreachableFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            let error = reqwest::Client::new()
                .get("not a URL")
                .send()
                .await
                .expect_err("the fake request URL must be invalid");
            Err(WeatherError::FeedRequest(error))
        })
    }
}

fn boot_over_the_unreachable_feed() -> BootedApp {
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(UnreachableFeedApi),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: std::env::temp_dir()
            .join(format!("allesbuien-failure-line-{}", std::process::id())),
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
fn a_failed_fetch_states_itself_once_and_leaves_the_reading_whole() {
    // Given: a booted window whose only feed request fails.
    let booted = boot_over_the_unreachable_feed();

    // When: that failure has reached the controller and the layout has settled.
    pump_until("the cold failure reaches the panel", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .weather_controller()
            .is_some_and(|controller| controller.status().to_string().contains(" — "))
    });
    let status = booted
        .weather_controller()
        .expect("controller should exist")
        .status()
        .to_string();

    // SAFETY: Category 8 - Qt owns the focused QQuickWindow and the QObject
    // tree under it for `booted`'s lifetime, and every call stays on the GUI
    // thread.
    unsafe {
        let window = NonNull::new(focus_window()).expect("root QQuickWindow should be focused");

        // Then: the conditions panel states the failure, and it is the only
        // thing on screen that does. The forecast list is itself reachable from
        // this same search, which is what makes the missing second label an
        // absence rather than a search that never got there.
        let conditions_error = child(window, "conditionsError");
        assert_eq!(
            property(conditions_error, "visible").value::<bool>(),
            Some(true)
        );
        assert_eq!(text_of(conditions_error), status);
        child(window, "forecastView");
        assert!(
            find(window, "forecastError").is_null(),
            "the forecast list printed a second copy of the same sentence"
        );

        // Then: the hero carries the mark rather than a word it has to cut, and
        // asks for no more width than the column gave it, so nothing elides.
        let hero = child(window, "heroReading");
        assert_eq!(text_of(hero), "—");
        let width = number_of(hero, "width");
        let implicit = number_of(hero, "implicitWidth");
        assert!(width > 0.0, "the hero should have been laid out");
        assert!(
            implicit <= width,
            "the hero wanted {implicit} px inside {width} px, so it elided"
        );
    }
}
