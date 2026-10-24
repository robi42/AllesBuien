//! Whether rain is worth interrupting someone for, and how it is announced.
//!
//! A weather app that pops a notification on every refresh is a weather app
//! people mute. Two rules keep this one quiet: rain further out than the
//! configured threshold is not news yet, and rain already announced is not news
//! again. A refresh finds rain due when its [`RainSoon`] starts within the
//! threshold. The first refresh to find it due announces a spell of rain, and
//! the refreshes after it stay quiet while that spell is open, however far the
//! forecast moves its start or its peak.
//!
//! The forecast cannot tell one spell from the next, because it describes the
//! same rain differently on every refresh: once the rain has begun, its first
//! wet sample is the window's first row, whose clock time moves on each time,
//! and before then the start and the peak are revised as the rain approaches.
//! An announcement keyed on what the forecast says would go out again at every
//! refresh while the rain falls, and at every revision before it.
//!
//! A spell closes on a refresh that finds no rain due and either a dry window
//! or the cooldown passed since the announcement. A dry window means the rain
//! is over, so the next rain is news at once. Rain still in the window keeps
//! the spell open until the cooldown has passed, which is what stops a forecast
//! that slips just past the threshold and back from announcing the same rain
//! twice.
//!
//! A spell belongs to the place it was announced for. A refresh for any other
//! place finds none open, and the spell is dropped rather than kept for a
//! return: while no refresh is for its place nothing could close it, and it
//! would silence that place's rain however long the viewer stayed away.
//!
//! [`Notifier`] is the process boundary. `notify-send` is a subprocess, so it is
//! the one thing here a test must not actually do; the tests record the calls
//! through a fake instead.

use std::process::Command;
use std::time::{Duration, Instant};

use crate::location::LocationConfig;
use crate::rain::RainSoon;

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("failed to run notify-send")]
    Spawn(#[source] std::io::Error),
}

pub trait Notifier {
    /// Announces `title` and `body` to whatever the desktop uses for notices.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Spawn`] when the adapter cannot reach that
    /// surface at all. Delivery beyond the handover is not observable here, so
    /// a notification the user never sees still reports success.
    fn send(&self, title: &str, body: &str) -> Result<(), NotifyError>;
}

#[derive(Debug, Default)]
pub struct NotifySend;

impl Notifier for NotifySend {
    fn send(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        Command::new("notify-send")
            .arg("-a")
            .arg("allesbuien")
            .arg(title)
            .arg(body)
            .spawn()
            .map_err(NotifyError::Spawn)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NotificationInput<'a> {
    pub rain: Option<&'a RainSoon>,
    /// Where the forecast was fetched for: the title names it, and an open
    /// spell counts only for the place it was announced for.
    pub location: &'a LocationConfig,
}

/// The two rules above, as settled values.
///
/// It lives with the rules it governs rather than with the environment reading
/// that produces it: [`crate::config`] resolves one of these and carries it, so
/// a threshold cannot be read here and be spelled differently there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotifyConfig {
    pub enabled: bool,
    pub threshold_minutes: u32,
    /// The shortest time an announced spell stays open while rain is still in
    /// the forecast.
    pub cooldown: Duration,
}

/// The spell of rain announced and not yet closed, if there is one.
///
/// Its fields are private so that only a refresh, through
/// [`maybe_send_rain_notification`], can open or close a spell.
#[derive(Debug)]
pub struct NotifyState {
    config: NotifyConfig,
    open_spell: Option<Spell>,
}

impl NotifyState {
    /// Starts with no spell open, so the first refresh that finds rain due
    /// announces it.
    #[must_use]
    pub const fn new(config: NotifyConfig) -> Self {
        Self {
            config,
            open_spell: None,
        }
    }
}

/// A spell of rain that has been announced: where, and when.
#[derive(Debug)]
struct Spell {
    place: Place,
    announced_at: Instant,
}

/// The coordinates a forecast was fetched for, which are what make two
/// refreshes the same place.
///
/// The label is only what the place is called: two places can share one, and
/// renaming a place changes nothing about the rain falling on it. Both sides of
/// the comparison are the setting as it was read, never a computed value, so
/// exact equality is the right test.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    latitude: f64,
    longitude: f64,
}

impl Place {
    const fn of(location: &LocationConfig) -> Self {
        Self {
            latitude: location.latitude,
            longitude: location.longitude,
        }
    }
}

/// Announces rain that has come due, unless its spell has been announced
/// already, and opens or closes the spell as the rules above say.
///
/// Returns `Ok(())` for the quiet cases too — no rain, rain beyond the
/// threshold, and rain already announced are all normal outcomes rather than
/// failures.
///
/// # Errors
///
/// Propagates the [`Notifier`]'s [`NotifyError`] when an announcement was due
/// and the adapter refused it. Only a dispatch that happened opens a spell, so
/// none is open afterwards, and the next refresh that finds rain due retries.
pub fn maybe_send_rain_notification(
    notifier: &dyn Notifier,
    input: NotificationInput<'_>,
    notify_state: &mut NotifyState,
) -> Result<(), NotifyError> {
    let config = notify_state.config;
    if !config.enabled {
        return Ok(());
    }
    let place = Place::of(input.location);
    // Taken rather than borrowed: a spell announced for another place counts
    // as none here, and a dispatch that fails returns with none open.
    let open = notify_state
        .open_spell
        .take()
        .filter(|spell| spell.place == place);
    let due = input
        .rain
        .filter(|rain| rain.minutes_until <= config.threshold_minutes);
    notify_state.open_spell = match (due, open) {
        (Some(rain), None) => {
            let (title, body) = announcement(rain, input.location.display_label());
            notifier.send(&title, &body)?;
            Some(Spell {
                place,
                announced_at: Instant::now(),
            })
        }
        // Already announced, wherever the forecast has moved its start or peak.
        (Some(_), Some(spell)) => Some(spell),
        // Not due any more but still in the window: open until the cooldown passes.
        (None, Some(spell))
            if input.rain.is_some() && spell.announced_at.elapsed() < config.cooldown =>
        {
            Some(spell)
        }
        // A dry window, or rain still out of reach once the cooldown has passed.
        (None, _) => None,
    };
    Ok(())
}

/// The title and body that announce `rain` at the place shown as `location_label`.
fn announcement(rain: &RainSoon, location_label: &str) -> (String, String) {
    let title = format!("Rain soon ({location_label})");
    // Zero minutes means the series' first row is already wet, so this rain has
    // started rather than being due. Both the clock time and the count are
    // dropped there because "about now" already says each of them, and saying
    // them anyway produced "Starts around 12:05 (0 min)" — the bug restated.
    let body = if rain.minutes_until == 0 {
        format!("Starts about now, peak ~{:.2} mm/h", rain.max_mmh)
    } else {
        format!(
            "Starts around {} ({} min), peak ~{:.2} mm/h",
            rain.time_label, rain.minutes_until, rain.max_mmh
        )
    };
    (title, body)
}

#[cfg(test)]
mod tests;
