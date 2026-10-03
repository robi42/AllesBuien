//! The booted window's radar transport, as a keyboard and a viewer meet it.
//!
//! `radar_keys` asserts what each key does to playback and what the focus ring
//! looks like. This is the half that reaches the window to find out: it types
//! through [`allesbuien_harness::keys`], reads the outcome back off the radar
//! controller rather than the QML that called it, and samples rendered pixels
//! for the ring, because a ring is only proven by being seen.
//!
//! It lives beside its one consumer rather than in `allesbuien-harness`: a
//! second harness that needs the play button and the slider by name is the
//! point at which this earns a place there.

use std::ptr::NonNull;

use allesbuien_harness::{
    Harness,
    keys::{Key, key_click, key_hold},
    probe::{
        QObject, QQuickWindow, child, focused_window, grab_window, map_to_scene, number_of,
        property,
    },
};

pub(crate) type Rgb = [i32; 3];

/// The largest per-channel difference between two colours.
pub(crate) fn spread(first: Rgb, second: Rgb) -> i32 {
    (0..3)
        .map(|channel| (first[channel] - second[channel]).abs())
        .max()
        .unwrap_or(0)
}

/// The three items of the transport a test reads.
#[derive(Clone, Copy)]
pub(crate) enum Control {
    PlayButton,
    Slider,
    /// The slider's handle, which is where the slider draws its ring.
    Handle,
}

/// The booted window, the two controls the keys act on, and the app behind them.
///
/// Every pointer came from the probes on `booted`, which this borrows, so none
/// can outlive the application; the harness runs on the one thread that booted
/// it. That is the whole of the probes' safety contract, discharged once here.
pub(crate) struct Transport<'a> {
    booted: &'a Harness,
    window: NonNull<QQuickWindow>,
    play_button: NonNull<QObject>,
    slider: NonNull<QObject>,
    handle: NonNull<QObject>,
}

impl<'a> Transport<'a> {
    /// Finds the focused window's play button, slider, and slider handle.
    ///
    /// # Panics
    ///
    /// When no window has focus or one of the three is missing from its tree.
    pub(crate) fn find(booted: &'a Harness) -> Self {
        // SAFETY: on the GUI thread of the live booted app; every item is looked
        // up under the window that `focused_window` returned.
        unsafe {
            let window = focused_window().expect("the root window should be focused");
            let item = |name| child(window, name).unwrap_or_else(|| panic!("{name} should exist"));
            Self {
                booted,
                window,
                play_button: item("playButton"),
                slider: item("frameSlider"),
                handle: item("sliderHandle"),
            }
        }
    }

    /// Whether the radar is playing, and which frame it is on.
    pub(crate) fn playback(&self) -> (bool, i32) {
        let controller = self
            .booted
            .radar_controller()
            .expect("the radar controller should exist");
        (*controller.playing(), *controller.current_frame_index())
    }

    /// Presses `key` once, then reports the playback it left.
    pub(crate) fn press(&self, key: Key) -> (bool, i32) {
        // SAFETY: `window` came from the probes on the still-borrowed `booted`.
        unsafe { key_click(self.window, key) };
        self.booted.process_events();
        self.playback()
    }

    /// Holds `key` for `repeats` auto-repeats, then reports the playback it left.
    pub(crate) fn hold(&self, key: Key, repeats: u16) -> (bool, i32) {
        // SAFETY: as in `press`.
        unsafe { key_hold(self.window, key, repeats) };
        self.booted.process_events();
        self.playback()
    }

    /// A boolean property of `control`, such as `activeFocus` or `visualFocus`.
    pub(crate) fn flag(&self, control: Control, name: &str) -> bool {
        // SAFETY: the item came from the probes on the still-borrowed `booted`.
        unsafe { property(self.item(control), name) }
            .value::<bool>()
            .unwrap_or_else(|| panic!("{name} should be a bool"))
    }

    /// The rendered pixel one unit inside `control`'s left edge, half way
    /// down, where both controls draw their ring.
    pub(crate) fn left_edge(&self, control: Control) -> Rgb {
        let item = self.item(control);
        // SAFETY: `window` and `item` came from the probes on the still-borrowed
        // `booted`, and all three are `QQuickItem`s.
        unsafe {
            let image = grab_window(self.window);
            // The scene is in logical units and the grab in physical pixels, so
            // a scaled desktop needs the ratio between them; the window's own
            // width measures it without trusting the grab's pixel ratio.
            let ratio = f64::from(image.width()) / number_of(self.window.cast(), "width");
            let middle = number_of(item, "height") / 2.0;
            let point = (map_to_scene(item, 1.0, middle) * ratio).to_point();
            let color = image.pixel_color(point.x(), point.y());
            [color.red(), color.green(), color.blue()]
        }
    }

    const fn item(&self, control: Control) -> NonNull<QObject> {
        match control {
            Control::PlayButton => self.play_button,
            Control::Slider => self.slider,
            Control::Handle => self.handle,
        }
    }
}
