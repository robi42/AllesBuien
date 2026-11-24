//! The Buienradar HTTP transport behind [`RadarApi`].
//!
//! Separating it from [`crate::radar`] keeps one question per module: this one
//! answers "how do bytes arrive", and `radar` answers "what is a run, and how
//! much of it do we keep". Only this half needs a network, so only this half has
//! to be swapped for a fake — which is precisely why the trait exists.
//!
//! Four vendor facts are encoded here and nowhere else. The metadata query
//! fixes the window of frames the radar downloads, the most the animation can
//! play, while how far into it the animation goes is [`crate::rain_reach`]'s;
//! the frame URLs it returns answer `302`, so the redirect-following client is
//! load-bearing rather than incidental; the run that query is answered from
//! is not reliably the current one, which is why asking is only half of getting;
//! and the size of each answer, measured and bounded here, so one far larger
//! than any the vendor has sent is refused rather than kept.

use jiff::Timestamp;

use crate::{
    bounded_body::read_bounded,
    radar::{RadarApi, RadarError, RadarFuture},
    radar_freshness::{RunFreshness, metadata_freshness},
};

/// The most one metadata answer may hold.
///
/// It measured 9 013 B for 32 frames on 24 November 2026, about 280 B a frame,
/// so this leaves room for a window many times longer than any the query asks
/// for; [`crate::radar::MAX_RUN_FRAMES`] cuts such a window, and this refuses
/// an answer whose size no listing of frames explains.
const MAX_METADATA_BYTES: usize = 256 * 1024;

/// The most one frame may hold.
///
/// Frames measured 94 KiB when this transport was written, and from 19 958 to
/// 34 979 B over the 128 frames of four runs on 24 November 2026. Those frames
/// were 2.6 to 3.6 % rain at 0.8 to 1.2 B a rain pixel, so a 1058×915 frame
/// that is rain from edge to edge comes to about 1.1 MB. Twice that is still a
/// frame; more is not one this query has ever answered with.
const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;

/// Observed steps requested first, at one frame per five minutes.
///
/// Two of them give the animation about ten minutes of past — enough for a
/// shower to read as moving rather than as a still that jumps into forecasts —
/// and with [`FORECAST_STEPS`] the response spans 32 frames, from five minutes
/// before the run to two and a half hours after it.
///
/// The window is also [`crate::radar::MAX_RUN_FRAMES`], the most frames a run
/// accepts. At the 94 KiB frames on record, [`crate::radar::MAX_CACHED_RUNS`] ×
/// 32 is about 9 MiB: the run cap dropped 6 → 3 when the window grew to 26
/// frames, to stay near the previous window's 5.5 MiB, and the six frames
/// [`FORECAST_STEPS`] added since cost the other 1.7 MiB. Frames measured 20 to
/// 35 KB in November 2026, which makes it nearer 3 MiB.
const PREFERRED_HISTORY: u8 = 2;

/// Predicted steps requested, at the same cadence: two and a half hours, half an
/// hour more than the rain graph beside the map covers.
///
/// The surplus is what lets the radar reach that graph's last bar, because the
/// two windows hang from different clocks. Raintext's 24 samples start at a
/// five-minute slot within a few minutes of the present, usually the nearest
/// one, and end 115 minutes later. These frames end this many steps after the
/// run, and the run trails the present: by five to fifteen minutes when healthy,
/// and by up to [`crate::radar_freshness::STALE_RUN_THRESHOLD_MINUTES`] before it
/// is refused. At 24 steps the two windows ended together only behind a run
/// about five minutes old. On 13 October at 10:01 the run was eleven and a half
/// minutes old, so the radar ended at 11:50 against raintext's 11:55, and no
/// frame put the graph's cursor on its last bar.
///
/// Thirty steps let raintext's first sample fall up to 35 minutes after the run.
/// That covers a run trailing by up to 25 minutes even when raintext starts a
/// whole slot ahead of the present and the radar was fetched a whole five-minute
/// refresh before it: ten minutes past the healthy worst, and five short of the
/// threshold. The frames past raintext's last sample are downloaded but not
/// played: [`crate::rain_reach`] ends the animation on the frame at that
/// sample, and the next raintext refresh, which moves it later, brings them back
/// from memory. Every new run still downloads six frames more. The vendor serves
/// at most 36 steps.
const FORECAST_STEPS: u8 = 30;

