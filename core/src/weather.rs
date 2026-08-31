// allow: SIZE_OK — task contract requires the HTTP adapter, wire models, derivation, and inline tests in this module.
use std::{future::Future, pin::Pin, time::Duration};

use serde::Deserialize;
use thiserror::Error;

use crate::{
    condition,
    datetime::{LocalTimestamp, clock_shift_minutes},
    rain::{self, RainSeries},
};

pub use crate::rain::{RainPoint, RainSoon, code_to_mmh as rain_code_to_mmh};

pub const DEFAULT_FEED_URL: &str = "https://data.buienradar.nl/2.0/feed/json";
pub const DEFAULT_RAINTEXT_URL: &str = "https://gpsgadget.buienradar.nl/data/raintext";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

pub type FeedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<WeatherData, WeatherError>> + Send + 'a>>;

pub trait FeedApi: Send + Sync {
    fn fetch(&self, latitude: f64, longitude: f64) -> FeedFuture<'_>;
}

#[derive(Debug, Clone)]
pub struct ReqwestFeedApi {
    client: reqwest::Client,
    feed_url: String,
    raintext_url: String,
}

impl ReqwestFeedApi {
    pub fn new(
        feed_url: impl Into<String>,
        raintext_url: impl Into<String>,
    ) -> Result<Self, WeatherError> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(WeatherError::ClientBuild)?;
        Ok(Self {
            client,
            feed_url: feed_url.into(),
            raintext_url: raintext_url.into(),
        })
    }
}

impl FeedApi for ReqwestFeedApi {
    fn fetch(&self, latitude: f64, longitude: f64) -> FeedFuture<'_> {
        Box::pin(async move {
            let feed = self
                .client
                .get(&self.feed_url)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(WeatherError::FeedRequest)?
                .error_for_status()
                .map_err(WeatherError::FeedRequest)?
                .text()
                .await
                .map_err(WeatherError::FeedRequest)?;
            let raintext = self
                .client
                .get(&self.raintext_url)
                .query(&[
                    ("lat", format!("{latitude:.4}")),
                    ("lon", format!("{longitude:.4}")),
                ])
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(WeatherError::RaintextRequest)?
                .error_for_status()
                .map_err(WeatherError::RaintextRequest)?
                .text()
                .await
                .map_err(WeatherError::RaintextRequest)?;
            let parsed = parse_feed(&feed, latitude, longitude)?;
            let rain = summarize_raintext_detail(&raintext, clock_shift_minutes());
            Ok(WeatherData {
                current: parsed.current,
                forecast: parsed.forecast,
                now_line: parsed.now_line,
                rain_line: rain.line,
                rain_soon: rain.rain_soon,
                rain_series: rain.points,
            })
        })
    }
}

