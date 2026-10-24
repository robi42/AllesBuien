//! The announcement rules, driven refresh by refresh through a notifier that
//! records what it is handed instead of running `notify-send`.
//!
//! No test waits for a cooldown. One that should never pass is hours long and
//! one that has always passed is zero, which also shows that a refresh with
//! rain due stays quiet on the open spell alone.

use std::cell::RefCell;
use std::time::Duration;

use super::{
    NotificationInput, Notifier, NotifyConfig, NotifyError, NotifyState,
    maybe_send_rain_notification,
};
use crate::location::{LocationConfig, LocationSource};
use crate::rain::RainSoon;

/// A cooldown no test outlives.
const HOURS: Duration = Duration::from_hours(6);

/// Records every dispatch instead of spawning `notify-send`.
#[derive(Debug, Default)]
struct FakeNotifier {
    calls: RefCell<Vec<(String, String)>>,
}

impl Notifier for FakeNotifier {
    fn send(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        self.calls
            .borrow_mut()
            .push((title.to_owned(), body.to_owned()));
        Ok(())
    }
}

/// A desktop without `notify-send`, where every dispatch fails to start.
struct MissingNotifier;

impl Notifier for MissingNotifier {
    fn send(&self, _title: &str, _body: &str) -> Result<(), NotifyError> {
        Err(NotifyError::Spawn(std::io::ErrorKind::NotFound.into()))
    }
}

/// One running app's notifications: the state its refreshes share, and what
/// they announced.
struct Viewer {
    notifier: FakeNotifier,
    state: NotifyState,
}

impl Viewer {
    fn with_cooldown(cooldown: Duration) -> Self {
        Self::configured(NotifyConfig {
            enabled: true,
            threshold_minutes: 30,
            cooldown,
        })
    }

    fn configured(config: NotifyConfig) -> Self {
        Self {
            notifier: FakeNotifier::default(),
            state: NotifyState::new(config),
        }
    }

    /// Refreshes `location` once for each forecast, in order, the way the app
    /// hands each refresh to the rules.
    fn refresh(&mut self, location: &LocationConfig, forecasts: &[Option<RainSoon>]) {
        for forecast in forecasts {
            let input = NotificationInput {
                rain: forecast.as_ref(),
                location,
            };
            maybe_send_rain_notification(&self.notifier, input, &mut self.state)
                .expect("fake send succeeds");
        }
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.notifier.calls.borrow().clone()
    }

    fn titles(&self) -> Vec<String> {
        self.calls().into_iter().map(|(title, _)| title).collect()
    }

    fn bodies(&self) -> Vec<String> {
        self.calls().into_iter().map(|(_, body)| body).collect()
    }
}

fn place(label: &str, latitude: f64, longitude: f64) -> LocationConfig {
    LocationConfig {
        latitude,
        longitude,
        label: label.to_owned(),
        source: LocationSource::DesktopSetting,
    }
}

fn home() -> LocationConfig {
    place("Home", 52.37, 4.84)
}

/// A forecast whose window holds rain; `None` stands for a dry one.
#[expect(
    clippy::unnecessary_wraps,
    reason = "it builds a forecast, and a forecast is an Option whose None is a dry window"
)]
fn rain(minutes_until: u32, time_label: &str, max_mmh: f64) -> Option<RainSoon> {
    Some(RainSoon {
        minutes_until,
        time_label: time_label.to_owned(),
        max_mmh,
    })
}

/// The title and body `rain` produces on a fresh state, read off a dispatch:
/// the one place a viewer ever meets the wording.
fn dispatched(rain: Option<RainSoon>) -> (String, String) {
    let mut viewer = Viewer::with_cooldown(HOURS);
    viewer.refresh(&home(), &[rain]);
    viewer
        .calls()
        .first()
        .cloned()
        .expect("the announcement went out")
}

#[test]
fn imminent_rain_is_never_announced_while_notifications_are_switched_off() {
    // Given: no cooldown either, so nothing but the switch can keep them quiet.
    let mut viewer = Viewer::configured(NotifyConfig {
        enabled: false,
        threshold_minutes: 30,
        cooldown: Duration::ZERO,
    });
    let forecasts = [rain(10, "12:10", 0.8), None, rain(0, "12:20", 1.4)];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert!(viewer.calls().is_empty());
}

#[test]
fn rain_further_out_than_the_threshold_waits_until_it_comes_within_it() {
    // Given: the same rain ten minutes nearer at each refresh, the last exactly
    // at the threshold, which already counts as within it.
    let mut viewer = Viewer::with_cooldown(HOURS);
    let forecasts = [
        rain(50, "12:50", 0.8),
        rain(40, "12:50", 0.8),
        rain(30, "12:50", 0.8),
    ];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert_eq!(
        viewer.bodies(),
        ["Starts around 12:50 (30 min), peak ~0.80 mm/h"]
    );
}

#[test]
fn falling_rain_is_announced_once_however_far_the_window_moves_on() {
    // Given: rain at every refresh, and so a first wet row ten minutes later
    // each time.
    let mut viewer = Viewer::with_cooldown(Duration::ZERO);
    let forecasts = [
        rain(0, "12:00", 1.2),
        rain(0, "12:10", 1.6),
        rain(0, "12:20", 0.9),
        rain(0, "12:30", 0.4),
    ];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert_eq!(viewer.bodies(), ["Starts about now, peak ~1.20 mm/h"]);
}

