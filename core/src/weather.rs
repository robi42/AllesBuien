// allow: SIZE_OK — task contract requires the HTTP adapter, wire models, derivation, and inline tests in this module.
use std::{future::Future, pin::Pin, time::Duration};

use serde::Deserialize;
use thiserror::Error;

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
            let rain = summarize_raintext_detail(&raintext);
            Ok(WeatherData {
                current: parsed.current,
                forecast: parsed.forecast,
                now_line: parsed.now_line,
                rain_line: rain.line,
                rain_soon: rain.rain_soon,
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

#[derive(Debug, Clone, PartialEq)]
pub struct RainSoon {
    pub minutes_until: u32,
    pub time_label: String,
    pub max_mmh: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentConditions {
    pub station_name: String,
    pub temperature: Option<f64>,
    pub feel_temperature: Option<f64>,
    pub wind_speed: Option<f64>,
    pub wind_bft: Option<i64>,
    pub humidity: Option<f64>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    #[serde(rename = "weatherdescription", default)]
    pub condition: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherData {
    pub current: Option<CurrentConditions>,
    pub forecast: Vec<ForecastDay>,
    pub now_line: String,
    pub rain_line: String,
    pub rain_soon: Option<RainSoon>,
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

#[derive(Debug, PartialEq)]
struct RainSummary {
    line: String,
    rain_soon: Option<RainSoon>,
}

fn parse_feed(feed: &str, latitude: f64, longitude: f64) -> Result<WeatherData, WeatherError> {
    let payload: FeedPayload = serde_json::from_str(feed)?;
    let station = nearest_station(&payload.actual.stationmeasurements, latitude, longitude);
    let current = station.map(|value| CurrentConditions {
        station_name: value.station_name.clone(),
        temperature: value.temperature,
        feel_temperature: value.feel_temperature,
        wind_speed: value.wind_speed,
        wind_bft: value.wind_bft,
        humidity: value.humidity,
        condition: value.condition.clone(),
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
    Ok(WeatherData {
        current,
        forecast: payload.forecast.fivedayforecast,
        now_line,
        rain_line: String::new(),
        rain_soon: None,
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

fn summarize_raintext_detail(raw: &str) -> RainSummary {
    let mut first_rain = None;
    let mut first_rain_minutes = None;
    let mut peak = (0, String::new());
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split('|');
        let code = parts.next().and_then(|part| part.parse::<i32>().ok());
        let time = parts.next().unwrap_or("?").to_owned();
        if let Some(code) = code {
            if code > 0 && first_rain.is_none() {
                first_rain = Some(time.clone());
                first_rain_minutes = Some(
                    u32::try_from(index).map_or(u32::MAX, |minutes| minutes.saturating_mul(5)),
                );
            }
            if code > peak.0 {
                peak = (code, time);
            }
        }
    }
    if peak.0 <= 0 {
        return RainSummary {
            line: String::from("next 2h: dry"),
            rain_soon: None,
        };
    }
    let max_mmh = rain_code_to_mmh(peak.0);
    let first_label = first_rain.unwrap_or_else(|| String::from("soon"));
    let peak_label = if peak.1.is_empty() { "?" } else { &peak.1 };
    RainSummary {
        line: format!(
            "starts around {}, max {:.2} mm/h at {}",
            first_label, max_mmh, peak_label
        ),
        rain_soon: Some(RainSoon {
            minutes_until: first_rain_minutes.unwrap_or(0),
            time_label: first_label,
            max_mmh,
        }),
    }
}

fn rain_code_to_mmh(code: i32) -> f64 {
    10_f64.powf((f64::from(code) - 109.0) / 32.0)
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
        let summary = summarize_raintext_detail(raw);

        // Then
        assert_eq!(summary.line, "next 2h: dry");
        assert!(summary.rain_soon.is_none());
    }

    #[test]
    fn raintext_reports_first_and_peak() {
        // Given
        let raw = "000|12:00\n050|12:05\n080|12:10\n060|12:15\n";

        // When
        let summary = summarize_raintext_detail(raw);

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
        let rain = summarize_raintext_detail(RAINTEXT_FIXTURE);

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
}
