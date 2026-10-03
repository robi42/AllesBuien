//! The radar's playback answers the keyboard, once per press, wherever the focus is.
//!
//! The hazard is the focused control. A Button clicks on Space, Return, and
//! Enter by itself, and a Slider steps on Left and Right, so a window-wide key
//! scheme can act twice or never depending on where the focus sits. Every key
//! here goes in through the window system, as a keyboard's does, and every
//! assertion reads the radar controller; `radar_transport` says how.

use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use allesbuien::TimerIntervals;
use allesbuien_core::radar::{RadarApi, RadarFuture};
use allesbuien_harness::{Harness, HarnessBuilder, keys::Key, probe::focused_window};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until};
use radar_transport::{Control, Rgb, Transport, spread};

mod radar_transport;

/// Omarchy's Nord, so the ring is checked against a theme's own colours.
const NORD: &str = "mode = \"dark\"\nbackground = \"#2e3440\"\nforeground = \"#d8dee9\"\n\
    accent = \"#81a1c1\"\nselection = \"#434c5e\"\nlighter_background = \"#3b4252\"\n\
    muted = \"#4c566a\"\n";

/// Nord's foreground, which `Theme.focusRing` resolves to.
const NORD_FOREGROUND: Rgb = [0xd8, 0xde, 0xe9];

/// The per-channel difference a reader can actually see, as `ForecastView`
/// measured for its row rules.
const VISIBLE_LEVELS: i32 = 12;

