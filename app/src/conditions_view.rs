//! Display strings for the current-conditions panel.
//!
//! Formatting and the English-first condition ladder live here rather than in
//! the controller so they can be asserted without a `QGuiApplication`.

use allesbuien_core::{
    condition::describe,
    status::{CONDITIONS_UNAVAILABLE, NO_READING, UNAVAILABLE},
    weather::CurrentConditions,
};

use crate::condition_icon;

/// Every value the conditions panel renders, already formatted for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionsView {
    pub station_name: String,
    pub temperature: String,
    /// Apparent temperature, or empty when the station did not report one.
    pub feels_like: String,
    pub wind: String,
    /// Relative humidity, or empty when the station did not report one.
    pub humidity: String,
    pub condition: String,
    pub icon_url: String,
    pub observed_label: String,
    pub status: String,
}

impl Default for ConditionsView {
    fn default() -> Self {
        Self {
            station_name: UNAVAILABLE.to_owned(),
            temperature: NO_READING.to_owned(),
            feels_like: String::new(),
            wind: UNAVAILABLE.to_owned(),
            humidity: String::new(),
            condition: UNAVAILABLE.to_owned(),
            icon_url: String::new(),
            observed_label: String::new(),
            status: CONDITIONS_UNAVAILABLE.to_owned(),
        }
    }
}

impl ConditionsView {
    /// Projects an optional station measurement onto the panel's strings.
    #[must_use]
    pub fn from_current(current: Option<&CurrentConditions>) -> Self {
        let Some(current) = current else {
            return Self::default();
        };
        Self {
            station_name: current.station_name.clone(),
            temperature: measurement(current.temperature, "°C", NO_READING),
            feels_like: measurement(current.feel_temperature, "°C", ""),
            wind: measurement(current.wind_speed, "m/s", UNAVAILABLE),
            humidity: optional_percentage(current.humidity),
            condition: describe(
                &current.condition_code,
                current.condition.as_deref().unwrap_or_default(),
            ),
            icon_url: condition_icon::display_url(&current.condition_code, ""),
            observed_label: current.observed_at.time_label.trim().to_owned(),
            status: String::new(),
        }
    }
}

/// Formats a station measurement, or the placeholder its rank can carry.
///
/// `absent` is a parameter rather than one constant inside the body because the
/// callers render at ranks three scale steps apart, and the word that fits the
/// chips overruns the hero; the apparent temperature passes an empty one and is
/// simply omitted. Deciding it here for everyone is what shipped
/// `Unavaila…` as the largest thing in the window; deciding it at the call site
/// puts the choice beside the field it is chosen for.
fn measurement(value: Option<f64>, unit: &str, absent: &str) -> String {
    value.map_or_else(|| absent.to_owned(), |value| format!("{value:.1} {unit}"))
}

fn optional_percentage(value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| format!("{}%", value.round()))
}

#[cfg(test)]
mod tests {
    use allesbuien_core::{datetime::LocalTimestamp, weather::CurrentConditions};

    use super::ConditionsView;

    #[test]
    fn a_complete_measurement_formats_every_panel_string() {
        // Given
        let current = CurrentConditions {
            station_name: String::from("Schiphol"),
            temperature: Some(20.34),
            feel_temperature: Some(21.4),
            wind_speed: Some(9.9),
            humidity: Some(65.0),
            condition: Some(String::from("Zwaar bewolkt")),
            condition_code: String::from("qq"),
            observed_at: LocalTimestamp::from_feed_in("2026-08-31T17:50:00", "Europe/Amsterdam"),
        };

        // When
        let view = ConditionsView::from_current(Some(&current));

        // Then
        assert_eq!(view.station_name, "Schiphol");
        assert_eq!(view.temperature, "20.3 °C");
        assert_eq!(view.feels_like, "21.4 °C");
        assert_eq!(view.wind, "9.9 m/s");
        assert_eq!(view.humidity, "65%");
        assert_eq!(view.condition, "Heavily clouded with rain");
        assert_eq!(view.icon_url, "image://condition/QQ");
        assert_eq!(view.observed_label, "17:50");
        assert_eq!(view.status, "");
    }

    #[test]
    fn an_unmapped_code_keeps_the_dutch_wording_rather_than_blanking() {
        // Given
        let current = CurrentConditions {
            condition: Some(String::from("Half bewolkt")),
            ..CurrentConditions::default()
        };

        // When
        let view = ConditionsView::from_current(Some(&current));

        // Then
        assert_eq!(view.condition, "Half bewolkt");
        assert_eq!(view.icon_url, "");
        assert_eq!(view.observed_label, "");
    }

    #[test]
    fn a_missing_measurement_dashes_the_reading_and_words_every_other_field() {
        // Given / When
        let view = ConditionsView::from_current(None);

        // Then
        assert_eq!(view.temperature, "—");
        assert_eq!(view.wind, "Unavailable");
        assert_eq!(view.condition, "Unavailable");
        assert_eq!(view.feels_like, "");
        assert_eq!(view.humidity, "");
        assert_eq!(view.status, "Current conditions unavailable");
    }

    #[test]
    fn absent_optional_readings_do_not_fabricate_numbers() {
        // Given
        let current = CurrentConditions {
            station_name: String::from("Schiphol"),
            condition_code: String::from("a"),
            ..CurrentConditions::default()
        };

        // When
        let view = ConditionsView::from_current(Some(&current));

        // Then
        assert_eq!(view.temperature, "—");
        assert_eq!(view.wind, "Unavailable");
        assert_eq!(view.condition, "Clear");
        assert_eq!(view.feels_like, "");
        assert_eq!(view.humidity, "");
        assert_eq!(view.status, "");
    }

    #[test]
    fn a_station_reporting_only_humidity_omits_the_apparent_temperature() {
        // Given
        let current = CurrentConditions {
            humidity: Some(94.4),
            ..CurrentConditions::default()
        };

        // When
        let view = ConditionsView::from_current(Some(&current));

        // Then
        assert_eq!(view.humidity, "94%");
        assert_eq!(view.feels_like, "");
    }
}
