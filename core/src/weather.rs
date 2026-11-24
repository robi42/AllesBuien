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
    bounded_body::{BodyError, read_bounded},
    datetime::{LocalTimestamp, clock_shift_minutes},
    rain::parse_series,
    weather_feed::parse_feed,
};

pub use crate::rain::{RainPoint, RainSoon};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// The most one feed answer may hold.
///
/// It measured 35 390 B on 24 November 2026, and the capture under
/// `tests/fixtures/feed.json` is 33 287 B: every station in the country and a
/// five-day forecast. Thirty times that is still a feed; more is not one the
/// vendor has ever sent.
const MAX_FEED_BYTES: usize = 1024 * 1024;

/// The most one rain expectation may hold.
///
/// It is 24 lines of `value|HH:MM`, 240 B as measured on 24 November 2026, so
/// this leaves room for a series many times longer than two hours.
const MAX_RAINTEXT_BYTES: usize = 16 * 1024;

pub type FeedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<WeatherData, WeatherError>> + Send + 'a>>;

pub trait FeedApi: Send + Sync {
    fn fetch(&self, latitude: f64, longitude: f64) -> FeedFuture<'_>;
}

/// The Buienradar transport behind [`FeedApi`], with a client for each request.
///
/// The rain request is the one that carries the viewer's coordinates, so it is
/// sent on a client of its own, held to HTTPS whenever the rain address is
/// HTTPS. reqwest checks that switch on the first request and on every redirect
/// target, and the shipped address answers with a redirect, so the forward can
/// never take the coordinates below HTTPS: a forward to plain HTTP fails the
/// request instead. reqwest's default redirect policy, ten hops at most, is kept
/// on both clients, and it is what follows that forward.
///
/// The floor follows the rain address alone. Only a plain-HTTP override of that
/// address, which is how a local stub is reached, lifts it; the feed's address
/// has no say, because the feed, like the radar and the icons, sends no location
/// and keeps the client it always had.
#[derive(Debug, Clone)]
pub struct ReqwestFeedApi {
    feed_client: reqwest::Client,
    rain_client: reqwest::Client,
    feed_url: String,
    raintext_url: String,
}

impl ReqwestFeedApi {
    /// Builds a transport for the supplied feed and raintext endpoints.
    ///
    /// # Errors
    /// Returns [`WeatherError::ClientBuild`] if either HTTP client cannot be built.
    pub fn new(
        feed_url: impl Into<String>,
        raintext_url: impl Into<String>,
    ) -> Result<Self, WeatherError> {
        let raintext_url = raintext_url.into();
        Ok(Self {
            feed_client: client(false)?,
            rain_client: client(is_https(&raintext_url))?,
            feed_url: feed_url.into(),
            raintext_url,
        })
    }

    /// The rain request for a location, on the rain client and not yet sent:
    /// the coordinates go in the query at four decimal places, about 11 metres.
    fn raintext_request(&self, latitude: f64, longitude: f64) -> reqwest::RequestBuilder {
        self.rain_client
            .get(&self.raintext_url)
            .query(&[
                ("lat", format!("{latitude:.4}")),
                ("lon", format!("{longitude:.4}")),
            ])
            .timeout(REQUEST_TIMEOUT)
    }
}

/// One HTTP client as both requests want it, refusing every plain-HTTP address,
/// redirect targets included, when `https_only` is set.
fn client(https_only: bool) -> Result<reqwest::Client, WeatherError> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(crate::config::USER_AGENT)
        .https_only(https_only)
        .build()
        .map_err(WeatherError::ClientBuild)
}

/// Whether `address` is an HTTPS URL, which is when its client is held to HTTPS.
/// An address that does not parse is not, and its requests fail on their own.
fn is_https(address: &str) -> bool {
    reqwest::Url::parse(address).is_ok_and(|url| url.scheme() == "https")
}

impl FeedApi for ReqwestFeedApi {
    fn fetch(&self, latitude: f64, longitude: f64) -> FeedFuture<'_> {
        Box::pin(async move {
            let feed = self
                .feed_client
                .get(&self.feed_url)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(WeatherError::FeedRequest)?;
            let feed = feed_text(feed).await?;
            let raintext = self
                .raintext_request(latitude, longitude)
                .send()
                .await
                .map_err(WeatherError::RaintextRequest)?;
            let raintext = raintext_text(raintext).await?;
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

/// The feed's body as text, refused past [`MAX_FEED_BYTES`].
async fn feed_text(response: reqwest::Response) -> Result<String, WeatherError> {
    body_text(
        response,
        MAX_FEED_BYTES,
        "weather feed",
        WeatherError::FeedRequest,
    )
    .await
}

/// The rain expectation's body as text, refused past [`MAX_RAINTEXT_BYTES`].
async fn raintext_text(response: reqwest::Response) -> Result<String, WeatherError> {
    body_text(
        response,
        MAX_RAINTEXT_BYTES,
        "raintext",
        WeatherError::RaintextRequest,
    )
    .await
}

/// A successful answer's body, read no further than `limit` bytes, as text.
///
/// The bytes become text the way reqwest's `Response::text` makes them without
/// its `charset` feature, which this workspace does not enable, so the text is
/// what it was before the limit: invalid UTF-8 becomes the replacement
/// character. A failed status or read keeps its request's own variant, `failed`.
async fn body_text(
    response: reqwest::Response,
    limit: usize,
    name: &'static str,
    failed: fn(reqwest::Error) -> WeatherError,
) -> Result<String, WeatherError> {
    let response = response.error_for_status().map_err(failed)?;
    match read_bounded(response, limit).await {
        Ok(body) => Ok(String::from_utf8_lossy(&body).into_owned()),
        Err(BodyError::TooLarge { limit }) => Err(WeatherError::BodyTooLarge {
            response: name,
            limit,
        }),
        Err(BodyError::Transport(error)) => Err(failed(error)),
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
    #[error("{response} response exceeded its {limit}-byte limit")]
    BodyTooLarge {
        response: &'static str,
        limit: usize,
    },
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
mod tests;