/// Observed steps requested after a stale answer, and only then.
///
/// `history=0` is the one window measured to come from the run the vendor is
/// currently publishing, so it is the escape hatch from an archived answer. It
/// is emphatically not the default: it has no past at all, and standing on it
/// permanently trades an intermittent fault for a constant one.
const CURRENT_RUN_HISTORY: u8 = 0;

fn metadata_path(history: u8) -> String {
    format!(
        "/2.0/metadata/sprite/RadarMapRainWebmercatorNL?width=1058&height=915&extension=png&renderBackground=false&renderText=false&renderBranding=false&history={history}&forecast={FORECAST_STEPS}&skip=0"
    )
}

/// The two sprite-metadata windows, resolved against one base URL.
struct MetadataUrls {
    with_history: String,
    current_run: String,
}

impl MetadataUrls {
    fn under(base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/');
        Self {
            with_history: format!("{base}{}", metadata_path(PREFERRED_HISTORY)),
            current_run: format!("{base}{}", metadata_path(CURRENT_RUN_HISTORY)),
        }
    }
}

/// The single operation the stale-run fallback needs from a transport.
///
/// It exists so the fallback can be driven by a fake without a socket while
/// [`RadarApi`] keeps its shape — the doubles in `allesbuien-testkit` and the
/// GUI harnesses implement that one, and widening it to expose this retry would
/// make every one of them carry a seam only the live transport has.
trait BytesSource: Sync {
    fn get<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>>;
}

/// Fetches the windowed metadata, and refetches the current run if that answer
/// is provably stale.
///
/// Exactly one fallback attempt, by construction rather than by a counter: the
/// second response is returned whatever it says, so this cannot loop and cannot
/// spend more than one extra request. It is the vendor's freshest window, which
/// is the best available answer rather than a guaranteed fresh one. A healthy
/// answer costs nothing extra — freshness is read from bytes already in hand.
async fn metadata_of_a_current_run<S: BytesSource>(
    source: &S,
    urls: &MetadataUrls,
    now: Timestamp,
) -> Result<Vec<u8>, RadarError> {
    let answered = source.get(&urls.with_history).await?;
    match metadata_freshness(&answered, now) {
        RunFreshness::Stale { .. } => source.get(&urls.current_run).await,
        RunFreshness::Current | RunFreshness::Undecidable => Ok(answered),
    }
}

/// Async Buienradar transport.
///
/// Reqwest's default redirect policy (up to ten hops) is deliberately retained:
/// metadata frame URLs return 302 responses to `image-cdn.buienradar.nl`.
pub struct ReqwestRadarApi {
    client: reqwest::Client,
    urls: MetadataUrls,
}

impl ReqwestRadarApi {
    /// Builds a transport for the supplied base URL.
    ///
    /// # Errors
    /// Returns [`RadarError::Request`] if the HTTP client cannot be built.
    pub fn new(base_url: &str) -> Result<Self, RadarError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(crate::config::USER_AGENT)
            .build()?;
        Ok(Self {
            client,
            urls: MetadataUrls::under(base_url),
        })
    }

    async fn send(&self, url: &str) -> Result<reqwest::Response, RadarError> {
        Ok(self.client.get(url).send().await?)
    }
}

/// A metadata answer's body, refused past [`MAX_METADATA_BYTES`].
async fn metadata_body(response: reqwest::Response) -> Result<Vec<u8>, RadarError> {
    body_within(response, MAX_METADATA_BYTES).await
}

/// A frame's body, refused past [`MAX_FRAME_BYTES`].
async fn frame_body(response: reqwest::Response) -> Result<Vec<u8>, RadarError> {
    body_within(response, MAX_FRAME_BYTES).await
}

/// A successful answer's body, read no further than `limit` bytes.
async fn body_within(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, RadarError> {
    let status = response.status();
    if !status.is_success() {
        return Err(RadarError::HttpStatus {
            status: status.as_u16(),
        });
    }
    Ok(read_bounded(response, limit).await?)
}

/// The metadata requests, both windows of them, are the ones this source sends.
impl BytesSource for ReqwestRadarApi {
    fn get<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async move { metadata_body(self.send(url).await?).await })
    }
}

impl RadarApi for ReqwestRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(metadata_of_a_current_run(
            self,
            &self.urls,
            Timestamp::now(),
        ))
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async move { frame_body(self.send(url).await?).await })
    }
}

#[cfg(test)]
mod tests;
