//! Current conditions, the five-day forecast, and the two-hour rain window.
//!
//! Two Buienradar endpoints answer the one question a viewer asks: the JSON feed
//! carries observations and the forecast, and `raintext` carries the minute series
//! behind them. [`FeedApi`] is where that pair is fetched together and where the
//! rest of the app stops having to know it is two requests — a refresh yields one
//! [`WeatherData`] or one [`WeatherError`], never a half-filled panel.
//!
//! The trait exists so controllers and their tests can be handed a deterministic
//! fake; [`ReqwestFeedApi`] is the only implementation that reaches the network.
//! Reading the JSON itself belongs to [`crate::weather_feed`], which owns the
//! vendor's wire shape, and reading `raintext` belongs to [`crate::rain`].
use std::{future::Future, pin::Pin, time::Duration};

use serde::Deserialize;
use thiserror::Error;

use crate::{
    datetime::{LocalTimestamp, clock_shift_minutes},
    rain::{self, RainSeries},
    weather_feed::parse_feed,
};

pub use crate::rain::{RainPoint, RainSoon, code_to_mmh as rain_code_to_mmh};

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
    /// Builds a transport for the supplied feed and raintext endpoints.
    ///
    /// # Errors
    /// Returns [`WeatherError::ClientBuild`] if the HTTP client cannot be built.
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
    /// Where the reading was measured: the feed's Dutch `"Meetstation
    /// Schiphol"` is reduced to `"Schiphol"` while the feed is parsed, so the
    /// Dutch form never reaches here. Names without that prefix arrive
    /// unchanged. `crate::station` argues why the word is dropped rather than
    /// translated.
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
    pub rain_line: String,
    pub rain_soon: Option<RainSoon>,
    /// Complete two-hour raintext window, oldest sample first.
    pub rain_series: Vec<RainPoint>,
}

/// Names this module's use of the raintext parser, so a refresh reads as one step.
fn summarize_raintext_detail(raw: &str, clock_shift_minutes: i32) -> RainSeries {
    rain::parse_series(raw, clock_shift_minutes)
}

#[cfg(test)]
mod tests {
    use super::{ReqwestFeedApi, parse_feed, rain_code_to_mmh, summarize_raintext_detail};
    use crate::config::{DEFAULT_FEED_URL, DEFAULT_RAINTEXT_URL};

    const FEED_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/feed.json"
    ));
    const RAINTEXT_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/raintext.txt"
    ));

    #[test]
    fn raintext_reports_a_dry_window_when_every_sample_is_zero() {
        // Given
        let raw = "000|12:00\n000|12:05\n";

        // When
        let summary = summarize_raintext_detail(raw, 0);

        // Then
        assert_eq!(summary.line, "next 2h: dry");
        assert!(summary.rain_soon.is_none());
    }

    #[test]
    fn raintext_reports_the_first_wet_minute_and_the_peak_intensity() {
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
    fn rain_code_conversion_rises_with_the_code_value() {
        // Given / When
        let low = rain_code_to_mmh(70);
        let high = rain_code_to_mmh(90);

        // Then
        assert!(high > low);
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
    #[expect(
        clippy::float_cmp,
        reason = "the summary must carry the conversion's own result and its own peak through unchanged, so identity is the assertion; a tolerance would let a rounded or recomputed value pass"
    )]
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
