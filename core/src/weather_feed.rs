//! Buienradar's JSON feed as it is published, and its projection into weather values.
//!
//! The vendor's spellings are its own — `stationmeasurements`, `feeltemperature`,
//! `iconurl` — and its `actual`/`forecast` envelope carries far more than this app
//! reads. Holding that wire shape here, behind the single [`parse_feed`] entry
//! point, keeps those names out of the rest of the crate and leaves
//! [`crate::weather`] to describe the transport seam alone.
//!
//! Choosing a station is part of reading the feed rather than part of fetching it:
//! `actual` reports every Dutch station at once and says nothing about which one
//! the viewer meant, so the nearest-station search lives here with the payload it
//! searches.
//!
//! [`ForecastDay`] is the one type that spans both sides. Its rows already have the
//! shape the forecast list wants, so it deserializes in place rather than through a
//! parallel wire struct; the fields the feed does not carry are filled in below.

use serde::Deserialize;

use crate::{
    condition,
    datetime::LocalTimestamp,
    station::station_place_name,
    weather::{CurrentConditions, ForecastDay, WeatherData, WeatherError},
};

/// One station's readings, exactly as `actual.stationmeasurements` publishes them.
#[derive(Debug, Clone, Deserialize)]
struct StationMeasurement {
    #[serde(rename = "stationname")]
    station_name: String,
    lat: f64,
    lon: f64,
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(rename = "feeltemperature", default)]
    feel_temperature: Option<f64>,
    #[serde(rename = "windspeed", default)]
    wind_speed: Option<f64>,
    #[serde(default)]
    humidity: Option<f64>,
    #[serde(rename = "weatherdescription", default)]
    condition: Option<String>,
    #[serde(rename = "iconurl", default)]
    icon_url: String,
    #[serde(default)]
    timestamp: String,
}

#[derive(Deserialize)]
struct FeedPayload {
    actual: Actual,
    forecast: Forecast,
}

#[derive(Deserialize)]
struct Actual {
    #[serde(default)]
    stationmeasurements: Vec<StationMeasurement>,
}

#[derive(Deserialize)]
struct Forecast {
    #[serde(default)]
    fivedayforecast: Vec<ForecastDay>,
}

/// Reads one feed response into the values the weather panels display.
///
/// The coordinates select the station; everything else comes from the payload.
///
/// # Errors
/// Returns [`WeatherError::FeedDecode`] when the body is not the documented JSON.
pub(crate) fn parse_feed(
    feed: &str,
    latitude: f64,
    longitude: f64,
) -> Result<WeatherData, WeatherError> {
    let payload: FeedPayload = serde_json::from_str(feed)?;
    let nearest = nearest_station(&payload.actual.stationmeasurements, latitude, longitude);
    let current = nearest.map(|measurement| CurrentConditions {
        station_name: station_place_name(&measurement.station_name),
        temperature: measurement.temperature,
        feel_temperature: measurement.feel_temperature,
        wind_speed: measurement.wind_speed,
        humidity: measurement.humidity,
        condition: measurement.condition.clone(),
        condition_code: condition::code_from_icon_url(&measurement.icon_url).unwrap_or_default(),
        observed_at: LocalTimestamp::from_feed(&measurement.timestamp),
    });
    let mut forecast = payload.forecast.fivedayforecast;
    for day in &mut forecast {
        day.condition_code = condition::code_from_icon_url(&day.icon_url).unwrap_or_default();
        day.day_at = LocalTimestamp::from_feed(&day.day);
    }
    Ok(WeatherData {
        current,
        forecast,
        rain_line: String::new(),
        rain_soon: None,
        rain_series: Vec::new(),
    })
}

/// Picks the station closest to the configured coordinates.
fn nearest_station(
    stations: &[StationMeasurement],
    latitude: f64,
    longitude: f64,
) -> Option<&StationMeasurement> {
    stations.iter().min_by(|left, right| {
        angular_separation(left, latitude, longitude)
            .total_cmp(&angular_separation(right, latitude, longitude))
    })
}

