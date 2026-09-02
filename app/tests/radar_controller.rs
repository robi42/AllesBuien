use std::{path::PathBuf, sync::Arc, time::Duration};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_core::datetime::LocalTimestamp;
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, StubNotifier, amsterdam_west, pump_until};
use radar_generations::{FakeRadarApi, Gate};

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
fn ignores_stale_generation_frame_and_refresh_started_resets_state() {
    // Given: one GUI application, deterministic radar generations, and injected timer intervals.
    let stale_gate = Arc::new(Gate::new());
    let radar = Arc::new(FakeRadarApi::new(Arc::clone(&stale_gate)));
    let cache = std::env::temp_dir().join(format!("allesbuien-radar-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(StubFeedApi),
        radar: radar.clone(),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: cache.clone(),
    });
    let intervals = TimerIntervals {
        radar_refresh: Duration::from_secs(300),
        radar_playback: Duration::from_millis(400),
        ..TimerIntervals::default()
    };

    // When: construction loads generation one, playback advances, and generation two is superseded.
    let mut booted = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        intervals,
    )
    .boot()
    .expect("fake-wired GUI should boot");
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
    stale_gate.release();
    pump_until(
        "the released stale generation finishes",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            radar
                .recorded()
                .iter()
                .any(|event| event == "frame-fake://stale/0")
        },
    );

    // Then: the stale completion ran after the fresh one but could not replace generation three.
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
    let fresh_position = ordering
        .iter()
        .position(|event| event == "frame-fake://fresh/2")
        .expect("fresh generation should finish");
    let stale_position = ordering
        .iter()
        .position(|event| event == "frame-fake://stale/0")
        .expect("stale generation should finish");
    assert!(fresh_position < stale_position);
    std::fs::remove_dir_all(cache).expect("radar test cache should be removable");
}
