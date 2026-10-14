use std::{sync::Arc, time::Duration};

use allesbuien::TimerIntervals;
use allesbuien_core::radar::{RadarApi, RadarFuture};
use allesbuien_harness::{
    HarnessBuilder,
    probe::{child, focused_window, grab_window, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};
use cxx_qt_lib::QUrl;

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
                Ok(include_bytes!("../../preview.png").to_vec())
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
    let intervals = TimerIntervals {
        radar_refresh: Duration::from_secs(300),
        radar_playback: Duration::from_millis(50),
        ..TimerIntervals::default()
    };

    // When: the real QML image binds the first provider URL and its Timer advances it.
    let booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .radar(Arc::new(AnimatedRadarApi))
        .intervals(intervals)
        .boot();
    pump_until(
        "the radar run exposes both frames and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let run_is_ready = booted
                .radar_controller()
                .is_some_and(|controller| *controller.frame_count() == 2);
            // SAFETY: on the GUI thread of the live booted app.
            run_is_ready && unsafe { focused_window() }.is_some()
        },
    );

    // SAFETY: the window and every item come from the probes on `booted`, which
    // outlives this block, and every call stays on the GUI thread.
    unsafe {
        let window = focused_window().expect("root QQuickWindow should be focused");
        let overlay = child(window, "radarOverlay").expect("radarOverlay should exist");
        let source = || {
            property(overlay, "source")
                .value::<QUrl>()
                .expect("Image source should be a QUrl")
        };

        let mut first_source = None;
        pump_until("the provider URL reaches QML", SETTLE_BUDGET, || {
            booted.process_events();
            first_source =
                Some(source()).filter(|url| url.to_string().starts_with("image://radar/"));
            first_source.is_some()
        });
        let first_source = first_source.expect("the wait returned with a provider URL");
        // Wait for the first frame itself to report Image.Ready before grabbing
        // it: a frame that never decoded leaves only the basemap underneath,
        // which can still differ from the second grab, so the comparison below
        // would not notice. Playback keeps advancing, so the source is pinned
        // too, or the next frame's Ready would answer for this one.
        pump_until(
            "the first provider frame reports Image.Ready",
            SETTLE_BUDGET,
            || {
                booted.process_events();
                source() == first_source && property(overlay, "status").value::<i32>() == Some(1)
            },
        );
        let first_image = grab_window(window);
        let mut second_source = None;
        pump_until("the playback Timer advances the URL", SETTLE_BUDGET, || {
            booted.process_events();
            second_source = Some(source()).filter(|url| *url != first_source);
            second_source.is_some()
        });
        let second_source = second_source.expect("the wait returned with an advanced URL");
        // The same wait for the second frame. One more pass of the event loop
        // here could fire the 50 ms Timer again and put frame 0 back, so the
        // second grab would capture the first frame again; nothing runs the
        // loop between this wait's last check and the grab.
        pump_until(
            "the second provider frame reports Image.Ready",
            SETTLE_BUDGET,
            || {
                booted.process_events();
                source() == second_source && property(overlay, "status").value::<i32>() == Some(1)
            },
        );
        let second_image = grab_window(window);

        // Then: Qt decoded both provider frames and their rendered pixels differ.
        assert!(second_source.to_string().starts_with("image://radar/"));
        assert_eq!(
            property(overlay, "status").value::<i32>(),
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
}