#[derive(Debug, Error)]
pub enum WeatherError {
    #[error("failed to build weather HTTP client: {0}")]
    ClientBuild(reqwest::Error),
    #[error("weather feed request failed: {0}")]
    FeedRequest(reqwest::Error),
    #[error("raintext request failed: {0}")]
    RaintextRequest(reqwest::Error),
    #[error("failed to decode weather feed JSON: {0}")]
    FeedDecode(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CurrentConditions {
    pub station_name: String,
    pub temperature: Option<f64>,
    pub feel_temperature: Option<f64>,
    pub wind_speed: Option<f64>,
    pub wind_bft: Option<i64>,
    pub humidity: Option<f64>,
    /// Dutch `weatherdescription` exactly as published.
    pub condition: Option<String>,
    /// Buienradar icon code, e.g. `q` or the night variant `qq`; empty when absent.
    pub condition_code: String,
    /// English description for [`Self::condition_code`], falling back to Dutch.
    pub condition_english: String,
    /// Observation time projected into the viewer's timezone.
    pub observed_at: LocalTimestamp,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ForecastDay {
    pub day: String,
    #[serde(rename = "mintemperatureMin")]
    pub min_temperature_min: i32,
    #[serde(rename = "mintemperatureMax")]
    pub min_temperature_max: i32,
    #[serde(rename = "maxtemperatureMin")]
    pub max_temperature_min: i32,
    #[serde(rename = "maxtemperatureMax")]
    pub max_temperature_max: i32,
    #[serde(rename = "rainChance", default)]
    pub rain_chance: u8,
    /// Dutch `weatherdescription` exactly as published.
    #[serde(rename = "weatherdescription", default)]
    pub condition: String,
    /// Icon URL exactly as published, e.g. `.../30x30/f.png`.
    #[serde(rename = "iconurl", default)]
    pub icon_url: String,
    /// Buienradar icon code, e.g. `f`; empty when absent. Derived, never deserialized.
    #[serde(skip)]
    pub condition_code: String,
    /// English description for [`Self::condition_code`], falling back to Dutch.
    #[serde(skip)]
    pub condition_english: String,
    /// Forecast day projected into the viewer's timezone.
    #[serde(skip)]
    pub day_at: LocalTimestamp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WeatherData {
    pub current: Option<CurrentConditions>,
    pub forecast: Vec<ForecastDay>,
    pub now_line: String,
    pub rain_line: String,
    pub rain_soon: Option<RainSoon>,
    /// Complete two-hour raintext window, oldest sample first.
    pub rain_series: Vec<RainPoint>,
}

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
    #[serde(rename = "windspeedBft", default)]
    wind_bft: Option<i64>,
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

fn parse_feed(feed: &str, latitude: f64, longitude: f64) -> Result<WeatherData, WeatherError> {
    let payload: FeedPayload = serde_json::from_str(feed)?;
    let station = nearest_station(&payload.actual.stationmeasurements, latitude, longitude);
    let current = station.map(|value| {
        let code = condition::code_from_icon_url(&value.icon_url).unwrap_or_default();
        let dutch = value.condition.clone().unwrap_or_default();
        CurrentConditions {
            station_name: value.station_name.clone(),
            temperature: value.temperature,
            feel_temperature: value.feel_temperature,
            wind_speed: value.wind_speed,
            wind_bft: value.wind_bft,
            humidity: value.humidity,
            condition: value.condition.clone(),
            condition_english: condition::describe(&code, &dutch),
            condition_code: code,
            observed_at: LocalTimestamp::from_feed(&value.timestamp),
        }
    });
    let now_line = station.map_or_else(
        || String::from("Now: unavailable"),
        |value| {
            format!(
                "Now ({}): {}C feels {}C, wind {} m/s ({} bft), RH {}%",
                value.station_name,
                fmt_opt_f64(value.temperature),
                fmt_opt_f64(value.feel_temperature),
                fmt_opt_f64(value.wind_speed),
                fmt_opt_i64(value.wind_bft),
                fmt_opt_f64(value.humidity)
            )
        },
    );
    let mut forecast = payload.forecast.fivedayforecast;
    for day in &mut forecast {
        day.condition_code = condition::code_from_icon_url(&day.icon_url).unwrap_or_default();
        day.condition_english = condition::describe(&day.condition_code, &day.condition);
        day.day_at = LocalTimestamp::from_feed(&day.day);
    }
    Ok(WeatherData {
        current,
        forecast,
        now_line,
        rain_line: String::new(),
        rain_soon: None,
        rain_series: Vec::new(),
    })
}

fn nearest_station(
    stations: &[StationMeasurement],
    latitude: f64,
    longitude: f64,
) -> Option<&StationMeasurement> {
    stations.iter().min_by(|left, right| {
        haversine_distance(left, latitude, longitude)
            .total_cmp(&haversine_distance(right, latitude, longitude))
    })
}

fn haversine_distance(station: &StationMeasurement, latitude: f64, longitude: f64) -> f64 {
    let latitude_delta = (station.lat - latitude).to_radians();
    let longitude_delta = (station.lon - longitude).to_radians();
    let station_latitude = station.lat.to_radians();
    let target_latitude = latitude.to_radians();
    let a = (latitude_delta / 2.0).sin().powi(2)
        + station_latitude.cos() * target_latitude.cos() * (longitude_delta / 2.0).sin().powi(2);
    2.0 * a.sqrt().asin()
}

fn summarize_raintext_detail(raw: &str, clock_shift_minutes: i32) -> RainSeries {
    rain::parse_series(raw, clock_shift_minutes)
}

fn fmt_opt_f64(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("n/a"), |number| format!("{number:.1}"))
}

fn fmt_opt_i64(value: Option<i64>) -> String {
    value.map_or_else(|| String::from("n/a"), |number| number.to_string())
}

#[cfg(test)]
impl StationMeasurement {
    fn named_at(name: &str, lat: f64, lon: f64) -> Self {
        Self {
            station_name: name.to_owned(),
            lat,
            lon,
            temperature: None,
            feel_temperature: None,
            wind_speed: None,
            wind_bft: None,
            humidity: None,
            condition: None,
            icon_url: String::new(),
            timestamp: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_FEED_URL, DEFAULT_RAINTEXT_URL, ReqwestFeedApi, StationMeasurement,
        nearest_station, parse_feed, rain_code_to_mmh, summarize_raintext_detail,
    };

    const FEED_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/feed.json"
    ));
    const RAINTEXT_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/raintext.txt"
    ));

    #[test]
    fn raintext_dry_summary() {
        // Given
        let raw = "000|12:00\n000|12:05\n";

        // When
        let summary = summarize_raintext_detail(raw, 0);

        // Then
        assert_eq!(summary.line, "next 2h: dry");
        assert!(summary.rain_soon.is_none());
    }

