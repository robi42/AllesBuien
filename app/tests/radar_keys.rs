//! The radar's playback answers the keyboard, once per press, wherever the focus is.
//!
//! The hazard is the focused control. A Button clicks on Space, Return, and
//! Enter by itself, and a Slider steps on Left and Right, so a window-wide key
//! scheme can act twice or never depending on where the focus sits. Every key
//! here goes in through the window system, as a keyboard's does, and every
//! assertion reads the radar controller; `radar_transport` says how.
//!
//! The focus ring answers the keyboard and nothing else, and it has to survive
//! the radar refresh that disables both controls every few minutes, so the
//! refreshes here happen under keyboard focus and under a click's.

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
use allesbuien_harness::{HarnessBuilder, keys::Key, probe::focused_window};
use allesbuien_testkit::{Rgb, SETTLE_BUDGET, StubFeedApi, VISIBLE_LEVELS, pump_until, spread};
use radar_transport::{Control, Transport, refresh_to};

mod radar_transport;

/// Omarchy's Nord, so the ring is checked against a theme's own colours.
const NORD: &str = "mode = \"dark\"\nbackground = \"#2e3440\"\nforeground = \"#d8dee9\"\n\
    accent = \"#81a1c1\"\nselection = \"#434c5e\"\nlighter_background = \"#3b4252\"\n\
    muted = \"#4c566a\"\n";

/// Nord's foreground, which `Theme.focusRing` resolves to.
const NORD_FOREGROUND: Rgb = [0xd8, 0xde, 0xe9];

/// Runs of three frames for the first four fetches, then a run of one.
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
            0..=3 => run_metadata("202608300900", 3),
            _ => run_metadata("202608300910", 1),
        };
        Box::pin(async move { Ok(body) })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async { Ok(allesbuien::BASEMAP_PNG.to_vec()) })
    }
}

/// Whether `control` shows its focus ring: whether the edge where both
/// controls draw one is the theme's foreground. Without a ring that edge is the
/// button's fill or the handle's surface-coloured gap, and asserting `false` in
/// that state is what proves those differ visibly from the ring.
fn ringed(transport: &Transport<'_>, control: Control) -> bool {
    spread(transport.left_edge(control), NORD_FOREGROUND) < VISIBLE_LEVELS
}

/// Space, Return, Enter, and the keypad's Enter each toggle exactly once, with
/// nothing focused and with the play button focused. A double toggle would
/// leave each of these reading what it read before the press. The focused
/// button would toggle once on its own, so the unfocused presses here and the
/// slider's in [`steps_and_holds`] are what prove the Shortcut is listening.
fn toggles_once(transport: &Transport<'_>) {
    assert_eq!(transport.playback(), (true, 0), "a run of three plays");
    let unfocused = "toggles once with nothing focused";
    transport.press_all(&[
        (Key::Space, (false, 0), unfocused),
        (Key::Return, (true, 0), unfocused),
        (Key::Enter, (false, 0), unfocused),
        (Key::KeypadEnter, (true, 0), unfocused),
        (Key::Backtab, (true, 0), "moves the focus and nothing else"),
    ]);
    assert!(
        transport.flag(Control::Slider, "activeFocus"),
        "Shift+Tab starts at the end"
    );
    assert!(ringed(transport, Control::Handle), "and rings the slider");
    transport.press(Key::Tab);
    assert!(
        transport.flag(Control::PlayButton, "activeFocus"),
        "Tab wraps to the start"
    );
    let once = "toggles once on the focused button";
    transport.press_all(&[
        (Key::Space, (false, 0), once),
        (Key::Return, (true, 0), once),
        (Key::Enter, (false, 0), once),
        (Key::KeypadEnter, (true, 0), once),
    ]);
}

