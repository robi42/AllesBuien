//! Everything the process has to settle before it can ask anyone anything.
//!
//! One [`Config`] is resolved at startup and injected from there on; nothing
//! reads the environment again. [`EnvSource`] is the seam that makes that
//! testable — a test hands it a fixed table instead of mutating the real
//! environment, which two tests sharing a process could not do safely.
//!
//! Overrides exist for the three endpoints and for the notification thresholds
//! so the app can be pointed at a local stub, and each one falls back to the
//! shipped value rather than to an empty string: an override that is present but
//! blank is a mistake, not an instruction. The viewer's location is the single
//! setting with a precedence rule of its own, and [`crate::location`] owns it.
//!
//! It is also the single setting that outlives this resolution. Omarchy rewrites
//! its half of it while the app runs, so `location` holds the re-askable
//! [`LocationSettings`] rather than one answer taken here and frozen.
//!
//! The shipped addresses live here rather than beside the transports that dial
//! them, because a default is a settling decision and this is the module that
//! settles it: it is the one place that has to name all three anyway. Held
//! apart, two of them drifted into a second spelling in a second file, and the
//! two copies could then disagree without anything failing.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::location_settings::LocationSettings;
use crate::notify::NotifyConfig;

/// Buienradar's published feed endpoint, and the fallback its override returns to.
pub const DEFAULT_FEED_URL: &str = "https://data.buienradar.nl/2.0/feed/json";
/// The raintext endpoint; [`crate::weather`] appends the viewer's coordinates.
pub const DEFAULT_RAINTEXT_URL: &str = "https://gpsgadget.buienradar.nl/data/raintext";
/// The radar host alone; [`crate::radar_transport`] appends the metadata path.
pub const DEFAULT_RADAR_URL: &str = "https://image.buienradar.nl";
const DEFAULT_NOTIFY_THRESHOLD_MINUTES: u32 = 30;
const DEFAULT_NOTIFY_COOLDOWN_MINUTES: u64 = 60;

pub trait EnvSource {
    fn var(&self, key: &str) -> Option<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StdEnv;

impl EnvSource for StdEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConfig {
    pub feed_url: String,
    pub raintext_url: String,
    pub radar_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub location: LocationSettings,
    pub endpoints: EndpointConfig,
    pub notify: NotifyConfig,
    pub state_dir: PathBuf,
}

impl Config {
    /// Resolves the configuration from the real process environment.
    #[must_use]
    pub fn load() -> Self {
        let env = StdEnv;
        let home = env
            .var("HOME")
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        Self::resolve(&env, &home)
    }

    /// Resolves the configuration from an arbitrary environment and home directory.
    #[must_use]
    pub fn resolve(env: &impl EnvSource, home: &Path) -> Self {
        Self {
            location: LocationSettings::new(env, home.to_owned()),
            endpoints: EndpointConfig {
                feed_url: value_or_default(env, "ALLESBUIEN_FEED_URL", DEFAULT_FEED_URL),
                raintext_url: value_or_default(
                    env,
                    "ALLESBUIEN_RAINTEXT_URL",
                    DEFAULT_RAINTEXT_URL,
                ),
                radar_url: value_or_default(env, "ALLESBUIEN_RADAR_URL", DEFAULT_RADAR_URL),
            },
            notify: resolve_notify(env),
            state_dir: env
                .var("XDG_STATE_HOME")
                .and_then(non_blank)
                .map_or_else(|| home.join(".local/state"), PathBuf::from)
                .join("allesbuien"),
        }
    }
}

fn resolve_notify(env: &impl EnvSource) -> NotifyConfig {
    let enabled = env.var("ALLESBUIEN_NOTIFY").is_none_or(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let threshold_minutes = env
        .var("ALLESBUIEN_NOTIFY_THRESHOLD_MIN")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_NOTIFY_THRESHOLD_MINUTES);
    let cooldown_minutes = env
        .var("ALLESBUIEN_NOTIFY_COOLDOWN_MIN")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_NOTIFY_COOLDOWN_MINUTES);
    NotifyConfig {
        enabled,
        threshold_minutes,
        cooldown: Duration::from_secs(cooldown_minutes.saturating_mul(60)),
    }
}

fn value_or_default(env: &impl EnvSource, key: &str, default: &str) -> String {
    env.var(key)
        .and_then(non_blank)
        .unwrap_or_else(|| default.to_owned())
}

