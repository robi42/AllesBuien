//! The five-day forecast the controller publishes as a `QAbstractListModel`.
//!
//! The role table and the per-row projection live here rather than in the
//! bridge so they can be asserted without a `QGuiApplication`, and so the
//! controller module stays the property surface it is named for.

use allesbuien_core::{condition::describe, weather::ForecastDay};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QString, QVariant};

use crate::{condition_icon, weather_controller::ffi::ForecastRole};

/// One forecast day, already projected onto the values QML renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastRow {
    pub day: String,
    pub weekday: String,
    pub min_temperature: i32,
    pub max_temperature: i32,
    pub rain_chance: i32,
    pub condition: String,
    pub icon_url: String,
}

impl ForecastRow {
    #[must_use]
    pub fn from_day(day: &ForecastDay) -> Self {
        Self {
            day: day.day.clone(),
            weekday: day.day_at.weekday_label.clone(),
            min_temperature: day.min_temperature_min,
            max_temperature: day.max_temperature_max,
            rain_chance: i32::from(day.rain_chance),
            condition: describe(&day.condition_code, &day.condition),
            icon_url: condition_icon::display_url(&day.condition_code, &day.icon_url),
        }
    }
}

/// Reads one role of one row, or an invalid variant for anything out of range.
#[must_use]
pub fn cell(forecast: &[ForecastDay], row: i32, role: i32) -> QVariant {
    let Some(row) = usize::try_from(row)
        .ok()
        .and_then(|row| forecast.get(row))
        .map(ForecastRow::from_day)
    else {
        return QVariant::default();
    };
    match (ForecastRole { repr: role }) {
        ForecastRole::Day => QVariant::from(&QString::from(&row.day)),
        ForecastRole::Weekday => QVariant::from(&QString::from(&row.weekday)),
        ForecastRole::MinTemperature => QVariant::from(&row.min_temperature),
        ForecastRole::MaxTemperature => QVariant::from(&row.max_temperature),
        ForecastRole::RainChance => QVariant::from(&row.rain_chance),
        ForecastRole::Condition => QVariant::from(&QString::from(&row.condition)),
        ForecastRole::IconUrl => QVariant::from(&QString::from(&row.icon_url)),
        _ => QVariant::default(),
    }
}

/// The QML-facing name of every role [`cell`] answers.
#[must_use]
pub fn role_names() -> QHash<QHashPair_i32_QByteArray> {
    let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
    for (role, name) in [
        (ForecastRole::Day.repr, "day"),
        (ForecastRole::Weekday.repr, "weekday"),
        (ForecastRole::MinTemperature.repr, "minTemperature"),
        (ForecastRole::MaxTemperature.repr, "maxTemperature"),
        (ForecastRole::RainChance.repr, "rainChance"),
        (ForecastRole::Condition.repr, "condition"),
        (ForecastRole::IconUrl.repr, "iconUrl"),
    ] {
        roles.insert(role, QByteArray::from(name));
    }
    roles
}

#[cfg(test)]
mod tests {
    use allesbuien_core::{datetime::LocalTimestamp, weather::ForecastDay};

    use super::ForecastRow;

    fn wednesday() -> ForecastDay {
        ForecastDay {
            day: String::from("2026-09-02T00:00:00"),
            min_temperature_min: 12,
            min_temperature_max: 14,
            max_temperature_min: 19,
            max_temperature_max: 21,
            rain_chance: 40,
            condition: String::from("Zwaar bewolkt en regen"),
            condition_code: String::from("q"),
            day_at: LocalTimestamp::from_feed_in("2026-09-02T12:00:00", "Europe/Amsterdam"),
            ..ForecastDay::default()
        }
    }

    #[test]
    fn a_row_reports_the_outer_temperature_range_and_english_wording() {
        // Given / When
        let row = ForecastRow::from_day(&wednesday());

        // Then
        assert_eq!(row.min_temperature, 12);
        assert_eq!(row.max_temperature, 21);
        assert_eq!(row.rain_chance, 40);
        assert_eq!(row.condition, "Heavily clouded with rain");
        assert_eq!(row.weekday, "Wed");
        assert_eq!(row.icon_url, "image://condition/Q");
    }

    #[test]
    fn a_row_without_a_code_keeps_the_published_icon_and_dutch_wording() {
        // Given
        let day = ForecastDay {
            condition_code: String::new(),
            icon_url: String::from("https://cdn.buienradar.nl/x/30x30/f.png"),
            ..wednesday()
        };

        // When
        let row = ForecastRow::from_day(&day);

        // Then
        assert_eq!(row.icon_url, "https://cdn.buienradar.nl/x/30x30/f.png");
        assert_eq!(row.condition, "Zwaar bewolkt en regen");
    }
}
