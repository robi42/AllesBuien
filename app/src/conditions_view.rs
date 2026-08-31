//! Display strings for the current-conditions panel.
//!
//! Formatting and the English-first condition ladder live here rather than in
//! the controller so they can be asserted without a `QGuiApplication`.

use allesbuien_core::{condition::describe, weather::CurrentConditions};

use crate::condition_icon;

const UNAVAILABLE: &str = "Unavailable";

/// Every value the conditions panel renders, already formatted for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionsView {
    pub station_name: String,
    pub temperature: String,
    pub wind: String,
    pub condition: String,
    pub icon_url: String,
    pub observed_label: String,
    pub status: String,
}

impl Default for ConditionsView {
    fn default() -> Self {
        Self {
            station_name: UNAVAILABLE.to_owned(),
            temperature: UNAVAILABLE.to_owned(),
            wind: UNAVAILABLE.to_owned(),
            condition: UNAVAILABLE.to_owned(),
            icon_url: String::new(),
            observed_label: String::new(),
            status: String::from("Current conditions unavailable"),
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
        let observed = current.observed_at.time_label.trim();
        Self {
            station_name: current.station_name.clone(),
            temperature: measurement(current.temperature, "°C"),
            wind: measurement(current.wind_speed, "m/s"),
            condition: describe(
                &current.condition_code,
                current.condition.as_deref().unwrap_or_default(),
            ),
            icon_url: condition_icon::icon_url(&current.condition_code, ""),
            observed_label: if observed.is_empty() {
                String::new()
            } else {
                format!("Observed {observed}")
            },
            status: String::new(),
        }
    }
}

fn measurement(value: Option<f64>, unit: &str) -> String {
    value.map_or_else(
        || UNAVAILABLE.to_owned(),
        |value| format!("{value:.1} {unit}"),
    )
}

#[cfg(test)]
mod tests {
    use allesbuien_core::{datetime::LocalTimestamp, weather::CurrentConditions};

    use super::ConditionsView;

    #[test]
    fn a_complete_measurement_formats_every_panel_string() {
        // Given
        let current = CurrentConditions {
            station_name: String::from("Meetstation Schiphol"),
            temperature: Some(20.34),
            wind_speed: Some(9.9),
            condition: Some(String::from("Zwaar bewolkt")),
            condition_code: String::from("qq"),
            observed_at: LocalTimestamp::from_feed_in("2026-08-31T17:50:00", "Europe/Amsterdam"),
            ..CurrentConditions::default()
        };

        // When
        let view = ConditionsView::from_current(Some(&current));

        // Then
        assert_eq!(view.station_name, "Meetstation Schiphol");
        assert_eq!(view.temperature, "20.3 °C");
        assert_eq!(view.wind, "9.9 m/s");
        assert_eq!(view.condition, "Heavily clouded with rain");
        assert!(
            view.icon_url.ends_with("/96x96/QQ.png"),
            "{}",
            view.icon_url
        );
        assert_eq!(view.observed_label, "Observed 17:50");
        assert!(view.status.is_empty());
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
        assert!(view.icon_url.is_empty());
        assert!(view.observed_label.is_empty());
    }

    #[test]
    fn a_missing_measurement_reports_every_field_as_unavailable() {
        // Given / When
        let view = ConditionsView::from_current(None);

        // Then
        assert_eq!(view.temperature, "Unavailable");
        assert_eq!(view.wind, "Unavailable");
        assert_eq!(view.condition, "Unavailable");
        assert_eq!(view.status, "Current conditions unavailable");
    }

    #[test]
    fn absent_optional_readings_do_not_fabricate_numbers() {
        // Given
        let current = CurrentConditions {
            station_name: String::from("Meetstation Schiphol"),
            condition_code: String::from("a"),
            ..CurrentConditions::default()
        };

        // When
        let view = ConditionsView::from_current(Some(&current));

        // Then
        assert_eq!(view.temperature, "Unavailable");
        assert_eq!(view.wind, "Unavailable");
        assert_eq!(view.condition, "Clear");
        assert!(view.status.is_empty());
    }
}
