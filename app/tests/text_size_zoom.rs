//! The window follows Omarchy's text size by zooming its whole content.
//!
//! One window launches at 20 px over a Nord theme and a temporary
//! `shell.toml`, which the test rewrites the way `omarchy-display-text-size`
//! does: a new file renamed over the old one. The window's own 25 ms theme
//! timer delivers every change to that file; the test only waits. It reads the
//! bridge, the window's size, the zoom wrapper's corners, and rendered pixels,
//! because a zoom is only proven by what it draws.
//!
//! Nothing assumes the offscreen screen's size, which is 800×800 under
//! `bin/test`, 400×400 under `QT_SCALE_FACTOR=2` and 1600×1600 under 0.5: the
//! launch values are checked against core's own fit for the screen QML says it
//! reported. Where that screen caps 20 px, as the gate's does, they match only
//! if QML did report it, since without a report nothing caps the zoom. Two
//! other monitors are stood in for through `track_screen`, the hand-off's
//! entry point.

mod zoomed_window;

use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

use allesbuien::{TimerIntervals, theme_controller::WINDOW};
use allesbuien_core::window_fit::{Screen, Size, WindowFit, WindowFitter, ZOOM_UNIT};
use allesbuien_harness::{
    Harness, HarnessBuilder,
    probe::{number_of, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};
use zoomed_window::{NORD, assert_zoomed, write_text_size};

/// The play glyph's side, in layout units, from `RadarTransport.qml`.
const GLYPH_SIDE: f64 = 12.0;

/// The window the 1920×1080 monitor fits at 20 px: the opening size it lands at.
const LANDED: Size = Size {
    width: 1512,
    height: 1037,
};

fn fit(zoom: u32, minimum: (u32, u32), opening: Size) -> WindowFit {
    let (width, height) = minimum;
    WindowFit {
        zoom,
        minimum: Size { width, height },
        opening,
    }
}

/// What the theme bridge publishes, as core's own fit.
fn bridge_fit(booted: &Harness) -> WindowFit {
    let bridge = booted.theme_controller().expect("the theme bridge exists");
    let side = |value: &i32| u32::try_from(*value).expect("a published size is not negative");
    WindowFit {
        zoom: side(bridge.zoom_parts()),
        minimum: Size {
            width: side(bridge.minimum_width()),
            height: side(bridge.minimum_height()),
        },
        opening: Size {
            width: side(bridge.opening_width()),
            height: side(bridge.opening_height()),
        },
    }
}

fn launching(booted: &Harness) -> bool {
    *booted
        .theme_controller()
        .expect("the theme bridge exists")
        .launching()
}

fn waits_for_zoom(booted: &Harness, zoom: u32) {
    pump_until(&format!("the zoom becomes {zoom}"), SETTLE_BUDGET, || {
        booted.process_events();
        bridge_fit(booted).zoom == zoom
    });
}

/// Requires the window to be no smaller than the minimum the bridge publishes.
fn assert_holds_minimum(booted: &Harness, phase: &str) {
    let minimum = bridge_fit(booted).minimum;
    assert!(
        zoomed_window::holds(minimum),
        "{phase}: the window is smaller than its minimum {minimum:?}"
    );
}

/// Runs the event loop for a while, long enough for several theme polls.
fn settle(booted: &Harness) {
    let until = Instant::now() + Duration::from_millis(200);
    pump_until("a few theme polls run", SETTLE_BUDGET, || {
        booted.process_events();
        Instant::now() >= until
    });
}

/// The screen `ScreenReport.qml` shows in its four mirrored properties.
fn reported_screen() -> Option<Screen> {
    let (_, report) = zoomed_window::item("screenReport")?;
    // SAFETY: called only on the GUI thread while the booted app is alive.
    let side = |name| {
        unsafe { property(report, name) }
            .value::<i32>()
            .unwrap_or(0)
    };
    Screen::from_report(
        side("screenWidth"),
        side("screenHeight"),
        side("workWidth"),
        side("workHeight"),
    )
}

/// The play glyph's `width` and `scale`.
fn glyph() -> (f64, f64) {
    let (_, glyph) = zoomed_window::item("transportGlyph").expect("the window has focus");
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe { (number_of(glyph, "width"), number_of(glyph, "scale")) }
}

/// Before any event has run, the window is launching, and a report of the
/// monitor it lands on re-fits its opening size. Returns the fit at launch.
fn lands_on_a_monitor_while_launching(booted: &mut Harness) -> WindowFit {
    let launched = bridge_fit(booted);
    assert!(
        launching(booted),
        "a window that has run no event is launching"
    );
    booted.track_screen(1920, 1080, 1920, 1080);
    assert_eq!(bridge_fit(booted), fit(18_517, (1315, 1037), LANDED));
    launched
}

/// The window shows the zoom its screen allows, at the size it landed at, and
/// the glyph's raster grows with the zoom while its footprint does not.
fn shows_the_zoom_where_it_landed(booted: &Harness, launched: WindowFit) {
    pump_until(
        "the window has focus at its landed size",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            zoomed_window::has_size(LANDED)
        },
    );
    let reported = reported_screen().expect("QML shows a whole screen");
    let mut expected = WindowFitter::new(WINDOW, 20);
    expected.track_screen(reported);
    assert_eq!(launched, expected.fit(), "the launch fit for {reported:?}");
    assert_zoomed("at 20 px", 18_517);
    assert_holds_minimum(booted, "at 20 px");
    let (width, scale) = glyph();
    let factor = 18_517.0 / f64::from(ZOOM_UNIT);
    assert!(
        (width - GLYPH_SIDE * factor).abs() <= 0.5,
        "the glyph is painted {width} wide"
    );
    assert!(
        (width * scale - GLYPH_SIDE).abs() < 1e-6,
        "the glyph takes {width}×{scale}"
    );
}

