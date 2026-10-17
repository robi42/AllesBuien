//! The five-day forecast the controller publishes as a `QAbstractListModel`.
//!
//! The role table and the per-row projection live here rather than in the
//! bridge so they can be asserted without a `QGuiApplication`, and so the
//! controller module stays the property surface it is named for. The three
//! `QAbstractListModel` overrides that answer from them are here too: the model
//! is one responsibility, and splitting its data from its Qt surface only meant
//! reading two files to learn what one row contains.

use std::pin::Pin;

use allesbuien_core::{condition::describe, weather::ForecastDay};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant, QVector,
};

use crate::{condition_icon, icon_delivery::PendingIcons, image_provider, weather_controller::ffi};
use ffi::ForecastRole;

impl ffi::WeatherController {
    pub fn row_count(&self, _parent: &QModelIndex) -> i32 {
        i32::try_from(self.forecast.len()).unwrap_or(i32::MAX)
    }

    pub fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        role_value(&self.forecast, &self.pending_icons, index.row(), role)
    }

    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        forecast_role_names()
    }

    pub(crate) fn replace_forecast(mut self: Pin<&mut Self>, forecast: Vec<ForecastDay>) {
        // SAFETY: no operation between this paired reset can fail or panic,
        // and CxxQtThread guarantees execution on the model's owning Qt thread.
        unsafe { self.as_mut().begin_reset_model() };
        self.as_mut().rust_mut().forecast = forecast;
        // SAFETY: this closes the immediately preceding model reset.
        unsafe { self.end_reset_model() };
    }

    /// Tells every row's delegate to re-read its icon and whether it is still
    /// loading, now that a download may have settled since the rows were read.
    pub(crate) fn forecast_icons_changed(self: Pin<&mut Self>) {
        let Some(last) = self.forecast.len().checked_sub(1) else {
            return;
        };
        let root = QModelIndex::default();
        let first_row = self.index(0, 0, &root);
        let last_row = self.index(i32::try_from(last).unwrap_or(i32::MAX), 0, &root);
        let roles = QVector::from([ForecastRole::IconUrl.repr, ForecastRole::IconLoading.repr]);
        self.data_changed(&first_row, &last_row, &roles);
    }
}

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
fn role_value(forecast: &[ForecastDay], pending: &PendingIcons, row: i32, role: i32) -> QVariant {
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
        ForecastRole::IconLoading => QVariant::from(&pending.awaits(&row.icon_url)),
        ForecastRole::IconUrl => QVariant::from(&QString::from(&image_provider::published_url(
            &row.icon_url,
        ))),
        _ => QVariant::default(),
    }
}

/// The QML-facing name of every role [`role_value`] answers.
fn forecast_role_names() -> QHash<QHashPair_i32_QByteArray> {
    let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
    for (role, name) in [
        (ForecastRole::Day.repr, "day"),
        (ForecastRole::Weekday.repr, "weekday"),
        (ForecastRole::MinTemperature.repr, "minTemperature"),
        (ForecastRole::MaxTemperature.repr, "maxTemperature"),
        (ForecastRole::RainChance.repr, "rainChance"),
        (ForecastRole::Condition.repr, "condition"),
        (ForecastRole::IconUrl.repr, "iconUrl"),
        (ForecastRole::IconLoading.repr, "iconLoading"),
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
