//! Where the viewer is, and how that is decided.
//!
//! Three sources answer the same question with different authority: the
//! `ALLESBUIEN_LAT`/`ALLESBUIEN_LON` pair a user set for this app, Omarchy's own
//! `weather.json` when the desktop already knows a place, and Amsterdam-West as
//! the last resort. Keeping the order in one function,
//! [`crate::location_settings::LocationSettings::resolve`], is what makes the
//! precedence readable — the alternative is a rule spread across three call
//! sites that nobody can state without reading all three.
//!
//! Every source is untrusted and every one of them can be half-written: a
//! coordinate arrives as a JSON number or a JSON string, in range or not, named
//! or nameless. A source that cannot produce a complete, in-range, named
//! location falls through to the next rather than contributing part of one,
//! which is why a damaged `weather.json` degrades to Amsterdam-West instead of
//! to a location off the coast of Africa.
//!
//! Which source answered travels with the answer. Amsterdam-West reached the
//! window looking exactly like a place the viewer had chosen, so the one state
//! worth distinguishing — nothing is configured — was the one state the window
//! could not show. [`crate::location_notice`] turns that into wording.

use std::path::Path;

use serde::Deserialize;

use crate::config::{EnvSource, non_blank};

/// Omarchy's desktop-wide weather location setting, relative to `$HOME`.
pub const OMARCHY_WEATHER_SETTING: &str = ".local/state/omarchy/settings/weather.json";

const DEFAULT_LABEL: &str = "Amsterdam-West";
/// Shown instead of an empty header when a caller supplies a nameless location.
const UNNAMED_LABEL: &str = "Your location";
const DEFAULT_LATITUDE: f64 = 52.37;
const DEFAULT_LONGITUDE: f64 = 4.84;

/// Which of the three sources produced a location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationSource {
    /// The `ALLESBUIEN_LAT`/`ALLESBUIEN_LON` pair, set for this app alone.
    Environment,
    /// Omarchy's `weather.json`, shared with the desktop's own weather widget.
    DesktopSetting,
    /// Amsterdam-West, because neither of the other two answered.
    BuiltInDefault,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocationConfig {
    pub latitude: f64,
    pub longitude: f64,
    pub label: String,
    pub source: LocationSource,
}

impl LocationConfig {
    /// The place name to show for this location; never blank.
    ///
    /// [`crate::location_settings::LocationSettings::resolve`] cannot produce an
    /// empty label, but `LocationConfig` is a public value other callers
    /// construct, and a blank header reads as a rendering bug rather than as
    /// missing configuration.
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

pub(crate) fn from_env(env: &impl EnvSource) -> Option<LocationConfig> {
    let latitude = parse_coordinate(&env.var("ALLESBUIEN_LAT")?, -90.0, 90.0)?;
    let longitude = parse_coordinate(&env.var("ALLESBUIEN_LON")?, -180.0, 180.0)?;
    let label = env
        .var("ALLESBUIEN_LOCATION_LABEL")
        .and_then(non_blank)
        .unwrap_or_else(|| String::from("Custom"));
    Some(LocationConfig {
        latitude,
        longitude,
        label,
        source: LocationSource::Environment,
    })
}

pub(crate) fn from_file(home: &Path) -> Option<LocationConfig> {
    let contents = std::fs::read_to_string(home.join(OMARCHY_WEATHER_SETTING)).ok()?;
    let weather: WeatherLocation = serde_json::from_str(&contents).ok()?;
    let latitude = parse_json_coordinate(weather.latitude?, -90.0, 90.0)?;
    let longitude = parse_json_coordinate(weather.longitude?, -180.0, 180.0)?;
    Some(LocationConfig {
        latitude,
        longitude,
        label: weather.name.and_then(non_blank)?,
        source: LocationSource::DesktopSetting,
    })
}

fn parse_json_coordinate(value: serde_json::Value, minimum: f64, maximum: f64) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number
            .as_f64()
            .filter(|degrees| in_range(*degrees, minimum, maximum)),
        serde_json::Value::String(text) => parse_coordinate(&text, minimum, maximum),
        _ => None,
    }
}

