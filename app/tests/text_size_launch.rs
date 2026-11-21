//! A larger text size read while the window launches never leaves it below its
//! minimum.
//!
//! The opening size is fitted at the text size read at creation and the
//! minimum at the live one, so a larger size read during the launch raises the
//! minimum past the opening size, and a screen reported after it fits the
//! opening size again, which Qt then resizes the window to, minimum or not.
//! Launched at 14 px, a poll reading 20 px, then the window landing on a
//! 1920×1080 monitor: before a fitting raised the opening size to the minimum
//! of its moment, that resized the window to 1143×887 under a 1315×1037
//! minimum. This harness runs that sequence before the window's first frame,
//! so all of it falls within the launch, and requires the bridge to ask for no
//! less than the minimum and the window to come up no smaller.

use std::{fs, sync::Arc};

use allesbuien_harness::{
    Harness, HarnessBuilder,
    probe::{QObject, focused_window, number_of},
};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};

/// The minimum at 20 px on a 1920×1080 monitor, below the bar's 43 px.
const MINIMUM: (i32, i32) = (1315, 1037);

/// The window's logical size, once it has focus.
fn window_size() -> Option<(f64, f64)> {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe {
        let window = focused_window()?.cast::<QObject>();
        Some((number_of(window, "width"), number_of(window, "height")))
    }
}

/// What the theme bridge asks the window for: its minimum and opening sizes.
fn requested(booted: &Harness) -> ((i32, i32), (i32, i32)) {
    let bridge = booted.theme_controller().expect("the theme bridge exists");
    (
        (*bridge.minimum_width(), *bridge.minimum_height()),
        (*bridge.opening_width(), *bridge.opening_height()),
    )
}

/// Whether `found` is `wanted` within one logical pixel, which is what a
/// display scale below 1 can round a window's size by.
fn near(found: f64, wanted: i32) -> bool {
    (found - f64::from(wanted)).abs() <= 1.0
}

#[test]
fn a_larger_text_size_read_while_launching_keeps_the_window_at_its_minimum() {
    // Given: a window launching at 14 px.
    let root = tempfile::tempdir().expect("a temporary root");
    let shell_config = root.path().join("shell.toml");
    fs::write(&shell_config, "[font]\nbase-size = 14\n").expect("the setting is written");
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .shell_config(shell_config.clone())
        .boot();

    // When: before its first frame, a poll reads 20 px, as the window's own
    // timer would, and the window lands on a 1920×1080 monitor.
    fs::write(&shell_config, "[font]\nbase-size = 20\n").expect("the setting is rewritten");
    booted.poll_theme();
    booted.track_screen(1920, 1080, 1920, 1080);

    // Then: the bridge asks for no less than the minimum, 1143×887 having been
    // the opening size fitted at 14 px there.
    assert_eq!(requested(&booted), (MINIMUM, MINIMUM));

    // And: the window comes up at it.
    pump_until(
        "the window has focus at its opening size",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let (_, opening) = requested(&booted);
            window_size()
                .is_some_and(|(width, height)| near(width, opening.0) && near(height, opening.1))
        },
    );
    let (width, height) = window_size().expect("the window has focus");
    assert!(
        width >= f64::from(MINIMUM.0) - 1.0 && height >= f64::from(MINIMUM.1) - 1.0,
        "the window is {width}×{height}, below its {MINIMUM:?} minimum"
    );
}
