//! Whether rain is worth interrupting someone for, and how it is announced.
//!
//! A weather app that pops a notification on every refresh is a weather app
//! people mute. Two rules keep this one quiet: rain further out than the
//! configured threshold is not news yet, and the same rain re-reported minute
//! after minute is not news again. `last_event_key` is what separates those —
//! a cooldown on its own would also swallow a genuinely new front that arrives
//! inside it.
//!
//! [`Notifier`] is the process boundary. `notify-send` is a subprocess, so it is
//! the one thing here a test must not actually do; [`FakeNotifier`] records the
//! calls instead, and it is what the wiring tests assert against.

use std::cell::RefCell;
use std::process::Command;
use std::time::{Duration, Instant};

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
pub struct FakeNotifier {
    calls: RefCell<Vec<(String, String)>>,
}

impl FakeNotifier {
    #[must_use]
    pub fn calls(&self) -> Vec<(String, String)> {
        self.calls.borrow().clone()
    }
}

impl Notifier for FakeNotifier {
    fn send(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        self.calls
            .borrow_mut()
            .push((title.to_owned(), body.to_owned()));
        Ok(())
    }
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
pub struct RainSoon<'a> {
    pub minutes_until: u32,
    pub time_label: &'a str,
    pub max_mmh: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct NotificationInput<'a> {
    pub rain: Option<&'a RainSoon<'a>>,
    pub location_label: &'a str,
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
    pub cooldown: Duration,
}

#[derive(Debug)]
pub struct NotifyState {
    pub config: NotifyConfig,
    pub last_sent_at: Option<Instant>,
    pub last_event_key: Option<String>,
}

#[must_use]
pub fn should_notify(rain: Option<&RainSoon<'_>>, notify_state: &NotifyState) -> Option<String> {
    if !notify_state.config.enabled {
        return None;
    }

    let rain = match rain {
        Some(rain) if rain.minutes_until <= notify_state.config.threshold_minutes => rain,
        Some(_) | None => return None,
    };

    let event_key = format!("{}-{:.0}", rain.time_label, (rain.max_mmh * 100.0).round());
    if let Some(last_at) = notify_state.last_sent_at {
        let same_event = notify_state.last_event_key.as_ref() == Some(&event_key);
        if same_event && last_at.elapsed() < notify_state.config.cooldown {
            return None;
        }
    }

    Some(event_key)
}

/// Announces the coming rain if the rules allow it, and records that it went out.
///
/// Returns `Ok(())` for the withheld cases too — no rain, rain beyond the
/// threshold, and a repeat inside the cooldown are all normal outcomes rather
/// than failures, and only a dispatch that actually happened updates the state.
///
/// # Errors
///
/// Propagates the [`Notifier`]'s [`NotifyError`] when a dispatch was due and
/// the adapter refused it. The cooldown and event key stay untouched in that
/// case, so the next refresh retries the same announcement.
pub fn maybe_send_rain_notification(
    notifier: &impl Notifier,
    input: NotificationInput<'_>,
    notify_state: &mut NotifyState,
) -> Result<(), NotifyError> {
    let Some(rain) = input.rain else {
        return Ok(());
    };
    let Some(event_key) = should_notify(Some(rain), notify_state) else {
        return Ok(());
    };

    let title = format!("Rain soon ({})", input.location_label);
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
    notifier.send(&title, &body)?;
    notify_state.last_sent_at = Some(Instant::now());
    notify_state.last_event_key = Some(event_key);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FakeNotifier, NotificationInput, NotifyConfig, NotifyState, RainSoon,
        maybe_send_rain_notification, should_notify,
    };
    use std::time::{Duration, Instant};

    fn base_state() -> NotifyState {
        NotifyState {
            config: NotifyConfig {
                enabled: true,
                threshold_minutes: 30,
                cooldown: Duration::from_secs(3600),
            },
            last_sent_at: None,
            last_event_key: None,
        }
    }

    fn rain(minutes_until: u32, time_label: &str, max_mmh: f64) -> RainSoon<'_> {
        RainSoon {
            minutes_until,
            time_label,
            max_mmh,
        }
    }

    /// The title and body `rain` produces on a fresh state.
    ///
    /// The wording has no accessor of its own — a dispatch is the only place it
    /// is observable — so every wording test has to arrange one.
    fn dispatched(rain: &RainSoon<'_>) -> (String, String) {
        let notifier = FakeNotifier::default();
        let mut state = base_state();
        let input = NotificationInput {
            rain: Some(rain),
            location_label: "Home",
        };

        maybe_send_rain_notification(&notifier, input, &mut state).expect("fake send succeeds");

        notifier
            .calls()
            .first()
            .cloned()
            .expect("the announcement went out")
    }

    #[test]
    fn imminent_rain_is_still_withheld_when_notifications_are_switched_off() {
        // Given
        let mut state = base_state();
        state.config.enabled = false;

        // When / Then
        assert!(should_notify(Some(&rain(10, "12:10", 0.8)), &state).is_none());
    }

    #[test]
    fn rain_further_out_than_the_threshold_is_not_announced_yet() {
        // Given
        let state = base_state();

        // When / Then
        assert!(should_notify(Some(&rain(45, "12:45", 0.8)), &state).is_none());
    }

    #[test]
    fn the_same_event_inside_the_cooldown_is_not_announced_twice() {
        // Given
        let mut state = base_state();
        state.last_event_key = Some(String::from("12:10-80"));
        state.last_sent_at = Some(Instant::now());

        // When / Then
        assert!(should_notify(Some(&rain(10, "12:10", 0.8)), &state).is_none());
    }

    #[test]
    fn a_different_event_is_announced_even_inside_the_cooldown() {
        // Given
        let mut state = base_state();
        state.last_event_key = Some(String::from("12:10-80"));
        state.last_sent_at = Some(Instant::now());

        // When / Then
        assert_eq!(
            should_notify(Some(&rain(10, "12:15", 0.8)), &state),
            Some(String::from("12:15-80"))
        );
    }

    #[test]
    fn event_change_dispatches_once_and_same_event_does_not_dispatch_again() {
        // Given
        let notifier = FakeNotifier::default();
        let mut state = base_state();
        state.last_event_key = Some(String::from("12:10-80"));
        state.last_sent_at = Some(Instant::now());
        let changed_rain = rain(10, "12:15", 0.8);
        let input = NotificationInput {
            rain: Some(&changed_rain),
            location_label: "Home",
        };

        // When
        maybe_send_rain_notification(&notifier, input, &mut state).expect("fake send succeeds");
        maybe_send_rain_notification(&notifier, input, &mut state).expect("dedup succeeds");

        // Then
        assert_eq!(
            notifier.calls(),
            [(
                String::from("Rain soon (Home)"),
                String::from("Starts around 12:15 (10 min), peak ~0.80 mm/h")
            )]
        );
    }

    #[test]
    fn rain_that_has_already_started_is_announced_as_starting_now() {
        // Given
        let falling = rain(0, "12:05", 1.4);

        // When
        let (title, body) = dispatched(&falling);

        // Then
        assert_eq!(title, "Rain soon (Home)");
        assert_eq!(body, "Starts about now, peak ~1.40 mm/h");
    }

    #[test]
    fn rain_still_minutes_away_keeps_its_clock_time_and_its_countdown() {
        // Given
        let coming = rain(5, "12:10", 0.35);

        // When
        let (title, body) = dispatched(&coming);

        // Then
        assert_eq!(title, "Rain soon (Home)");
        assert_eq!(body, "Starts around 12:10 (5 min), peak ~0.35 mm/h");
    }
}
