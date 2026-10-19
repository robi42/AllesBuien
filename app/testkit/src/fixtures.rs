//! The weather answers more than one GUI harness renders.
//!
//! A harness that only needs *a* populated window reaches for these instead of
//! transcribing its own five-day response, so the forecast every harness sees is
//! the same one and a change to its shape happens in one place.

use allesbuien_core::{
    datetime::{LocalTimestamp, SOURCE_TIMEZONE},
    rain::parse_series,
    weather::{CurrentConditions, ForecastDay, RainSoon, WeatherData},
};

/// Rain ten minutes out, peaking at 1.5 mm/h: inside every notification
/// threshold the harnesses configure.
#[must_use]
pub fn rain_soon() -> RainSoon {
    RainSoon {
        minutes_until: 10,
        time_label: String::from("14:10"),
        max_mmh: 1.5,
    }
}

/// A complete feed answer: a Schiphol measurement, five forecast days, and a
/// four-sample rain window whose third sample is the first wet one.
///
/// The station is deliberately not the configured location's label, so an
/// assertion that swapped the two would fail.
#[must_use]
pub fn five_day_weather() -> WeatherData {
    WeatherData {
        current: Some(CurrentConditions {
            station_name: String::from("Schiphol"),
            temperature: Some(19.5),
            feel_temperature: Some(18.8),
            wind_speed: Some(3.2),
            humidity: Some(71.0),
            condition: Some(String::from("Half bewolkt")),
            condition_code: String::from("qq"),
            observed_at: LocalTimestamp::from_feed_in("2026-08-31T17:50:00", SOURCE_TIMEZONE),
        }),
        forecast: (1..=5).map(forecast_day).collect(),
        rain_line: String::from("starts around 14:10"),
        rain_soon: Some(rain_soon()),
        rain_series: parse_series("000|14:00\n000|14:05\n077|14:10\n090|14:15\n", 0).points,
    }
}

/// The `day`th of September 2026, cloudy with light rain and a 20 % chance of it.
fn forecast_day(day: i32) -> ForecastDay {
    ForecastDay {
        day: format!("2026-09-0{day}T00:00:00"),
        min_temperature_min: 10 + day,
        max_temperature_max: 19 + day,
        rain_chance: 20,
        condition: String::from("Bewolkt"),
        condition_code: String::from("f"),
        day_at: LocalTimestamp::from_feed_in(&format!("2026-09-0{day}T12:00:00"), SOURCE_TIMEZONE),
        ..ForecastDay::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{five_day_weather, rain_soon};

    #[test]
    fn the_five_day_answer_is_complete_and_about_to_rain() {
        // Given / When
        let weather = five_day_weather();

        // Then
        assert_eq!(weather.forecast.len(), 5);
        assert_eq!(weather.forecast[4].day, "2026-09-05T00:00:00");
        assert_eq!(weather.rain_soon, Some(rain_soon()));
        assert_eq!(weather.rain_series.len(), 4);
        assert!(
            weather
                .current
                .is_some_and(|current| current.station_name == "Schiphol")
        );
    }
}
