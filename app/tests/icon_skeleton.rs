//! A pending condition icon shows a skeleton that can be seen, on dark and light.
//!
//! Twice the skeleton passed its binding checks and still could not be seen:
//! first it was filled with its card's own colour, then with a tone whose whole
//! pulse moved four levels per channel. So this harness reads rendered pixels.
//! It holds the first refresh open forever, samples the hero skeleton's centre
//! and the card beside it frame after frame, and requires both a visible
//! contrast in every frame and a visible pulse across frames.

use std::{
    fs::{self, FileTimes},
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime},
};

use allesbuien_core::weather::{FeedApi, FeedFuture};
use allesbuien_harness::{
    Harness, HarnessBuilder,
    probe::{child, focused_window, grab_window, map_to_scene, number_of, property},
};
use allesbuien_testkit::{Rgb, SETTLE_BUDGET, VISIBLE_LEVELS, pump_until, spread};
use cxx_qt_lib::QPointF;

/// How far left of the hero icon the card is sampled: inside the card's padding.
const CARD_OFFSET: f64 = 8.0;

/// Omarchy's Nord, a dark theme.
const NORD: &str = "mode = \"dark\"\nbackground = \"#2e3440\"\nforeground = \"#d8dee9\"\n\
    accent = \"#81a1c1\"\nselection = \"#434c5e\"\nlighter_background = \"#3b4252\"\n\
    muted = \"#4c566a\"\n";

/// Omarchy's Flexoki Light, so the tint is proven against a light card too.
const FLEXOKI_LIGHT: &str = "mode = \"light\"\nbackground = \"#FFFCF0\"\n\
    foreground = \"#100F0F\"\naccent = \"#205EA6\"\nselection = \"#CECDC3\"\n\
    lighter_background = \"#E6E4D9\"\nmuted = \"#B7B5AC\"\n";

/// A feed whose answer never arrives, so the window stays in its loading state.
struct SilentFeedApi;

impl FeedApi for SilentFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(std::future::pending())
    }
}

/// Writes `colors` as the theme, stamped `modified` seconds after the epoch so
/// the controller's modification-time poll sees each write as a change.
fn write_theme(theme: &Path, colors: &str, modified: u64) {
    let path = theme.join("colors.toml");
    fs::write(&path, colors).expect("the theme should be written");
    fs::File::options()
        .write(true)
        .open(path)
        .expect("the theme should reopen")
        .set_times(
            FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(modified)),
        )
        .expect("the theme's modification time should be set");
}

/// The rendered pixels at the hero skeleton's centre and at the card beside it,
/// once the skeleton is on screen.
fn sample() -> Option<(Rgb, Rgb)> {
    // SAFETY: called only on the GUI thread while the booted app is alive; the
    // hero icon and its skeleton are `QQuickItem`s.
    unsafe {
        let window = focused_window()?;
        let hero = child(window, "heroIcon")?;
        let shown = property(hero, "loading").value::<bool>() == Some(true);
        let middle = number_of(hero, "height") / 2.0;
        let centre = map_to_scene(hero, number_of(hero, "width") / 2.0, middle);
        let card = map_to_scene(hero, -CARD_OFFSET, middle);
        let image = grab_window(window);
        // The scene is in logical units and the grab in physical pixels, so a
        // scaled desktop (`QT_SCALE_FACTOR=2`, a HiDPI shell) needs the ratio
        // between them. The window's own width measures it without trusting the
        // grab to carry its device pixel ratio.
        let ratio = f64::from(image.width()) / number_of(window.cast(), "width");
        let pixel = |point: QPointF| {
            // `toPoint` rounds to the nearest pixel, which is the pixel meant.
            let point = (point * ratio).to_point();
            let color = image.pixel_color(point.x(), point.y());
            [color.red(), color.green(), color.blue()]
        };
        shown.then(|| (pixel(centre), pixel(card)))
    }
}

/// Pumps frames until the skeleton's pulse has spanned a visible range, and
/// requires it to stand out from its card in every frame on the way.
fn watch_pulse(booted: &Harness, theme: &str) {
    let (mut faintest, mut strongest) = ([i32::MAX; 3], [i32::MIN; 3]);
    pump_until(
        &format!("the skeleton visibly pulses on {theme}"),
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let Some((skeleton, card)) = sample() else {
                return false;
            };
            assert!(
                spread(skeleton, card) >= VISIBLE_LEVELS,
                "on {theme} the skeleton {skeleton:?} is not visible on its card {card:?}"
            );
            for channel in 0..3 {
                faintest[channel] = faintest[channel].min(skeleton[channel]);
                strongest[channel] = strongest[channel].max(skeleton[channel]);
            }
            spread(faintest, strongest) >= VISIBLE_LEVELS
        },
    );
}

#[test]
fn a_pending_hero_icon_shows_a_visibly_pulsing_skeleton_on_dark_and_light() {
    // Given: a window on Nord whose first weather refresh never answers.
    let root = tempfile::tempdir().expect("a temporary theme root");
    write_theme(root.path(), NORD, 1);
    let mut booted = HarnessBuilder::with_feed(Arc::new(SilentFeedApi))
        .theme_path(root.path().to_owned())
        .boot();

    // When / Then: the hero skeleton stands out from its card and breathes.
    watch_pulse(&booted, "Nord");

    // When: the desktop switches to a light theme.
    write_theme(root.path(), FLEXOKI_LIGHT, 2);
    booted.poll_theme();
    pump_until("the card turns light", SETTLE_BUDGET, || {
        booted.process_events();
        sample().is_some_and(|(_, card)| card.iter().all(|level| *level > 200))
    });

    // Then: the same holds on the light card.
    watch_pulse(&booted, "Flexoki Light");
}
