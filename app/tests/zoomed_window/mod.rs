//! The zoomed window as the text-size harnesses read it: the theme and the
//! text-size setting they boot over, the window's size, and whether its
//! content is zoomed, by the wrapper's scale and corners and by the play
//! button's rendered fill, since a zoom is only proven by what it draws.
//!
//! Every reading looks the window up again rather than holding a pointer, so
//! none can outlive the application the harness booted, and each is in
//! logical units: a grab is in physical pixels, so its readings are divided by
//! the image's width over the window's, as `icon_skeleton` does. A display
//! scale below 1 makes a device pixel larger than a logical one, so readings
//! taken off pixels carry that pixel's size, and a window's size is matched
//! within one logical pixel: at `QT_SCALE_FACTOR=0.5` a 1037-high window is
//! 518.5 device pixels, which Qt rounds, and it comes back 1038 high.

use std::{fs, path::Path, ptr::NonNull};

use allesbuien_core::window_fit::{Size, ZOOM_UNIT};
use allesbuien_harness::probe::{
    QObject, QQuickWindow, child, focused_window, grab_window, map_to_scene, number_of,
};
use allesbuien_testkit::{Rgb, VISIBLE_LEVELS, spread};

/// Omarchy's Nord, so the play button's fill stands out from its bar.
pub(crate) const NORD: &str = "background = \"#2e3440\"\nforeground = \"#d8dee9\"\n\
    accent = \"#81a1c1\"\nselection = \"#434c5e\"\nlighter_background = \"#3b4252\"\n\
    muted = \"#4c566a\"\n";

/// The play button's side, in layout units, from `RadarTransport.qml`.
const BUTTON_SIDE: f64 = 36.0;

/// How far left of the play button its background is sampled, in layout units:
/// inside the transport bar's own margin.
const GROUND_OFFSET: f64 = 4.0;

/// Writes `size` as the text size, as `omarchy-display-text-size` does: into a
/// new file that is renamed over the old one.
pub(crate) fn write_text_size(shell_config: &Path, size: u32) {
    let next = shell_config.with_extension("next");
    fs::write(&next, format!("[font]\nbase-size = {size}\n")).expect("the setting is written");
    fs::rename(next, shell_config).expect("the setting replaces the old one");
}

/// The focused window and its item named `name`.
///
/// # Panics
///
/// When the window has focus but holds no such item.
pub(crate) fn item(name: &str) -> Option<(NonNull<QQuickWindow>, NonNull<QObject>)> {
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

/// Whether the window has focus and is no smaller than `minimum`, within one
/// logical pixel.
pub(crate) fn holds(minimum: Size) -> bool {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    let found = unsafe {
        let Some(window) = focused_window() else {
            return false;
        };
        let window = window.cast::<QObject>();
        (number_of(window, "width"), number_of(window, "height"))
    };
    found.0 >= f64::from(minimum.width) - 1.0 && found.1 >= f64::from(minimum.height) - 1.0
}

/// The zoom wrapper's `scale`.
fn scale() -> f64 {
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
fn fill_width() -> (f64, f64) {
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

/// Requires the content to be zoomed by `zoom`: the rendered button first, so
/// a wrapper that does not scale is caught by its pixels, then the wrapper's
/// corners, then its scale, which pins `Theme.qml`'s 12000 to core's unit.
pub(crate) fn assert_zoomed(phase: &str, zoom: u32) {
    let factor = f64::from(zoom) / f64::from(ZOOM_UNIT);
    let (fill, pixel) = fill_width();
    assert!(
        (fill - BUTTON_SIDE * factor).abs() <= pixel.max(1.0),
        "{phase}: the play button is drawn {fill} wide, not {}",
        BUTTON_SIDE * factor
    );
    assert_content_fills(phase);
    assert!(
        (scale() - factor).abs() < 1e-9,
        "{phase}: the content is scaled by {}, not {factor}",
        scale()
    );
}
