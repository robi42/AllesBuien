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
    rain::parse_series,
    weather_feed::parse_feed,
};

pub use crate::rain::{RainPoint, RainSoon};

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
            .user_agent(crate::config::USER_AGENT)
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
            let rain = parse_series(&raintext, clock_shift_minutes());
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
    pub humidity: Option<f64>,
    /// Dutch `weatherdescription` exactly as published.
    pub condition: Option<String>,
    /// Buienradar icon code, e.g. `q` or the night variant `qq`; empty when absent.
    pub condition_code: String,
    /// Observation time projected into the viewer's timezone.
    pub observed_at: LocalTimestamp,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ForecastDay {
    pub day: String,
    /// The lowest the day's minimum is expected to fall. The feed also bounds
    /// the minimum from above and the maximum from below, `mintemperatureMax`
    /// and `maxtemperatureMin`; the forecast shows only the outer bounds, so
    /// those two are not read.
    #[serde(rename = "mintemperatureMin")]
    pub min_temperature_min: i32,
    /// The highest the day's maximum is expected to rise.
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

#[cfg(test)]
mod tests {
    use super::{ReqwestFeedApi, parse_feed, parse_series};
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
    fn live_fixtures_parse_offline() {
        // Given / When
        let weather = parse_feed(FEED_FIXTURE, 52.37, 4.84).expect("fixture should parse");
        let rain = parse_series(RAINTEXT_FIXTURE, 0);

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
}