/// Once the launch is over, another monitor moves the zoom and the minimum
/// but never the opening size, and the window stays as it is.
fn keeps_its_size_after_the_launch(booted: &mut Harness) {
    pump_until("the launch ends", SETTLE_BUDGET, || {
        booted.process_events();
        !launching(booted)
    });
    booted.track_screen(1280, 800, 1280, 800);
    assert_eq!(bridge_fit(booted), fit(13_517, (960, 757), LANDED));
    settle(booted);
    assert!(
        zoomed_window::has_size(LANDED),
        "a monitor resized the window"
    );
    assert_holds_minimum(booted, "on a smaller monitor");
}

#[test]
fn the_window_zooms_its_content_with_omarchys_text_size() {
    // Given: a window launched at 20 px over Nord, polling the theme every 25 ms.
    let root = tempfile::tempdir().expect("a temporary root");
    let theme = root.path().join("theme");
    fs::create_dir(&theme).expect("the theme directory is created");
    fs::write(theme.join("colors.toml"), NORD).expect("the theme is written");
    let shell_config = root.path().join("shell.toml");
    write_text_size(&shell_config, 20);
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .theme_path(theme)
        .shell_config(shell_config.clone())
        .intervals(TimerIntervals {
            theme_poll: Duration::from_millis(25),
            ..TimerIntervals::default()
        })
        .boot();

    // When / Then: it lands on a 1920×1080 monitor and launches there.
    let launched = lands_on_a_monitor_while_launching(&mut booted);
    shows_the_zoom_where_it_landed(&booted, launched);
    keeps_its_size_after_the_launch(&mut booted);

    // When / Then: the text size shrinks to 9 px, live.
    write_text_size(&shell_config, 9);
    waits_for_zoom(&booted, 9_000);
    assert_eq!(bridge_fit(&booted), fit(9_000, (639, 504), LANDED));
    assert_zoomed("at 9 px", 9_000);
    assert!(
        zoomed_window::has_size(LANDED),
        "a smaller text size resized the window"
    );
    assert_holds_minimum(&booted, "at 9 px");

    // When / Then: `reset` removes the line, here the whole file.
    fs::remove_file(&shell_config).expect("the setting is removed");
    waits_for_zoom(&booted, ZOOM_UNIT);
    assert_eq!(bridge_fit(&booted), fit(ZOOM_UNIT, (852, 672), LANDED));
    zoomed_window::assert_content_fills("at 12 px");
    assert_holds_minimum(&booted, "at 12 px");
}