fn parse_coordinate(value: &str, minimum: f64, maximum: f64) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|degrees| in_range(*degrees, minimum, maximum))
}

/// Whether a coordinate lies within its axis's bounds; NaN and infinities never do.
fn in_range(degrees: f64, minimum: f64, maximum: f64) -> bool {
    (minimum..=maximum).contains(&degrees)
}

pub(crate) fn default_location() -> LocationConfig {
    LocationConfig {
        latitude: DEFAULT_LATITUDE,
        longitude: DEFAULT_LONGITUDE,
        label: DEFAULT_LABEL.to_owned(),
        source: LocationSource::BuiltInDefault,
    }
}

#[cfg(test)]
mod tests {
    use super::{LocationConfig, LocationSource, default_location, from_env, from_file};
    use crate::test_support::{FakeEnv, write_weather};

    #[test]
    fn each_producer_stamps_the_answer_with_the_source_that_gave_it() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(
            home.path(),
            r#"{"name":"Utrecht","latitude":"52.09","longitude":5.12,"extra":true}"#,
        );
        let env = FakeEnv([("ALLESBUIEN_LAT", "51.9"), ("ALLESBUIEN_LON", "4.5")]);

        // When
        let environment = from_env(&env).expect("a complete environment pair");
        let desktop = from_file(home.path()).expect("a complete file entry");
        let fallback = default_location();

        // Then
        assert_eq!(environment.source, LocationSource::Environment);
        assert_eq!(environment.label, "Custom");
        assert_eq!(desktop.source, LocationSource::DesktopSetting);
        assert_eq!((desktop.latitude, desktop.longitude), (52.09, 5.12));
        assert_eq!(fallback.source, LocationSource::BuiltInDefault);
        assert_eq!(fallback.label, "Amsterdam-West");
    }

    #[test]
    fn an_incomplete_source_produces_nothing_rather_than_half_a_location() {
        // Given: each case is one field short, out of range, or unnamed.
        let home = tempfile::tempdir().expect("temporary home");
        let cases = [
            (r#"{"latitude":52.09,"longitude":5.12}"#, "no name"),
            (
                r#"{"name":"  ","latitude":52.09,"longitude":5.12}"#,
                "blank name",
            ),
            (r#"{"name":"Nowhere","longitude":5.12}"#, "no latitude"),
            (
                r#"{"name":"Broken","latitude":"oops","longitude":5.12}"#,
                "unparseable",
            ),
            (
                r#"{"name":"Impossible","latitude":91.0,"longitude":5.12}"#,
                "out of range",
            ),
            (
                r#"{"name":"Listy","latitude":[52.09],"longitude":5.12}"#,
                "wrong JSON type",
            ),
            ("not json at all", "not json"),
        ];

        // When / Then
        for (json, case) in cases {
            write_weather(home.path(), json);
            assert_eq!(from_file(home.path()), None, "{case} should not resolve");
        }
        assert_eq!(
            from_env(&FakeEnv([("ALLESBUIEN_LAT", "51.9")])),
            None,
            "a latitude without a longitude should not resolve"
        );
    }

    #[test]
    fn an_environment_label_is_taken_when_it_carries_a_name() {
        // Given
        let named = FakeEnv([
            ("ALLESBUIEN_LAT", "51.9"),
            ("ALLESBUIEN_LON", "4.5"),
            ("ALLESBUIEN_LOCATION_LABEL", "Rotterdam"),
        ]);
        let blank = FakeEnv([
            ("ALLESBUIEN_LAT", "51.9"),
            ("ALLESBUIEN_LON", "4.5"),
            ("ALLESBUIEN_LOCATION_LABEL", "   "),
        ]);

        // When / Then
        assert_eq!(from_env(&named).expect("named").label, "Rotterdam");
        assert_eq!(from_env(&blank).expect("blank").label, "Custom");
    }

    #[test]
    fn the_resolved_location_always_has_a_name_to_display() {
        // Given
        let resolved = default_location();
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