/// Great-circle separation in radians, by the haversine formula.
///
/// The Earth's radius is deliberately left out: the result is only ever compared
/// against another one, and a shared constant factor cannot change that ordering.
fn angular_separation(station: &StationMeasurement, latitude: f64, longitude: f64) -> f64 {
    let latitude_delta = (station.lat - latitude).to_radians();
    let longitude_delta = (station.lon - longitude).to_radians();
    let station_latitude = station.lat.to_radians();
    let target_latitude = latitude.to_radians();
    let half_chord_squared = (latitude_delta / 2.0).sin().powi(2)
        + station_latitude.cos() * target_latitude.cos() * (longitude_delta / 2.0).sin().powi(2);
    2.0 * half_chord_squared.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::{StationMeasurement, nearest_station, parse_feed};
    use crate::datetime::LocalTimestamp;

    const FEED_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/feed.json"
    ));

    #[test]
    fn nearest_station_prefers_smallest_distance() {
        // Given: deserialized rather than built, so the search runs on real rows.
        let stations: Vec<StationMeasurement> = serde_json::from_str(
            r#"[{"stationname":"Far","lat":51.0,"lon":4.0},
                {"stationname":"Near","lat":52.37,"lon":4.84}]"#,
        )
        .expect("stations should parse");

        // When
        let station = nearest_station(&stations, 52.37, 4.84).expect("station expected");

        // Then
        assert_eq!(station.station_name, "Near");
    }

    #[test]
    fn empty_stationmeasurements_has_no_current_conditions() {
        // Given
        let feed = r#"{
            "actual":{"stationmeasurements":[]},
            "forecast":{"fivedayforecast":[]}
        }"#;

        // When
        let weather = parse_feed(feed, 52.37, 4.84).expect("feed should parse");

        // Then
        assert!(weather.current.is_none());
    }

    #[test]
    fn forecast_uses_integer_range_fields_not_string_temperatures() {
        // Given
        let feed = r#"{
            "actual":{"stationmeasurements":[]},
            "forecast":{"fivedayforecast":[{
                "day":"2026-08-30T00:00:00",
                "mintemperature":"not-a-number",
                "maxtemperature":"also-not-a-number",
                "mintemperatureMin":14,
                "mintemperatureMax":16,
                "maxtemperatureMin":20,
                "maxtemperatureMax":22,
                "rainChance":40,
                "weatherdescription":"Cloudy"
            }]}
        }"#;

        // When
        let weather = parse_feed(feed, 52.37, 4.84).expect("feed should parse");

        // Then
        assert_eq!(weather.forecast[0].min_temperature_min, 14);
        assert_eq!(weather.forecast[0].max_temperature_max, 22);
    }

    #[test]
    fn malformed_feed_returns_typed_decode_error() {
        // Given / When
        let result = parse_feed("not-json", 52.37, 4.84);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn missing_station_readings_stay_absent_rather_than_becoming_zero() {
        // Given
        let feed = r#"{
            "actual":{"stationmeasurements":[{"stationname":"Sparse","lat":52.37,"lon":4.84}]},
            "forecast":{"fivedayforecast":[]}
        }"#;

        // When
        let weather = parse_feed(feed, 52.37, 4.84).expect("feed should parse");

        // Then: absent is not zero, and only absence lets the panel say so.
        let current = weather.current.expect("station expected");
        assert_eq!(current.station_name, "Sparse");
        assert_eq!(
            (
                current.temperature,
                current.feel_temperature,
                current.wind_speed,
                current.humidity
            ),
            (None, None, None, None)
        );
    }

    #[test]
    fn current_conditions_carry_the_icon_code_and_the_feeds_own_text_and_time() {
        // Given / When
        let weather = parse_feed(FEED_FIXTURE, 52.07, 5.88).expect("fixture should parse");

        // Then
        let current = weather.current.expect("nearest station expected");
        assert_eq!(current.station_name, "Arnhem");
        assert_eq!(current.feel_temperature, Some(18.0));
        assert_eq!(current.humidity, Some(94.0));
        assert_eq!(current.condition_code, "qq");
        assert_eq!(current.condition.as_deref(), Some("Zwaar bewolkt en regen"));
        assert_eq!(
            current.observed_at,
            LocalTimestamp::from_feed("2026-08-29T23:10:00")
        );
    }

    #[test]
    fn forecast_days_carry_the_icon_code_and_the_feeds_own_day() {
        // Given / When
        let weather = parse_feed(FEED_FIXTURE, 52.37, 4.84).expect("fixture should parse");

        // Then
        let first = &weather.forecast[0];
        assert_eq!(first.condition_code, "f");
        assert_eq!(
            first.day_at,
            LocalTimestamp::from_feed("2026-08-30T00:00:00")
        );
        for day in &weather.forecast {
            assert!(!day.condition_code.is_empty(), "{}", day.day);
        }
    }

    #[test]
    fn an_unmapped_icon_code_is_kept_beside_the_dutch_description() {
        // Given
        let feed = r#"{
            "actual":{"stationmeasurements":[{
                "stationname":"Onbekend","lat":52.37,"lon":4.84,
                "timestamp":"2026-08-30T17:50:00",
                "weatherdescription":"Zwaar bewolkt",
                "iconurl":"https://cdn.buienradar.nl/resources/images/icons/weather/30x30/z.png"
            }]},
            "forecast":{"fivedayforecast":[]}
        }"#;

        // When
        let weather = parse_feed(feed, 52.37, 4.84).expect("feed should parse");

        // Then
        let current = weather.current.expect("station expected");
        assert_eq!(current.condition_code, "z");
        assert_eq!(current.condition.as_deref(), Some("Zwaar bewolkt"));
    }

    #[test]
    fn a_station_without_icon_or_description_reports_neither() {
        // Given
        let feed = r#"{
            "actual":{"stationmeasurements":[{"stationname":"Kaal","lat":52.37,"lon":4.84}]},
            "forecast":{"fivedayforecast":[]}
        }"#;

        // When
        let weather = parse_feed(feed, 52.37, 4.84).expect("feed should parse");

        // Then
        let current = weather.current.expect("station expected");
        assert!(current.condition_code.is_empty());
        assert_eq!(current.condition, None);
    }
}