#[test]
fn a_revised_start_or_peak_is_not_announced_as_new_rain() {
    // Given: the start moves later and back, and the peak up, down, and up.
    let mut viewer = Viewer::with_cooldown(Duration::ZERO);
    let forecasts = [
        rain(25, "12:25", 0.8),
        rain(20, "12:30", 1.3),
        rain(5, "12:25", 0.6),
        rain(0, "12:30", 2.1),
    ];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert_eq!(
        viewer.bodies(),
        ["Starts around 12:25 (25 min), peak ~0.80 mm/h"]
    );
}

#[test]
fn a_dry_window_ends_the_spell_so_the_next_rain_is_announced() {
    // Given: a cooldown that never passes, so only the dry window can close it.
    let mut viewer = Viewer::with_cooldown(HOURS);
    let forecasts = [rain(10, "12:10", 0.8), None, rain(20, "12:40", 1.5)];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert_eq!(
        viewer.bodies(),
        [
            "Starts around 12:10 (10 min), peak ~0.80 mm/h",
            "Starts around 12:40 (20 min), peak ~1.50 mm/h",
        ]
    );
}

#[test]
fn rain_that_slips_past_the_threshold_and_back_inside_the_cooldown_is_not_announced_again() {
    // Given
    let mut viewer = Viewer::with_cooldown(HOURS);
    let forecasts = [
        rain(25, "12:25", 0.8),
        rain(35, "12:45", 0.8),
        rain(25, "12:45", 0.9),
    ];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert_eq!(
        viewer.bodies(),
        ["Starts around 12:25 (25 min), peak ~0.80 mm/h"]
    );
}

#[test]
fn rain_out_of_reach_once_the_cooldown_has_passed_ends_the_spell() {
    // Given: the forecasts of the test above, with a cooldown already over.
    let mut viewer = Viewer::with_cooldown(Duration::ZERO);
    let forecasts = [
        rain(25, "12:25", 0.8),
        rain(35, "12:45", 0.8),
        rain(25, "12:45", 0.9),
    ];

    // When
    viewer.refresh(&home(), &forecasts);

    // Then
    assert_eq!(
        viewer.bodies(),
        [
            "Starts around 12:25 (25 min), peak ~0.80 mm/h",
            "Starts around 12:45 (25 min), peak ~0.90 mm/h",
        ]
    );
}

#[test]
fn a_dispatch_that_failed_is_retried_by_the_next_refresh() {
    // Given
    let mut viewer = Viewer::with_cooldown(HOURS);
    let due = rain(10, "12:10", 0.8);
    let location = home();
    let input = NotificationInput {
        rain: due.as_ref(),
        location: &location,
    };

    // When
    let failed = maybe_send_rain_notification(&MissingNotifier, input, &mut viewer.state);
    viewer.refresh(&location, &[rain(0, "12:10", 0.8)]);

    // Then
    assert!(matches!(failed, Err(NotifyError::Spawn(_))));
    assert_eq!(viewer.bodies(), ["Starts about now, peak ~0.80 mm/h"]);
}

#[test]
fn a_spell_belongs_to_the_coordinates_rather_than_to_the_name() {
    // Given: the same coordinates under a new name, and a place of its own.
    let mut viewer = Viewer::with_cooldown(HOURS);
    let renamed = place("Thuis", 52.37, 4.84);
    let elsewhere = place("Utrecht", 52.09, 5.12);

    // When
    viewer.refresh(&home(), &[rain(10, "12:10", 0.8)]);
    viewer.refresh(&renamed, &[rain(0, "12:20", 1.2)]);
    viewer.refresh(&elsewhere, &[rain(5, "12:25", 1.1)]);

    // Then
    assert_eq!(viewer.titles(), ["Rain soon (Home)", "Rain soon (Utrecht)"]);
}

#[test]
fn a_refresh_elsewhere_leaves_no_spell_open_for_the_place_left_behind() {
    // Given: a cooldown that never passes, so only the refresh elsewhere can
    // have closed the spell.
    let mut viewer = Viewer::with_cooldown(HOURS);
    let elsewhere = place("Utrecht", 52.09, 5.12);

    // When
    viewer.refresh(&home(), &[rain(10, "12:10", 0.8)]);
    viewer.refresh(&elsewhere, &[None]);
    viewer.refresh(&home(), &[rain(0, "12:20", 1.4)]);

    // Then
    assert_eq!(
        viewer.bodies(),
        [
            "Starts around 12:10 (10 min), peak ~0.80 mm/h",
            "Starts about now, peak ~1.40 mm/h",
        ]
    );
}

#[test]
fn rain_that_has_already_started_is_announced_as_starting_now() {
    // Given
    let falling = rain(0, "12:05", 1.4);

    // When
    let (title, body) = dispatched(falling);

    // Then
    assert_eq!(title, "Rain soon (Home)");
    assert_eq!(body, "Starts about now, peak ~1.40 mm/h");
}

#[test]
fn rain_still_minutes_away_keeps_its_clock_time_and_its_countdown() {
    // Given
    let coming = rain(5, "12:10", 0.35);

    // When
    let (title, body) = dispatched(coming);

    // Then
    assert_eq!(title, "Rain soon (Home)");
    assert_eq!(body, "Starts around 12:10 (5 min), peak ~0.35 mm/h");
}