/// With the slider focused, the toggle still toggles once, and the arrows,
/// Home, and End step, pause, and stop at both ends without wrapping. Then a
/// held Space toggles once while a held arrow keeps stepping.
fn steps_and_holds(transport: &Transport<'_>) {
    transport.press(Key::Tab);
    assert!(transport.flag(Control::Slider, "activeFocus"));
    let once = "toggles once on the focused slider";
    transport.press_all(&[
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
    ]);
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
/// when the focus moves on. Starts with the slider focused by Shift+Tab, and
/// leaves it focused by Tab.
fn rings_follow_the_focus(transport: &Transport<'_>) {
    assert!(
        ringed(transport, Control::Handle),
        "Shift+Tab rings the slider"
    );
    assert!(
        !ringed(transport, Control::PlayButton),
        "only one ring at a time"
    );
    transport.press(Key::Tab);
    assert!(
        ringed(transport, Control::PlayButton),
        "Tab rings the button"
    );
    assert!(
        !ringed(transport, Control::Handle),
        "the slider's ring left with Tab"
    );
    transport.press(Key::Tab);
    assert!(
        !ringed(transport, Control::PlayButton),
        "the button's ring left"
    );
    assert!(
        ringed(transport, Control::Handle),
        "the slider's ring came back"
    );
}

/// After a refresh under the keyboard's focus, the same control has the focus
/// and the ring again, and the keys carry on from it. Leaves the play button
/// focused by Tab.
fn a_refresh_keeps_the_slider_ring(transport: &Transport<'_>) {
    assert_eq!(transport.playback(), (true, 0), "a refreshed run plays");
    assert!(
        transport.flag(Control::Slider, "activeFocus"),
        "the focus came back"
    );
    assert!(
        ringed(transport, Control::Handle),
        "the ring came back with it"
    );
    transport.press_all(&[
        (Key::Space, (false, 0), "toggles once after a refresh"),
        (Key::Right, (false, 1), "steps after a refresh"),
        (Key::Tab, (false, 1), "moves on from where the focus was"),
    ]);
    assert!(
        ringed(transport, Control::PlayButton),
        "Tab rings the button"
    );
}

/// The play button keeps its ring across a refresh too. Focus that a click
/// moves onto it is the mouse's, though: the click toggles once, like a key,
/// and neither control shows a ring. A click on a control that already has the
/// focus moves nothing, so it keeps whatever ring it had, as Qt's own
/// `visualFocus` does.
fn a_refresh_keeps_the_button_ring(transport: &Transport<'_>) {
    let button = Control::PlayButton;
    assert!(transport.flag(button, "activeFocus"), "the focus came back");
    assert!(ringed(transport, button), "the ring came back with it");
    transport.press(Key::Tab);
    assert!(ringed(transport, Control::Handle), "Tab rings the slider");
    assert_eq!(transport.click(button), (false, 0), "a click toggles once");
    assert!(
        transport.flag(button, "activeFocus"),
        "the click took the focus"
    );
    assert!(
        !ringed(transport, button),
        "focus from a click shows no ring"
    );
    assert!(
        !ringed(transport, Control::Handle),
        "nor the control it left"
    );
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
    transport.press_all(&[
        (Key::Space, (false, 0), inert),
        (Key::KeypadEnter, (false, 0), inert),
        (Key::Right, (false, 0), inert),
        (Key::End, (false, 0), inert),
        (Key::Tab, (false, 0), inert),
    ]);
    assert!(!transport.flag(Control::PlayButton, "activeFocus"));
    assert!(!transport.flag(Control::Slider, "activeFocus"));
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

    // When / Then: refreshes come and go under the keyboard's focus.
    refresh_to(&mut booted, 3);
    a_refresh_keeps_the_slider_ring(&Transport::find(&booted));
    refresh_to(&mut booted, 3);
    a_refresh_keeps_the_button_ring(&Transport::find(&booted));

    // When / Then: a refresh under a click's focus brings no ring back.
    refresh_to(&mut booted, 3);
    let transport = Transport::find(&booted);
    assert!(
        transport.flag(Control::PlayButton, "activeFocus"),
        "the focus came back"
    );
    assert!(
        !ringed(&transport, Control::PlayButton),
        "and still shows no ring"
    );

    // When / Then: a run of one frame leaves every key with nothing to do.
    refresh_to(&mut booted, 1);
    one_frame_is_inert(&Transport::find(&booted));
}
