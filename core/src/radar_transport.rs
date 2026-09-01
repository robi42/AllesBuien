//! The Buienradar HTTP transport behind [`RadarApi`].
//!
//! Separating it from [`crate::radar`] keeps one question per module: this one
//! answers "how do bytes arrive", and `radar` answers "what is a run, and how
//! much of it do we keep". Only this half needs a network, so only this half has
//! to be swapped for a fake — which is precisely why the trait exists.
//!
//! Two vendor facts are encoded here and nowhere else. The metadata query fixes
//! the animation window, and the frame URLs it returns answer `302`, so the
//! redirect-following client is load-bearing rather than incidental.

use crate::radar::{RadarApi, RadarError, RadarFuture};

pub const DEFAULT_RADAR_BASE_URL: &str = "https://image.buienradar.nl";

/// One frame per 5 minutes, so this window returns 26 frames spanning 5 minutes
/// back to 2 hours ahead — the same horizon the raintext rain graph covers.
///
/// Future-weighted on purpose: a run timestamp trails the wall clock by about 10
/// minutes, so the earlier `history=6&forecast=4` reached only 7 minutes past
/// now. Raising `history` cannot help — it saturates at 16 frames, measured
/// identical for 12 and 24.
///
/// Frames measure 94 KiB, so [`crate::radar::MAX_CACHED_RUNS`] × 26 is about
/// 7 MiB; the run cap dropped 6 → 3 to stay near the previous window's 5.5 MiB.
const METADATA_PATH: &str = "/2.0/metadata/sprite/RadarMapRainWebmercatorNL?width=1058&height=915&extension=png&renderBackground=false&renderText=false&renderBranding=false&history=2&forecast=24&skip=0";

/// Async Buienradar transport.
///
/// Reqwest's default redirect policy (up to ten hops) is deliberately retained:
/// metadata frame URLs return 302 responses to `image-cdn.buienradar.nl`.
pub struct ReqwestRadarApi {
    client: reqwest::Client,
    metadata_url: String,
}

impl ReqwestRadarApi {
    /// Builds a transport for the supplied base URL.
    ///
    /// # Errors
    /// Returns [`RadarError::Request`] if the HTTP client cannot be built.
    pub fn new(base_url: &str) -> Result<Self, RadarError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self {
            client,
            metadata_url: format!("{}{METADATA_PATH}", base_url.trim_end_matches('/')),
        })
    }

    async fn get(&self, url: &str) -> Result<Vec<u8>, RadarError> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(RadarError::HttpStatus {
                status: status.as_u16(),
            });
        }
        Ok(response.bytes().await?.to_vec())
    }
}

impl RadarApi for ReqwestRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(self.get(&self.metadata_url))
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(self.get(url))
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_RADAR_BASE_URL, ReqwestRadarApi};

    #[test]
    fn a_trailing_slash_on_the_base_url_does_not_double_the_path_separator() {
        // Given / When
        let plain = ReqwestRadarApi::new("https://image.example").expect("client should build");
        let trailing = ReqwestRadarApi::new("https://image.example/").expect("client should build");

        // Then
        assert_eq!(plain.metadata_url, trailing.metadata_url);
        assert!(
            plain.metadata_url.starts_with("https://image.example/2.0/"),
            "{}",
            plain.metadata_url
        );
    }

    #[test]
    fn the_metadata_query_asks_for_the_window_the_rain_graph_covers() {
        // Given / When
        let api = ReqwestRadarApi::new(DEFAULT_RADAR_BASE_URL).expect("client should build");

        // Then: two observed steps back, two hours ahead, and bare sprites.
        for parameter in [
            "history=2",
            "forecast=24",
            "renderBackground=false",
            "renderText=false",
            "renderBranding=false",
        ] {
            assert!(
                api.metadata_url.contains(parameter),
                "missing {parameter} in {}",
                api.metadata_url
            );
        }
    }
}
