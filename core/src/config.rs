// allow: SIZE_OK — plan acceptance requires comprehensive inline config tests.
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

const DEFAULT_FEED_URL: &str = "https://data.buienradar.nl/2.0/feed/json";
const DEFAULT_RAINTEXT_URL: &str = "https://gpsgadget.buienradar.nl/data/raintext";
const DEFAULT_RADAR_URL: &str = "https://image.buienradar.nl";
const DEFAULT_LABEL: &str = "Amsterdam-West";
const DEFAULT_LATITUDE: f64 = 52.37;
const DEFAULT_LONGITUDE: f64 = 4.84;
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

#[derive(Debug, Clone, PartialEq)]
pub struct LocationConfig {
    pub latitude: f64,
    pub longitude: f64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConfig {
    pub feed_url: String,
    pub raintext_url: String,
    pub radar_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyConfig {
    pub enabled: bool,
    pub threshold_minutes: u32,
    pub cooldown: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub location: LocationConfig,
    pub endpoints: EndpointConfig,
    pub notify: NotifyConfig,
    pub state_dir: PathBuf,
}

impl Config {
    #[must_use]
    pub fn load() -> Self {
        let env = StdEnv;
        let home = env
            .var("HOME")
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        Self::resolve(&env, &home)
    }

    #[must_use]
    pub fn resolve(env: &impl EnvSource, home: &Path) -> Self {
        Self {
            location: location_from_env(env)
                .or_else(|| location_from_file(home))
                .unwrap_or_else(default_location),
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
                .filter(|value| !value.trim().is_empty())
                .map_or_else(|| home.join(".local/state"), PathBuf::from)
                .join("allesbuien"),
        }
    }
}

#[derive(Deserialize)]
struct WeatherLocation {
    #[serde(alias = "label")]
    name: Option<String>,
    #[serde(alias = "lat")]
    latitude: Option<serde_json::Value>,
    #[serde(alias = "lon", alias = "lng")]
    longitude: Option<serde_json::Value>,
}

fn location_from_env(env: &impl EnvSource) -> Option<LocationConfig> {
    let latitude = parse_coordinate(&env.var("ALLESBUIEN_LAT")?, -90.0, 90.0)?;
    let longitude = parse_coordinate(&env.var("ALLESBUIEN_LON")?, -180.0, 180.0)?;
    let label = env
        .var("ALLESBUIEN_LOCATION_LABEL")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("Custom"));
    Some(LocationConfig {
        latitude,
        longitude,
        label,
    })
}

fn location_from_file(home: &Path) -> Option<LocationConfig> {
    let contents =
        std::fs::read_to_string(home.join(".local/state/omarchy/settings/weather.json")).ok()?;
    let weather: WeatherLocation = serde_json::from_str(&contents).ok()?;
    let latitude = parse_json_coordinate(weather.latitude?, -90.0, 90.0)?;
    let longitude = parse_json_coordinate(weather.longitude?, -180.0, 180.0)?;
    Some(LocationConfig {
        latitude,
        longitude,
        label: weather.name.filter(|value| !value.trim().is_empty())?,
    })
}

fn parse_json_coordinate(value: serde_json::Value, minimum: f64, maximum: f64) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number
            .as_f64()
            .filter(|v| (*v >= minimum) && (*v <= maximum)),
        serde_json::Value::String(text) => parse_coordinate(&text, minimum, maximum),
        _ => None,
    }
}

fn parse_coordinate(value: &str, minimum: f64, maximum: f64) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite() && (*parsed >= minimum) && (*parsed <= maximum))
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
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn default_location() -> LocationConfig {
    LocationConfig {
        latitude: DEFAULT_LATITUDE,
        longitude: DEFAULT_LONGITUDE,
        label: DEFAULT_LABEL.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{Config, DEFAULT_FEED_URL, DEFAULT_RADAR_URL, DEFAULT_RAINTEXT_URL, EnvSource};

    struct FakeEnv<const N: usize>([(&'static str, &'static str); N]);

    impl<const N: usize> EnvSource for FakeEnv<N> {
        fn var(&self, key: &str) -> Option<String> {
            self.0
                .iter()
                .find_map(|(candidate, value)| (*candidate == key).then(|| (*value).to_owned()))
        }
    }

    fn write_weather(home: &Path, json: &str) {
        let path = home.join(".local/state/omarchy/settings/weather.json");
        fs::create_dir_all(path.parent().expect("weather parent")).expect("create settings");
        fs::write(path, json).expect("write weather fixture");
    }

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
        // Then
        assert_eq!(
            (
                config.location.latitude,
                config.location.longitude,
                config.location.label.as_str(),
                config.notify.threshold_minutes,
                config.notify.cooldown.as_secs()
            ),
            (51.9, 4.5, "Rotterdam", 12, 2_040)
        );
        assert_eq!(config.state_dir, Path::new("/tmp/state/allesbuien"));
    }

    #[test]
    fn file_location_wins_when_environment_is_absent() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"Utrecht","latitude":"52.09","longitude":5.12,"extra":true}"#,
        );
        // When
        let config = Config::resolve(&FakeEnv([]), home.path());
        // Then
        assert_eq!(
            (
                config.location.latitude,
                config.location.longitude,
                config.location.label.as_str()
            ),
            (52.09, 5.12, "Utrecht")
        );
    }

    #[test]
    fn default_location_is_used_when_sources_are_absent() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        // When
        let config = Config::resolve(&FakeEnv([]), home.path());
        // Then
        assert_eq!(
            (
                config.location.latitude,
                config.location.longitude,
                config.location.label.as_str()
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
        assert_eq!(config.location.label, "Amsterdam-West");
        assert_eq!(
            (
                config.notify.threshold_minutes,
                config.notify.cooldown.as_secs()
            ),
            (30, 3_600)
        );
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
}
