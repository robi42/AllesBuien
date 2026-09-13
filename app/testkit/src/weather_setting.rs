//! Omarchy's desktop location setting, written and removed the way the desktop does.
//!
//! `omarchy-weather-location --set` rewrites `weather.json` and `--clear` unlinks
//! it; a harness about following that setting has to do both under a throwaway
//! home, and the path it writes is the one `allesbuien-core` reads.

use std::{fs, path::Path};

use allesbuien_core::location::OMARCHY_WEATHER_SETTING;

/// Writes `json` as the desktop's `weather.json` under `home`.
///
/// # Panics
///
/// When the settings directory or the file cannot be written.
pub fn write_weather(home: &Path, json: &str) {
    let path = home.join(OMARCHY_WEATHER_SETTING);
    fs::create_dir_all(path.parent().expect("the setting lives in a directory"))
        .expect("create the settings directory");
    fs::write(path, json).expect("write the desktop location");
}

/// Removes the desktop's `weather.json` under `home`, as `--clear` does.
///
/// # Panics
///
/// When there is no setting to remove.
pub fn clear_weather(home: &Path) {
    fs::remove_file(home.join(OMARCHY_WEATHER_SETTING)).expect("remove the desktop location");
}

#[cfg(test)]
mod tests {
    use allesbuien_core::location_settings::LocationSettings;

    use super::{clear_weather, write_weather};

    #[test]
    fn a_written_setting_is_read_back_and_a_cleared_one_falls_back() {
        // Given
        let home = tempfile::tempdir().expect("temporary home");
        let settings = LocationSettings::watching(home.path().to_owned());

        // When
        write_weather(
            home.path(),
            r#"{"name":"Utrecht","latitude":52.09,"longitude":5.12}"#,
        );
        let written = settings.resolve();
        clear_weather(home.path());
        let cleared = settings.resolve();

        // Then
        assert_eq!(written.label, "Utrecht");
        assert_eq!(cleared.label, "Amsterdam-West");
    }
}