/// A key, the playback it should leave behind, and what that proves.
type Step = (Key, (bool, i32), &'static str);

/// A three-frame run on the first two fetches, then a run of one frame.
#[derive(Default)]
struct ScriptedRunRadarApi {
    metadata_calls: AtomicUsize,
}

/// A run stamped `run` holding `count` frames five minutes apart.
fn run_metadata(run: &str, count: usize) -> Vec<u8> {
    let times = (0..count)
        .map(|index| {
            let minute = 5 * index;
            format!(
                r#"{{"timestamp":"2026-08-30T09:{minute:02}:00","url":"fake://{run}/{index}"}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"timestamp":"{run}","timeOffset":0,"times":[{times}]}}"#).into_bytes()
}

impl RadarApi for ScriptedRunRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        let body = match self.metadata_calls.fetch_add(1, Ordering::SeqCst) {
            0 | 1 => run_metadata("202608300900", 3),
            _ => run_metadata("202608300910", 1),
        };
        Box::pin(async move { Ok(body) })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async { Ok(allesbuien::BASEMAP_PNG.to_vec()) })
    }
}

/// Presses each key in turn and asserts the playback it leaves behind.
fn press_all(transport: &Transport<'_>, steps: &[Step]) {
    for &(key, expected, meaning) in steps {
        assert_eq!(transport.press(key), expected, "{key:?}: {meaning}");
    }
}

/// Space, Return, Enter, and the keypad's Enter each toggle exactly once, with
/// nothing focused and with the play button focused. A double toggle would
/// leave each of these reading what it read before the press. The focused
/// button would toggle once on its own, so the unfocused presses here and the
/// slider's in [`steps_and_holds`] are what prove the Shortcut is listening.
fn toggles_once(transport: &Transport<'_>) {
    assert_eq!(transport.playback(), (true, 0), "a run of three plays");
    let unfocused = "toggles once with nothing focused";
    press_all(
        transport,
        &[
            (Key::Space, (false, 0), unfocused),
            (Key::Return, (true, 0), unfocused),
            (Key::Enter, (false, 0), unfocused),
            (Key::KeypadEnter, (true, 0), unfocused),
            (Key::Tab, (true, 0), "moves the focus and nothing else"),
        ],
    );
    assert!(transport.flag(Control::PlayButton, "activeFocus"));
    let once = "toggles once on the focused button";
    press_all(
        transport,
        &[
            (Key::Space, (false, 0), once),
            (Key::Return, (true, 0), once),
            (Key::Enter, (false, 0), once),
            (Key::KeypadEnter, (true, 0), once),
        ],
    );
}

/// With the slider focused, the toggle still toggles once, and the arrows,
/// Home, and End step, pause, and stop at both ends without wrapping. Then a
/// held Space toggles once while a held arrow keeps stepping.
fn steps_and_holds(transport: &Transport<'_>) {
    transport.press(Key::Tab);
    assert!(transport.flag(Control::Slider, "activeFocus"));
    let once = "toggles once on the focused slider";
    press_all(
        transport,
        &[
            (Key::Space, (false, 0), once),
            (Key::Return, (true, 0), once),
            (Key::KeypadEnter, (false, 0), once),
            (Key::Enter, (true, 0), once),
            (Key::Right, (false, 1), "steps forward and pauses"),
            (Key::Right, (false, 2), "steps onto the latest frame"),
            (Key::Right, (false, 2), "stops at the latest frame"),
            (Key::Left, (false, 1), "steps back"),
            (Key::Home, (false, 0), "jumps to the first frame"),
            (Key::Left, (false, 0), "stops at the first frame"),
            (Key::Space, (true, 0), "resumes"),
            (Key::End, (false, 2), "jumps to the latest frame and pauses"),
            (Key::Tab, (false, 2), "wraps the focus to the play button"),
        ],
    );
    assert!(transport.flag(Control::PlayButton, "activeFocus"));
    transport.press(Key::Backtab);
    assert!(transport.flag(Control::Slider, "activeFocus"));

    transport.press(Key::Home);
    let held = transport.hold(Key::Space, 3);
    assert_eq!(held, (true, 0), "a held Space toggles once");
    transport.press(Key::Space);
    let held = transport.hold(Key::Right, 1);
    assert_eq!(held, (false, 2), "a held Right steps on its repeat");
}

/// A Tab-focused control shows a ring in the theme's foreground, and loses it
/// when the focus moves on. Starts with the slider focused by Shift+Tab.
fn rings_follow_the_focus(transport: &Transport<'_>) {
    let (button, handle) = (Control::PlayButton, Control::Handle);
    assert!(transport.flag(Control::Slider, "visualFocus"));
    let (handle_ring, button_plain) = (transport.left_edge(handle), transport.left_edge(button));

    transport.press(Key::Tab);
    assert!(transport.flag(button, "visualFocus"));
    assert!(!transport.flag(Control::Slider, "visualFocus"));
    let (handle_plain, button_ring) = (transport.left_edge(handle), transport.left_edge(button));

    for (control, ring, plain) in [
        ("play button", button_ring, button_plain),
        ("slider handle", handle_ring, handle_plain),
    ] {
        assert!(
            spread(ring, plain) >= VISIBLE_LEVELS,
            "the {control}'s ring {ring:?} cannot be told from {plain:?}"
        );
        assert!(
            spread(ring, NORD_FOREGROUND) < VISIBLE_LEVELS,
            "the {control}'s ring {ring:?} is not the theme's foreground"
        );
    }

    transport.press(Key::Tab);
    assert!(!transport.flag(button, "visualFocus"));
    assert_eq!(
        transport.left_edge(button),
        button_plain,
        "the button's ring left"
    );
    assert_eq!(
        transport.left_edge(handle),
        handle_ring,
        "the handle's ring returned"
    );
}

/// A refresh empties the run while it fetches, which disables both controls.
/// Qt hands the focus back when they re-enable, though not as keyboard focus,
/// so the ring goes with the refresh, as `RadarTransport.qml` says, while the
/// focus and every key carry on. Starts after a refresh with the slider focused.
fn a_refresh_keeps_the_keys(transport: &Transport<'_>) {
    assert_eq!(transport.playback(), (true, 0), "a refreshed run plays");
    assert!(
        transport.flag(Control::Slider, "activeFocus"),
        "the focus came back"
    );
    assert!(
        !transport.flag(Control::Slider, "visualFocus"),
        "the ring did not"
    );
    press_all(
        transport,
        &[
            (Key::Space, (false, 0), "toggles once after a refresh"),
            (Key::Right, (false, 1), "steps after a refresh"),
            (Key::Tab, (false, 1), "moves on from where the focus was"),
        ],
    );
    assert!(transport.flag(Control::PlayButton, "activeFocus"));
}

/// On a run of one frame there is nothing to play or step through: every key
/// finds nothing to do, and neither disabled control takes the focus.
fn one_frame_is_inert(transport: &Transport<'_>) {
    assert_eq!(
        transport.playback(),
        (false, 0),
        "a run of one does not play"
    );
    let inert = "is inert on a run of one";
    press_all(
        transport,
        &[
            (Key::Space, (false, 0), inert),
            (Key::KeypadEnter, (false, 0), inert),
            (Key::Right, (false, 0), inert),
            (Key::End, (false, 0), inert),
            (Key::Tab, (false, 0), inert),
        ],
    );
    assert!(!transport.flag(Control::PlayButton, "activeFocus"));
    assert!(!transport.flag(Control::Slider, "activeFocus"));
}

/// Refreshes the radar and waits until the run it brings holds `frames` frames.
fn refresh_to(booted: &mut Harness, frames: i32) {
    booted.refresh_radar();
    pump_until(
        &format!("a run of {frames} is adopted"),
        SETTLE_BUDGET,
        || {
            booted.process_events();
            booted
                .radar_controller()
                .is_some_and(|controller| *controller.frame_count() == frames)
        },
    );
}

#[test]
fn radar_playback_answers_each_key_once_wherever_the_focus_is() {
    // Given: a window on Nord over a three-frame run whose playback Timer never
    // fires during the test, so only the keys move the frame.
    let theme = tempfile::tempdir().expect("a temporary theme root");
    fs::write(theme.path().join("colors.toml"), NORD).expect("the theme should be written");
    let intervals = TimerIntervals {
        radar_refresh: Duration::from_secs(300),
        radar_playback: Duration::from_secs(3600),
        ..TimerIntervals::default()
    };
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .radar(Arc::new(ScriptedRunRadarApi::default()))
        .theme_path(theme.path().to_owned())
        .intervals(intervals)
        .boot();
    pump_until(
        "a themed, focused window adopts the run",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let themed = booted
                .theme_controller()
                .is_some_and(|controller| *controller.theme_available());
            let adopted = booted
                .radar_controller()
                .is_some_and(|controller| *controller.frame_count() == 3);
            // SAFETY: on the GUI thread of the live booted app.
            themed && adopted && unsafe { focused_window() }.is_some()
        },
    );

    // When / Then: the keys drive playback, with and without focus.
    let transport = Transport::find(&booted);
    toggles_once(&transport);
    steps_and_holds(&transport);
    rings_follow_the_focus(&transport);

    // When / Then: a refresh comes and goes under the focused slider.
    refresh_to(&mut booted, 3);
    a_refresh_keeps_the_keys(&Transport::find(&booted));

    // When: the next refresh brings a run of one frame.
    refresh_to(&mut booted, 1);

    // Then: every key finds nothing to do, and nothing breaks.
    one_frame_is_inert(&Transport::find(&booted));
}
