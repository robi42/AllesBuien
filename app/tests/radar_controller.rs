use std::{sync::Arc, time::Duration};

use allesbuien::TimerIntervals;
use allesbuien_core::datetime::LocalTimestamp;
use allesbuien_harness::HarnessBuilder;
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};
use radar_generations::SupersededRunRadarApi;

mod radar_generations;

/// Asserts the frame caption is a wall-clock reading rather than the raw feed
/// stamp. The exact digits depend on the host timezone — including half-hour
/// zones — so only the shape is pinned here; `app/src/radar_clock.rs` owns the
/// host-independent assertions about the offset itself.
fn assert_local_clock_label(label: &str) {
    assert!(
        !label.contains('T'),
        "raw feed timestamp leaked into the frame label: {label}"
    );
    let Some((hours, minutes)) = label.split_once(':').and_then(|(hours, minutes)| {
        Some((hours.parse::<u32>().ok()?, minutes.parse::<u32>().ok()?))
    }) else {
        panic!("expected a HH:MM label, got {label}");
    };
    assert!(hours < 24 && minutes < 60, "not a wall clock: {label}");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this one-process GUI regression keeps generation replacement, playback, seeking, cancellation, and final-state assertions in one ordered narrative"
)]
fn cancels_and_awaits_superseded_refresh_before_starting_its_replacement() {
    // Given: one GUI application, deterministic radar generations, and injected timer intervals.
    let radar = Arc::new(SupersededRunRadarApi::new());
    let intervals = TimerIntervals {
        radar_refresh: Duration::from_secs(300),
        radar_playback: Duration::from_millis(400),
        ..TimerIntervals::default()
    };

    // When: construction loads generation one, playback advances, and generation two is superseded.
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .radar(radar.clone())
        .intervals(intervals)
        .boot();
    pump_until("generation one exposes two frames", SETTLE_BUDGET, || {
        booted.process_events();
        booted
            .radar_controller()
            .is_some_and(|controller| *controller.frame_count() == 2)
    });
    let controller = booted.radar_controller().expect("controller should exist");
    assert_eq!(*controller.generation(), 1);
    assert_eq!(*controller.frame_count(), 2);
    assert_eq!(*controller.current_frame_index(), 0);
    assert!(*controller.playing());
    assert_eq!(*controller.refresh_interval_ms(), 300_000);
    assert_eq!(*controller.playback_interval_ms(), 400);
    assert_eq!(controller.timestamps().len(), 2);
    assert_local_clock_label(&controller.current_frame_label().to_string());
    assert_eq!(
        controller.current_frame_label().to_string(),
        LocalTimestamp::from_feed("2026-08-30T09:00:00").time_label,
        "the vendor's declared timeOffset was ignored in favour of a derivation"
    );
    // The configured 52.37N/4.84E projects onto pixel (512.07, 433.11) of the
    // 1058x915 raster, measured against a live vendor background render.
    assert!(*controller.marker_visible());
    assert!(
        (*controller.marker_x_fraction() * 1058.0 - 512.07).abs() < 0.05,
        "marker x {}",
        *controller.marker_x_fraction() * 1058.0
    );
    assert!(
        (*controller.marker_y_fraction() * 915.0 - 433.11).abs() < 0.05,
        "marker y {}",
        *controller.marker_y_fraction() * 915.0
    );
    assert_eq!(*controller.first_forecast_index(), 2);
    assert!(
        *controller.current_frame_observed(),
        "a run whose every frame is in the past has nothing to forecast"
    );
    assert_eq!(
        controller.current_frame_url().to_string(),
        "image://radar/1/0"
    );

    booted.tick_radar();
    assert_eq!(
        *booted
            .radar_controller()
            .expect("controller should exist")
            .current_frame_index(),
        1
    );
    booted.seek_radar(9);
    {
        let controller = booted.radar_controller().expect("controller should exist");
        assert_eq!(*controller.current_frame_index(), 1);
        assert!(!*controller.playing());
        assert_eq!(
            controller.current_frame_url().to_string(),
            "image://radar/1/1"
        );
    }
    booted.seek_radar(-3);
    {
        let controller = booted.radar_controller().expect("controller should exist");
        assert_eq!(*controller.current_frame_index(), 0);
        assert_eq!(
            controller.current_frame_url().to_string(),
            "image://radar/1/0"
        );
    }
    booted.refresh_radar();
    let controller = booted.radar_controller().expect("controller should exist");
    assert_eq!(*controller.generation(), 2);
    assert_eq!(*controller.frame_count(), 0);
    assert!(!*controller.playing());
    pump_until(
        "generation two starts its metadata fetch",
        SETTLE_BUDGET,
        || radar.metadata_calls() == 2,
    );
    booted.refresh_radar();
    pump_until(
        "generation three exposes three frames",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            booted
                .radar_controller()
                .is_some_and(|controller| *controller.frame_count() == 3)
        },
    );
    // Then: generation two was cancelled before generation three touched the cache.
    let controller = booted.radar_controller().expect("controller should exist");
    assert_eq!(*controller.generation(), 3);
    assert_eq!(*controller.frame_count(), 3);
    assert_eq!(controller.timestamps().len(), 3);
    assert_local_clock_label(&controller.current_frame_label().to_string());
    assert_eq!(
        controller.current_frame_url().to_string(),
        "image://radar/3/0"
    );
    let ordering = radar.recorded();
    let cancelled_position = ordering
        .iter()
        .position(|event| event == "metadata-stale-cancelled")
        .expect("superseded metadata fetch should be cancelled");
    let replacement_position = ordering
        .iter()
        .position(|event| event == "metadata-3")
        .expect("replacement metadata fetch should start");
    assert!(cancelled_position < replacement_position);
    assert!(
        !ordering
            .iter()
            .any(|event| event.starts_with("frame-fake://stale/"))
    );
}
