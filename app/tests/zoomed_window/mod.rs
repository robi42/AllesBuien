//! The zoomed window as `text_size_zoom` reads it: its size, the zoom wrapper's
//! scale and corners, the screen QML reports, the play button's rendered fill,
//! and the play glyph's raster.
//!
//! Every reading looks the window up again rather than holding a pointer, so
//! none can outlive the application the harness booted, and each is in
//! logical units: a grab is in physical pixels, so its readings are divided by
//! the image's width over the window's, as `icon_skeleton` does. A display
//! scale below 1 makes a device pixel larger than a logical one, so readings
//! taken off pixels carry that pixel's size, and a window's size is matched
//! within one logical pixel: at `QT_SCALE_FACTOR=0.5` a 1037-high window is
//! 518.5 device pixels, which Qt rounds, and it comes back 1038 high.

use std::ptr::NonNull;

use allesbuien_core::window_fit::{Screen, Size};
use allesbuien_harness::probe::{
    QObject, QQuickWindow, child, focused_window, grab_window, map_to_scene, number_of, property,
};
use allesbuien_testkit::{Rgb, VISIBLE_LEVELS, spread};

/// How far left of the play button its background is sampled, in layout units:
/// inside the transport bar's own margin.
const GROUND_OFFSET: f64 = 4.0;

/// The focused window and its item named `name`.
///
/// # Panics
///
/// When the window has focus but holds no such item.
fn item(name: &str) -> Option<(NonNull<QQuickWindow>, NonNull<QObject>)> {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe {
        let window = focused_window()?;
        let item = child(window, name).unwrap_or_else(|| panic!("the window holds no {name}"));
        Some((window, item))
    }
}

/// Whether the window has focus and is `size`, within one logical pixel.
pub(crate) fn has_size(size: Size) -> bool {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    let found = unsafe {
        let Some(window) = focused_window() else {
            return false;
        };
        let window = window.cast::<QObject>();
        (number_of(window, "width"), number_of(window, "height"))
    };
    (found.0 - f64::from(size.width)).abs() <= 1.0
        && (found.1 - f64::from(size.height)).abs() <= 1.0
}

/// The screen `ScreenReport.qml` shows in its four mirrored properties.
pub(crate) fn reported_screen() -> Option<Screen> {
    let (_, report) = item("screenReport")?;
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

/// The zoom wrapper's `scale`.
pub(crate) fn scale() -> f64 {
    let (_, zoom_root) = item("zoomRoot").expect("the window has focus");
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe { number_of(zoom_root, "scale") }
}

/// Requires the zoom wrapper's corners to land on the window's, within 1 px:
/// laid out at the window's size over the zoom and scaled from its top-left
/// corner, it fills the window exactly, neither clipped nor short of an edge.
pub(crate) fn assert_content_fills(phase: &str) {
    let (window, zoom_root) = item("zoomRoot").expect("the window has focus");
    // SAFETY: called only on the GUI thread while the booted app is alive, and
    // `zoomRoot` is a `QQuickItem`.
    let (top_left, bottom_right, window_size) = unsafe {
        let (width, height) = (
            number_of(zoom_root, "width"),
            number_of(zoom_root, "height"),
        );
        let window = window.cast::<QObject>();
        (
            map_to_scene(zoom_root, 0.0, 0.0),
            map_to_scene(zoom_root, width, height),
            (number_of(window, "width"), number_of(window, "height")),
        )
    };
    let near = |found: f64, wanted: f64| (found - wanted).abs() <= 1.0;
    assert!(
        near(top_left.x(), 0.0) && near(top_left.y(), 0.0),
        "{phase}: the content starts at {top_left:?}, not the window's corner"
    );
    assert!(
        near(bottom_right.x(), window_size.0) && near(bottom_right.y(), window_size.1),
        "{phase}: the content ends at {bottom_right:?}, not at the window's {window_size:?}"
    );
}

/// The play button's rendered fill, measured along a row a quarter of the way
/// down the button, clear of the glyph, and the size of the device pixel it was
/// counted in, both in logical units.
pub(crate) fn fill_width() -> (f64, f64) {
    let (window, button) = item("playButton").expect("the window has focus");
    // SAFETY: called only on the GUI thread while the booted app is alive, and
    // the play button is a `QQuickItem`.
    let (image, ratio, inside, ground) = unsafe {
        let row = number_of(button, "height") / 4.0;
        let inside = map_to_scene(button, number_of(button, "width") / 2.0, row);
        let ground = map_to_scene(button, -GROUND_OFFSET, row);
        let image = grab_window(window);
        let ratio = f64::from(image.width()) / number_of(window.cast(), "width");
        (
            image,
            ratio,
            (inside * ratio).to_point(),
            (ground * ratio).to_point(),
        )
    };
    let pixel = |x: i32| -> Rgb {
        let colour = image.pixel_color(x, inside.y());
        [colour.red(), colour.green(), colour.blue()]
    };
    let (fill, background) = (pixel(inside.x()), pixel(ground.x()));
    assert!(
        spread(fill, background) >= VISIBLE_LEVELS,
        "the play button's fill {fill:?} cannot be told from its ground {background:?}"
    );
    let filled = |x: i32| {
        (0..image.width()).contains(&x) && spread(pixel(x), fill) < spread(pixel(x), background)
    };
    let (mut left, mut right) = (inside.x(), inside.x());
    while filled(left - 1) {
        left -= 1;
    }
    while filled(right + 1) {
        right += 1;
    }
    (f64::from(right - left + 1) / ratio, 1.0 / ratio)
}

/// The play glyph's `width` and `scale`.
pub(crate) fn glyph() -> (f64, f64) {
    let (_, glyph) = item("transportGlyph").expect("the window has focus");
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe { (number_of(glyph, "width"), number_of(glyph, "scale")) }
}
