use std::cell::RefCell;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("failed to run notify-send")]
    Spawn(#[source] std::io::Error),
}

pub trait Notifier {
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

#[derive(Clone, Copy, Debug)]
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
    let body = format!(
        "Starts around {} ({} min), peak ~{:.2} mm/h",
        rain.time_label, rain.minutes_until, rain.max_mmh
    );
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

    #[test]
    fn no_notify_when_disabled() {
        let mut state = base_state();
        state.config.enabled = false;
        assert!(should_notify(Some(&rain(10, "12:10", 0.8)), &state).is_none());
    }

    #[test]
    fn no_notify_when_outside_threshold() {
        let state = base_state();
        assert!(should_notify(Some(&rain(45, "12:45", 0.8)), &state).is_none());
    }

    #[test]
    fn no_notify_for_same_event_within_cooldown() {
        let mut state = base_state();
        state.last_event_key = Some(String::from("12:10-80"));
        state.last_sent_at = Some(Instant::now());
        assert!(should_notify(Some(&rain(10, "12:10", 0.8)), &state).is_none());
    }

    #[test]
    fn notify_when_event_changes() {
        let mut state = base_state();
        state.last_event_key = Some(String::from("12:10-80"));
        state.last_sent_at = Some(Instant::now());
        assert_eq!(
            should_notify(Some(&rain(10, "12:15", 0.8)), &state),
            Some(String::from("12:15-80"))
        );
    }

    #[test]
    fn event_change_dispatches_once_and_same_event_does_not_dispatch_again() {
        let notifier = FakeNotifier::default();
        let mut state = base_state();
        state.last_event_key = Some(String::from("12:10-80"));
        state.last_sent_at = Some(Instant::now());
        let changed_rain = rain(10, "12:15", 0.8);
        let input = NotificationInput {
            rain: Some(&changed_rain),
            location_label: "Home",
        };

        maybe_send_rain_notification(&notifier, input, &mut state).expect("fake send succeeds");
        maybe_send_rain_notification(&notifier, input, &mut state).expect("dedup succeeds");

        assert_eq!(
            notifier.calls(),
            [(
                String::from("Rain soon (Home)"),
                String::from("Starts around 12:15 (10 min), peak ~0.80 mm/h")
            )]
        );
    }
}
