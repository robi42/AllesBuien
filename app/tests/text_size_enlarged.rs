//! A text size read while the window launches leaves a window the person
//! enlarged at the size they gave it.
//!
//! `Main.qml` binds the window's size to the opening size the theme bridge
//! publishes, so a published opening size that moves resizes the window to
//! it, whatever size the window had. The opening size therefore moves only
//! when the window is created and when it lands on another screen during the
//! launch; a text size changes the zoom and the minimum, and Qt grows a window
//! that has become too small by itself. This harness opens a window at the
//! default 12 px on a 1920×1080 monitor, enlarges it to 1600×1000, and reads
//! 14 px while it is still launching. When the opening size was raised to
//! every new minimum, that poll cut the window down to 994×784.

use std::{
    fs,
    ptr::NonNull,
    sync::Arc,
    time::{Duration, Instant},
};

use allesbuien::TimerIntervals;
use allesbuien_harness::{
    Harness, HarnessBuilder,
    probe::{QObject, QQuickWindow, focused_window, number_of},
    resize::resize,
};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};

/// The size the person drags the window to: larger than every size the
/// bridge publishes in this harness.
const ENLARGED: (i32, i32) = (1600, 1000);

/// The window's logical size, once it has focus.
fn window_size() -> Option<(f64, f64)> {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe {
        let window = focused_window()?.cast::<QObject>();
        Some((number_of(window, "width"), number_of(window, "height")))
    }
}

/// Whether the window has focus and is `size`, within the one logical pixel a
/// display scale below 1 can round a window's size by.
fn has_size(size: (i32, i32)) -> bool {
    window_size().is_some_and(|(width, height)| {
        (width - f64::from(size.0)).abs() <= 1.0 && (height - f64::from(size.1)).abs() <= 1.0
    })
}

/// What the theme bridge publishes: the zoom, the minimum, and the opening size.
fn published(booted: &Harness) -> (i32, (i32, i32), (i32, i32)) {
    let bridge = booted.theme_controller().expect("the theme bridge exists");
    (
        *bridge.zoom_parts(),
        (*bridge.minimum_width(), *bridge.minimum_height()),
        (*bridge.opening_width(), *bridge.opening_height()),
    )
}

/// The focused window, to resize.
fn window() -> NonNull<QQuickWindow> {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe { focused_window() }.expect("the window has focus")
}

/// Runs the event loop for a while, long enough for a resize to land.
fn settle(booted: &Harness) {
    let until = Instant::now() + Duration::from_millis(200);
    pump_until("the window settles", SETTLE_BUDGET, || {
        booted.process_events();
        Instant::now() >= until
    });
}

fn launching(booted: &Harness) -> bool {
    *booted
        .theme_controller()
        .expect("the theme bridge exists")
        .launching()
}

#[test]
fn a_text_size_read_while_launching_leaves_an_enlarged_window_alone() {
    // Given: a window launching at the default 12 px on a 1920×1080 monitor.
    // The theme timer is an hour, so its own polls cannot end the launch
    // before the one this harness makes.
    let root = tempfile::tempdir().expect("a temporary root");
    let shell_config = root.path().join("shell.toml");
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .shell_config(shell_config.clone())
        .intervals(TimerIntervals {
            theme_poll: Duration::from_secs(3600),
            ..TimerIntervals::default()
        })
        .boot();
    booted.track_screen(1920, 1080, 1920, 1080);
    pump_until("the window has focus at 980×760", SETTLE_BUDGET, || {
        booted.process_events();
        has_size((980, 760))
    });

    // When: the person enlarges it, and a poll then reads 14 px, while the
    // window is still launching.
    // SAFETY: the window came from `focused_window` on the live booted app,
    // and this runs on its GUI thread.
    unsafe { resize(window(), ENLARGED.0, ENLARGED.1) };
    pump_until("the window is enlarged", SETTLE_BUDGET, || {
        booted.process_events();
        has_size(ENLARGED)
    });
    fs::write(&shell_config, "[font]\nbase-size = 14\n").expect("the setting is written");
    booted.poll_theme();
    assert!(launching(&booted), "the poll fell after the launch");

    // Then: the zoom and the minimum follow the text size, the opening size
    // does not, and the window keeps the size the person gave it.
    assert_eq!(published(&booted), (14_000, (994, 784), (980, 760)));
    settle(&booted);
    assert!(
        has_size(ENLARGED),
        "the poll resized the window to {:?}",
        window_size()
    );
}