/// Treats a blank or whitespace-only setting as unset, the rule every override here follows.
pub(crate) fn non_blank(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Config, DEFAULT_FEED_URL, DEFAULT_RADAR_URL, DEFAULT_RAINTEXT_URL};
    use crate::test_support::{FakeEnv, write_weather};

    #[test]
    fn environment_location_wins_over_file() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"File","latitude":53.0,"longitude":5.0}"#,
        );
        let env = FakeEnv([
            ("ALLESBUIEN_LAT", "51.9"),
            ("ALLESBUIEN_LON", "4.5"),
            ("ALLESBUIEN_LOCATION_LABEL", "Rotterdam"),
            ("ALLESBUIEN_NOTIFY_THRESHOLD_MIN", "12"),
            ("ALLESBUIEN_NOTIFY_COOLDOWN_MIN", "34"),
            ("XDG_STATE_HOME", "/tmp/state"),
        ]);
        // When
        let config = Config::resolve(&env, home.path());
        let location = config.location.resolve();
        // Then
        assert_eq!(
            (
                location.latitude,
                location.longitude,
                location.label.as_str(),
                config.notify.threshold_minutes,
                config.notify.cooldown.as_secs()
            ),
            (51.9, 4.5, "Rotterdam", 12, 2_040)
        );
        assert_eq!(config.state_dir, Path::new("/tmp/state/allesbuien"));
    }

    #[test]
    fn default_location_is_used_when_sources_are_absent() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        // When
        let config = Config::resolve(&FakeEnv([]), home.path());
        let location = config.location.resolve();
        // Then
        assert_eq!(
            (
                location.latitude,
                location.longitude,
                location.label.as_str()
            ),
            (52.37, 4.84, "Amsterdam-West")
        );
        assert_eq!(
            (
                config.endpoints.feed_url.as_str(),
                config.endpoints.raintext_url.as_str(),
                config.endpoints.radar_url.as_str()
            ),
            (DEFAULT_FEED_URL, DEFAULT_RAINTEXT_URL, DEFAULT_RADAR_URL)
        );
        assert_eq!(
            (
                config.notify.threshold_minutes,
                config.notify.cooldown.as_secs()
            ),
            (30, 3_600)
        );
        assert_eq!(
            config.state_dir,
            home.path().join(".local/state/allesbuien")
        );
    }

    #[test]
    fn malformed_file_falls_through_without_panicking() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"Broken","latitude":"oops","longitude":5.0}"#,
        );
        let env = FakeEnv([
            ("ALLESBUIEN_NOTIFY_THRESHOLD_MIN", "bad"),
            ("ALLESBUIEN_NOTIFY_COOLDOWN_MIN", "bad"),
        ]);
        // When
        let config = Config::resolve(&env, home.path());
        // Then
        assert_eq!(config.location.resolve().label, "Amsterdam-West");
        assert_eq!(
            (
                config.notify.threshold_minutes,
                config.notify.cooldown.as_secs()
            ),
            (30, 3_600)
        );
    }

    #[test]
    fn the_resolved_location_setting_stays_answerable_after_the_desktop_rewrites_it() {
        // Given: the configuration a running process holds, resolved once at startup.
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"Utrecht","latitude":52.09,"longitude":5.12}"#,
        );
        let config = Config::resolve(&FakeEnv([]), home.path());
        let before = config.location.resolve();

        // When: Omarchy rewrites the setting under the process that already read it.
        write_weather(
            home.path(),
            r#"{"name":"Groningen","latitude":53.22,"longitude":6.57}"#,
        );

        // Then
        assert_eq!(before.label, "Utrecht");
        assert_eq!(config.location.resolve().label, "Groningen");
    }

    #[test]
    fn feed_endpoint_override_wins() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        let env = FakeEnv([("ALLESBUIEN_FEED_URL", "http://127.0.0.1:9")]);
        // When
        let config = Config::resolve(&env, home.path());
        // Then
        assert_eq!(config.endpoints.feed_url, "http://127.0.0.1:9");
    }

    #[test]
    fn raintext_endpoint_override_wins() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        let env = FakeEnv([("ALLESBUIEN_RAINTEXT_URL", "https://rain.test")]);
        // When
        let config = Config::resolve(&env, home.path());
        // Then
        assert_eq!(config.endpoints.raintext_url, "https://rain.test");
    }

    #[test]
    fn radar_endpoint_override_wins() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        let env = FakeEnv([("ALLESBUIEN_RADAR_URL", "https://radar.test")]);
        // When
        let config = Config::resolve(&env, home.path());
        // Then
        assert_eq!(config.endpoints.radar_url, "https://radar.test");
    }

    #[test]
    fn a_blank_endpoint_override_is_ignored_rather_than_obeyed() {
        // Given: a variable that is set but empty, as an unset shell var expands to.
        let home = tempfile::tempdir().expect("temporary home");
        let env = FakeEnv([("ALLESBUIEN_RADAR_URL", "   ")]);
        // When
        let config = Config::resolve(&env, home.path());
        // Then
        assert_eq!(config.endpoints.radar_url, DEFAULT_RADAR_URL);
    }
}
