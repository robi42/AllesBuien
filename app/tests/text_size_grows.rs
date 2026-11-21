//! A larger text size read after the launch grows a window that no longer
//! fits to its new minimum, with the opening size left where it was.
//!
//! A text size moves only the zoom and the minimum, so the theme bridge never
//! asks for this resize: Qt makes it, because a window's minimum that rises
//! past its size resizes the window. This harness holds the app to that rather
//! than to the bridge's numbers alone. A window launches at 12 px on a
//! 1920×1080 monitor, which leaves room for 14 px, opens at 980×760, and is
//! left to end its launch by its own 25 ms theme timer. The setting then
//! changes to 14 px, whose 994×784 minimum is larger on both axes, with no
//! screen reported in between. The window has to grow to that minimum, the
//! published opening size has to stay 980×760, and the zoomed content has to
//! fill the grown window, which the play button's rendered fill and the zoom
//! wrapper's corners show.

mod zoomed_window;

use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use allesbuien::TimerIntervals;
use allesbuien_core::window_fit::Size;
use allesbuien_harness::{Harness, HarnessBuilder};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};
use zoomed_window::{NORD, assert_zoomed, has_size, holds, write_text_size};

/// The 980×760 the window opens at, which 12 px leaves as designed.
const OPENING: Size = Size {
    width: 980,
    height: 760,
};

/// The 852×672 minimum zoomed to 14 px, past the opening size on both axes.
const GROWN: Size = Size {
    width: 994,
    height: 784,
};

/// What the theme bridge publishes: the zoom, the minimum, and the opening size.
fn published(booted: &Harness) -> (i32, (i32, i32), (i32, i32)) {
    let bridge = booted.theme_controller().expect("the theme bridge exists");
    (
        *bridge.zoom_parts(),
        (*bridge.minimum_width(), *bridge.minimum_height()),
        (*bridge.opening_width(), *bridge.opening_height()),
    )
}

fn waits_for_zoom(booted: &Harness, zoom: i32) {
    pump_until(&format!("the zoom becomes {zoom}"), SETTLE_BUDGET, || {
        booted.process_events();
        published(booted).0 == zoom
    });
}

fn launching(booted: &Harness) -> bool {
    *booted
        .theme_controller()
        .expect("the theme bridge exists")
        .launching()
}

/// Boots a window at 12 px over Nord, polling the theme every 25 ms, and has
/// it land on a 1920×1080 monitor before it has run any event.
fn launch(theme: PathBuf, shell_config: PathBuf) -> Harness {
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .theme_path(theme)
        .shell_config(shell_config)
        .intervals(TimerIntervals {
            theme_poll: Duration::from_millis(25),
            ..TimerIntervals::default()
        })
        .boot();
    booted.track_screen(1920, 1080, 1920, 1080);
    booted
}

#[test]
fn a_larger_text_size_after_the_launch_grows_the_window_to_its_minimum() {
    // Given: a window launched at 12 px on a 1920×1080 monitor, open at
    // 980×760 and done launching.
    let root = tempfile::tempdir().expect("a temporary root");
    let theme = root.path().join("theme");
    fs::create_dir(&theme).expect("the theme directory is created");
    fs::write(theme.join("colors.toml"), NORD).expect("the theme is written");
    let shell_config = root.path().join("shell.toml");
    write_text_size(&shell_config, 12);
    let booted = launch(theme, shell_config.clone());
    assert_eq!(published(&booted), (12_000, (852, 672), (980, 760)));
    pump_until("the window has focus at 980×760", SETTLE_BUDGET, || {
        booted.process_events();
        has_size(OPENING)
    });
    pump_until("the launch ends", SETTLE_BUDGET, || {
        booted.process_events();
        !launching(&booted)
    });

    // When: the text size becomes 14 px, and the theme timer reads it.
    write_text_size(&shell_config, 14);
    waits_for_zoom(&booted, 14_000);

    // Then: the bridge raises the minimum past the window and leaves the
    // opening size alone, and the window grows to the minimum all the same.
    assert_eq!(published(&booted), (14_000, (994, 784), (980, 760)));
    pump_until(
        "the window grows to its 14 px minimum",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            holds(GROWN)
        },
    );
    assert!(
        has_size(GROWN),
        "the window grew past its minimum rather than to it"
    );

    // And: the content is zoomed to 14 px and fills the grown window.
    assert_zoomed("at 14 px", 14_000);
}