    #[test]
    fn raintext_reports_first_and_peak() {
        // Given
        let raw = "000|12:00\n050|12:05\n080|12:10\n060|12:15\n";

        // When
        let summary = summarize_raintext_detail(raw, 0);

        // Then
        let rain = summary.rain_soon.expect("expected rain_soon");
        assert_eq!(rain.minutes_until, 5);
        assert_eq!(rain.time_label, "12:05");
        assert!(rain.max_mmh > 0.0);
    }

    #[test]
    fn nearest_station_prefers_smallest_distance() {
        // Given
        let stations = [
            StationMeasurement::named_at("Far", 51.0, 4.0),
            StationMeasurement::named_at("Near", 52.37, 4.84),
        ];

        // When
        let station = nearest_station(&stations, 52.37, 4.84).expect("station expected");

        // Then
        assert_eq!(station.station_name, "Near");
    }

    #[test]
    fn rain_code_conversion_monotonic() {
        // Given / When
        let low = rain_code_to_mmh(70);
        let high = rain_code_to_mmh(90);

        // Then
        assert!(high > low);
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
        assert_eq!(weather.forecast[0].min_temperature_max, 16);
        assert_eq!(weather.forecast[0].max_temperature_min, 20);
        assert_eq!(weather.forecast[0].max_temperature_max, 22);
    }

    #[test]
    fn live_fixtures_parse_offline() {
        // Given / When
        let weather = parse_feed(FEED_FIXTURE, 52.37, 4.84).expect("fixture should parse");
        let rain = summarize_raintext_detail(RAINTEXT_FIXTURE, 0);

        // Then
        assert_eq!(weather.forecast.len(), 5);
        assert!(!rain.line.is_empty());
    }

    #[test]
    fn client_accepts_owned_and_borrowed_endpoint_urls() {
        // Given / When
        let client = ReqwestFeedApi::new(DEFAULT_FEED_URL.to_owned(), DEFAULT_RAINTEXT_URL)
            .expect("client should build");

        // Then
        assert_eq!(client.feed_url, DEFAULT_FEED_URL);
        assert_eq!(client.raintext_url, DEFAULT_RAINTEXT_URL);
    }

    #[test]
    fn malformed_feed_returns_typed_decode_error() {
        // Given / When
        let result = parse_feed("not-json", 52.37, 4.84);

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn missing_station_values_are_summarized_as_unavailable() {
        // Given
        let feed = r#"{
            "actual":{"stationmeasurements":[{"stationname":"Sparse","lat":52.37,"lon":4.84}]},
            "forecast":{"fivedayforecast":[]}
        }"#;

        // When
        let weather = parse_feed(feed, 52.37, 4.84).expect("feed should parse");

        // Then
        assert_eq!(
            weather.now_line,
            "Now (Sparse): n/aC feels n/aC, wind n/a m/s (n/a bft), RH n/a%"
        );
    }

    #[test]
    fn current_conditions_carry_the_icon_code_and_english_text() {
        // Given / When
        let weather = parse_feed(FEED_FIXTURE, 52.07, 5.88).expect("fixture should parse");

        // Then
        let current = weather.current.expect("nearest station expected");
        assert_eq!(current.station_name, "Meetstation Arnhem");
        assert_eq!(current.condition_code, "qq");
        assert_eq!(current.condition_english, "Heavily clouded with rain");
        assert_eq!(current.condition.as_deref(), Some("Zwaar bewolkt en regen"));
        assert_eq!(current.observed_at.source, "2026-08-29T23:10:00");
        assert_eq!(current.observed_at.date_label.len(), 10);
    }

    #[test]
    fn forecast_days_carry_the_icon_code_and_english_text() {
        // Given / When
        let weather = parse_feed(FEED_FIXTURE, 52.37, 4.84).expect("fixture should parse");

        // Then
        let first = &weather.forecast[0];
        assert_eq!(first.condition_code, "f");
        assert_eq!(first.condition_english, "Partly cloudy with light rain");
        assert_eq!(first.day_at.source, "2026-08-30T00:00:00");
        for day in &weather.forecast {
            assert!(!day.condition_english.is_empty());
        }
    }

    #[test]
    fn unmapped_icon_code_falls_back_to_the_dutch_description() {
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
        assert_eq!(current.condition_english, "Zwaar bewolkt");
    }

    #[test]
    fn missing_icon_and_description_yield_a_neutral_condition_label() {
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
        assert_eq!(current.condition_english, "Unknown");
    }

    #[test]
    fn raintext_summary_exposes_the_whole_series_alongside_rain_soon() {
        // Given / When
        let summary = summarize_raintext_detail(RAINTEXT_FIXTURE, 0);

        // Then
        assert_eq!(summary.points.len(), 24);
        assert_eq!(summary.points[0].source_time_label, "23:25");
        assert_eq!(summary.points[1].mmh, rain_code_to_mmh(87));
        let rain = summary.rain_soon.expect("expected rain_soon");
        assert_eq!(rain.max_mmh, summary.peak_mmh);
    }
}
