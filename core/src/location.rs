//! Where the viewer is, and how that is decided.
//!
//! Three sources answer the same question with different authority: the
//! `ALLESBUIEN_LAT`/`ALLESBUIEN_LON` pair a user set for this app, Omarchy's own
//! `weather.json` when the desktop already knows a place, and Amsterdam-West as
//! the last resort. Keeping the order in one function is what makes the
//! precedence readable — the alternative is a rule spread across three call
//! sites that nobody can state without reading all three.
//!
//! Every source is untrusted and every one of them can be half-written: a
//! coordinate arrives as a JSON number or a JSON string, in range or not, named
//! or nameless. A source that cannot produce a complete, in-range, named
//! location falls through to the next rather than contributing part of one,
//! which is why a damaged `weather.json` degrades to Amsterdam-West instead of
//! to a location off the coast of Africa.

use std::path::Path;

use serde::Deserialize;

use crate::config::EnvSource;

const DEFAULT_LABEL: &str = "Amsterdam-West";
/// Shown instead of an empty header when a caller supplies a nameless location.
const UNNAMED_LABEL: &str = "Your location";
const DEFAULT_LATITUDE: f64 = 52.37;
const DEFAULT_LONGITUDE: f64 = 4.84;

#[derive(Debug, Clone, PartialEq)]
pub struct LocationConfig {
    pub latitude: f64,
    pub longitude: f64,
    pub label: String,
}

impl LocationConfig {
    /// The place name to show for this location; never blank.
    ///
    /// [`resolve`] cannot produce an empty label, but `LocationConfig` is a
    /// public value other callers construct, and a blank header reads as a
    /// rendering bug rather than as missing configuration.
    #[must_use]
    pub fn display_label(&self) -> &str {
        let trimmed = self.label.trim();
        if trimmed.is_empty() {
            UNNAMED_LABEL
        } else {
            trimmed
        }
    }
}

/// Resolves the viewer's location: environment, then Omarchy settings, then default.
pub(crate) fn resolve(env: &impl EnvSource, home: &Path) -> LocationConfig {
    from_env(env)
        .or_else(|| from_file(home))
        .unwrap_or_else(default_location)
}

/// Omarchy's `weather.json`, whose key spellings have varied across releases.
#[derive(Deserialize)]
struct WeatherLocation {
    #[serde(alias = "label")]
    name: Option<String>,
    #[serde(alias = "lat")]
    latitude: Option<serde_json::Value>,
    #[serde(alias = "lon", alias = "lng")]
    longitude: Option<serde_json::Value>,
}

fn from_env(env: &impl EnvSource) -> Option<LocationConfig> {
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

fn from_file(home: &Path) -> Option<LocationConfig> {
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

    use super::{LocationConfig, resolve};
    use crate::config::EnvSource;

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
    fn file_location_wins_when_environment_is_absent() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"Utrecht","latitude":"52.09","longitude":5.12,"extra":true}"#,
        );
        // When
        let location = resolve(&FakeEnv([]), home.path());
        // Then
        assert_eq!(
            (
                location.latitude,
                location.longitude,
                location.label.as_str()
            ),
            (52.09, 5.12, "Utrecht")
        );
    }

    #[test]
    fn an_out_of_range_environment_coordinate_falls_through_to_the_next_source() {
        // Given: a latitude no point on Earth has, alongside a usable file.
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"Utrecht","latitude":52.09,"longitude":5.12}"#,
        );
        let env = FakeEnv([("ALLESBUIEN_LAT", "91.0"), ("ALLESBUIEN_LON", "4.5")]);
        // When
        let location = resolve(&env, home.path());
        // Then
        assert_eq!(location.label, "Utrecht");
    }

    #[test]
    fn a_nameless_file_entry_is_rejected_rather_than_shown_without_a_place() {
        // Given: coordinates Omarchy knows, but no name to caption them with.
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(home.path(), r#"{"latitude":52.09,"longitude":5.12}"#);
        // When
        let location = resolve(&FakeEnv([]), home.path());
        // Then
        assert_eq!(
            (location.latitude, location.label.as_str()),
            (52.37, "Amsterdam-West")
        );
    }

    #[test]
    fn the_resolved_location_always_has_a_name_to_display() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        let resolved = resolve(&FakeEnv([]), home.path());
        let padded = LocationConfig {
            label: String::from("  Utrecht  "),
            ..resolved.clone()
        };
        let nameless = LocationConfig {
            label: String::from("   "),
            ..resolved.clone()
        };

        // When / Then
        assert_eq!(resolved.display_label(), "Amsterdam-West");
        assert_eq!(padded.display_label(), "Utrecht");
        assert_eq!(nameless.display_label(), "Your location");
    }
}
