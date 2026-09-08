use std::{
    ffi::{CString, c_char},
    path::PathBuf,
    ptr::NonNull,
    sync::Arc,
    time::{Duration, Instant},
};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::radar::{RadarApi, RadarFuture};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, StubNotifier, amsterdam_west, pump_until};
use cxx_qt_lib::{QImage, QString, QUrl, QVariant};

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

    #[link_name = "_ZN7QObject16staticMetaObjectE"]
    static QOBJECT_STATIC_META_OBJECT: QMetaObject;
}

struct AnimatedRadarApi;

impl RadarApi for AnimatedRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async {
            Ok(br#"{"timestamp":"202608302300","times":[{"timestamp":"23:00","url":"fake://frame/0"},{"timestamp":"23:05","url":"fake://frame/1"}]}"#.to_vec())
        })
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async move {
            if url.ends_with("/0") {
                Ok(include_bytes!("../../assets/screenshot.png").to_vec())
            } else {
                Ok(allesbuien::BASEMAP_PNG.to_vec())
            }
        })
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this one-process GUI regression keeps provider loading, timer advancement, FFI object access, and rendered-frame assertions in one ordered narrative"
)]
fn renders_and_advances_provider_frames_when_radar_is_playing() {
    // Given: two visibly distinct PNG frames and a short injected playback interval.
    let cache =
        std::env::temp_dir().join(format!("allesbuien-radar-view-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(StubFeedApi),
        radar: Arc::new(AnimatedRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: cache.clone(),
    });
    let intervals = TimerIntervals {
        radar_refresh: Duration::from_secs(300),
        radar_playback: Duration::from_millis(50),
        ..TimerIntervals::default()
    };

    // When: the real QML image binds the first provider URL and its Timer advances it.
    let booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        intervals,
    )
    .boot()
    .expect("fake-wired GUI should boot");
    pump_until(
        "the radar run exposes both frames and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let run_is_ready = booted
                .radar_controller()
                .is_some_and(|controller| *controller.frame_count() == 2);
            // SAFETY: Category 8 — Qt owns the focus-window pointer and this call
            // only tests it for null on the Qt GUI thread.
            run_is_ready && unsafe { !focus_window().is_null() }
        },
    );

    // SAFETY: Category 8 — Qt owns the focused QQuickWindow and QObject tree for
    // `booted`'s lifetime; null pointers are rejected and calls stay on the GUI thread.
    unsafe {
        let window = NonNull::new(focus_window()).expect("root QQuickWindow should be focused");
        let overlay = NonNull::new(find_child(
            window.as_ptr().cast::<QObject>(),
            &QString::from("radarOverlay"),
            &QOBJECT_STATIC_META_OBJECT,
            1,
        ))
        .expect("radarOverlay should exist");
        let source_name = CString::new("source").expect("static property name is valid");
        let status_name = CString::new("status").expect("static property name is valid");

        let deadline = Instant::now() + Duration::from_secs(1);
        let first_source = loop {
            booted.process_events();
            let source = object_property(overlay.as_ptr(), source_name.as_ptr())
                .value::<QUrl>()
                .expect("Image source should be a QUrl");
            if source.to_string().starts_with("image://radar/") {
                break source;
            }
            assert!(Instant::now() < deadline, "provider URL should reach QML");
            std::thread::yield_now();
        };
        let first_image = grab_window(window.as_ptr());

        let deadline = Instant::now() + Duration::from_secs(1);
        let second_source = loop {
            booted.process_events();
            let source = object_property(overlay.as_ptr(), source_name.as_ptr())
                .value::<QUrl>()
                .expect("Image source should be a QUrl");
            if source != first_source {
                break source;
            }
            assert!(
                Instant::now() < deadline,
                "playback Timer should advance the URL"
            );
            std::thread::yield_now();
        };
        booted.process_events();
        let second_image = grab_window(window.as_ptr());

        // Then: Qt decoded both provider frames and their rendered pixels differ.
        assert!(second_source.to_string().starts_with("image://radar/"));
        assert_eq!(
            object_property(overlay.as_ptr(), status_name.as_ptr()).value::<i32>(),
            Some(1),
            "radar overlay should report Image.Ready"
        );
        assert!(!first_image.is_null());
        assert!(!second_image.is_null());
        assert!(
            first_image != second_image,
            "rendered radar frames should differ"
        );
    }

    std::fs::remove_dir_all(cache).expect("radar view cache should be removable");
}
