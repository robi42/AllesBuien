//! The viewer's location as a standing question rather than a startup answer.
//!
//! Omarchy's `weather.json` is a desktop-wide setting: `omarchy-weather-location`
//! rewrites it, `--clear` removes it outright, and the desktop's own weather
//! widget follows both within a second. Reading it once at construction made
//! this app the only surface on the machine that did not, so the same setting
//! produced two different places until the app was restarted.
//!
//! Holding the *inputs* rather than the answer is what makes the question
//! re-askable, and it is why [`LocationSettings`] carries no interior mutability
//! and no cached value: every caller re-resolves and compares against whatever
//! it last adopted, so two callers polling on their own timers cannot race each
//! other for a change and cannot disagree about what the answer now is.
//!
//! An environment pair is captured once and outranks the file for the process's
//! life. That is the whole reason it exists — a viewer who pinned this app to a
//! coordinate did not ask for the desktop to move it later.

use std::path::PathBuf;

use crate::{
    config::EnvSource,
    location::{self, LocationConfig},
};

#[derive(Debug, Clone, PartialEq)]
pub struct LocationSettings {
    home: PathBuf,
    /// An explicit pin that outranks the desktop setting and short-circuits it.
    override_location: Option<LocationConfig>,
}

impl LocationSettings {
    /// Captures any environment pin, and watches `home` for the rest.
    #[must_use]
    pub fn new(env: &impl EnvSource, home: PathBuf) -> Self {
        Self {
            override_location: location::from_env(env),
            home,
        }
    }

    /// Watches `home` with no pin, so the desktop setting always decides.
    #[must_use]
    pub fn watching(home: PathBuf) -> Self {
        Self {
            home,
            override_location: None,
        }
    }

    /// Answers with one fixed location and never reads a file.
    #[must_use]
    pub fn pinned(location: LocationConfig) -> Self {
        Self {
            home: PathBuf::new(),
            override_location: Some(location),
        }
    }

    /// Asks again: the pin, then Omarchy's setting, then Amsterdam-West.
    #[must_use]
    pub fn resolve(&self) -> LocationConfig {
        self.override_location
            .clone()
            .or_else(|| location::from_file(&self.home))
            .unwrap_or_else(location::default_location)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LocationSettings;
    use crate::{
        location::{LocationConfig, LocationSource},
        test_support::{FakeEnv, clear_weather, write_weather},
    };

    const UTRECHT: &str = r#"{"name":"Utrecht","latitude":"52.09","longitude":5.12}"#;
    const GRONINGEN: &str = r#"{"name":"Groningen","latitude":53.22,"longitude":6.57}"#;

    #[test]
    fn the_desktop_setting_decides_while_no_pin_is_set() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(home.path(), UTRECHT);
        let settings = LocationSettings::new(&FakeEnv([]), home.path().to_owned());

        // When
        let location = settings.resolve();

        // Then
        assert_eq!(
            (
                location.latitude,
                location.longitude,
                location.label.as_str(),
                location.source
            ),
            (52.09, 5.12, "Utrecht", LocationSource::DesktopSetting)
        );
    }

    #[test]
    fn a_rewritten_setting_is_picked_up_by_the_next_resolve() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(home.path(), UTRECHT);
        let settings = LocationSettings::watching(home.path().to_owned());
        let before = settings.resolve();

        // When: as `omarchy-weather-location --set Groningen 53.22,6.57` writes it.
        write_weather(home.path(), GRONINGEN);
        let after = settings.resolve();

        // Then
        assert_eq!(before.label, "Utrecht");
        assert_eq!(after.label, "Groningen");
        assert_eq!((after.latitude, after.longitude), (53.22, 6.57));
    }

    #[test]
    fn a_cleared_setting_falls_back_and_says_it_is_the_default() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(home.path(), UTRECHT);
        let settings = LocationSettings::watching(home.path().to_owned());
        let before = settings.resolve();

        // When: as `omarchy-weather-location --clear` removes it.
        clear_weather(home.path());
        let after = settings.resolve();

        // Then
        assert_eq!(before.source, LocationSource::DesktopSetting);
        assert_eq!(
            (after.label.as_str(), after.latitude, after.source),
            ("Amsterdam-West", 52.37, LocationSource::BuiltInDefault)
        );
    }

    #[test]
    fn an_environment_pin_ignores_every_later_change_to_the_setting() {
        // Given: a pin, and a file that keeps moving underneath it.
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(home.path(), UTRECHT);
        let env = FakeEnv([
            ("ALLESBUIEN_LAT", "51.9"),
            ("ALLESBUIEN_LON", "4.5"),
            ("ALLESBUIEN_LOCATION_LABEL", "Rotterdam"),
        ]);
        let settings = LocationSettings::new(&env, home.path().to_owned());

        // When
        let first = settings.resolve();
        write_weather(home.path(), GRONINGEN);
        let after_rewrite = settings.resolve();
        clear_weather(home.path());
        let after_clear = settings.resolve();

        // Then
        assert_eq!(first, after_rewrite);
        assert_eq!(first, after_clear);
        assert_eq!(
            (first.latitude, first.label.as_str(), first.source),
            (51.9, "Rotterdam", LocationSource::Environment)
        );
    }

    #[test]
    fn an_unusable_environment_pair_falls_through_to_the_setting_and_stays_live() {
        // Given: a latitude no point on Earth has, so the pair never pins.
        let home = tempfile::tempdir().expect("temporary home");
        write_weather(home.path(), UTRECHT);
        let env = FakeEnv([("ALLESBUIEN_LAT", "91.0"), ("ALLESBUIEN_LON", "4.5")]);
        let settings = LocationSettings::new(&env, home.path().to_owned());

        // When
        let before = settings.resolve();
        write_weather(home.path(), GRONINGEN);
        let after = settings.resolve();

        // Then
        assert_eq!(before.label, "Utrecht");
        assert_eq!(after.label, "Groningen");
    }

    #[test]
    fn a_half_written_setting_degrades_to_the_default_rather_than_to_a_partial_place() {
        // Given: what `omarchy-weather-location --set Malibu` writes with no coordinates.
        let home = tempfile::tempdir().expect("temporary home");
        let settings = LocationSettings::watching(home.path().to_owned());
        write_weather(home.path(), r#"{"name":"Malibu"}"#);

        // When
        let location = settings.resolve();

        // Then
        assert_eq!(location.label, "Amsterdam-West");
        assert_eq!(location.source, LocationSource::BuiltInDefault);
    }

    #[test]
    fn a_pinned_settings_object_never_consults_a_file() {
        // Given: a pin whose home is empty, so any read would resolve relatively.
        let pin = LocationConfig {
            latitude: 53.22,
            longitude: 6.57,
            label: String::from("Groningen"),
            source: LocationSource::DesktopSetting,
        };

        // When
        let settings = LocationSettings::pinned(pin.clone());

        // Then
        assert_eq!(settings.resolve(), pin);
        assert_eq!(settings.resolve(), pin);
        assert_eq!(settings.home, PathBuf::new());
    }
}
