//! The Buienradar HTTP transport behind [`RadarApi`].
//!
//! Separating it from [`crate::radar`] keeps one question per module: this one
//! answers "how do bytes arrive", and `radar` answers "what is a run, and how
//! much of it do we keep". Only this half needs a network, so only this half has
//! to be swapped for a fake — which is precisely why the trait exists.
//!
//! Three vendor facts are encoded here and nowhere else. The metadata query
//! fixes the animation window; the frame URLs it returns answer `302`, so the
//! redirect-following client is load-bearing rather than incidental; and the run
//! that query is answered from is not reliably the current one, which is why
//! asking is only half of getting.

use jiff::Timestamp;

use crate::{
    radar::{RadarApi, RadarError, RadarFuture},
    radar_freshness::{RunFreshness, metadata_freshness},
};

/// Observed steps requested first, at one frame per five minutes.
///
/// Two of them give the animation about ten minutes of past — enough for a
/// shower to read as moving rather than as a still that jumps into forecasts —
/// and with [`FORECAST_STEPS`] the response spans 32 frames, from five minutes
/// before the run to two and a half hours after it.
///
/// Frames measure 94 KiB, so [`crate::radar::MAX_CACHED_RUNS`] × 32 is about
/// 9 MiB: the run cap dropped 6 → 3 when the window grew to 26 frames, to stay
/// near the previous window's 5.5 MiB, and the six frames [`FORECAST_STEPS`]
/// added since cost the other 1.7 MiB.
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
/// threshold. The frames past raintext's last sample show no cursor, as
/// [`crate::rain_cursor`] already arranges for the observed ones before its
/// first, and every new run downloads six frames more. The vendor serves at most
/// 36 steps.
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

    async fn fetch(&self, url: &str) -> Result<Vec<u8>, RadarError> {
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

impl BytesSource for ReqwestRadarApi {
    fn get<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(self.fetch(url))
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
        Box::pin(self.fetch(url))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{BytesSource, MetadataUrls, ReqwestRadarApi, metadata_of_a_current_run};
    use crate::{
        config::DEFAULT_RADAR_URL,
        radar::RadarFuture,
        test_support::{block_on, sample_instant},
    };

    const STALE_RUN: &[u8] = br#"{"timestamp":"2026-09-02T12:18:00","timeOffset":2,"times":[]}"#;
    const CURRENT_RUN: &[u8] = br#"{"timestamp":"2026-09-02T13:45:00","timeOffset":2,"times":[]}"#;

    /// Answers each window with a canned body and records what was asked for.
    struct StubSource {
        with_history: &'static [u8],
        requested: Mutex<Vec<String>>,
    }

    impl StubSource {
        fn answering(with_history: &'static [u8]) -> Self {
            Self {
                with_history,
                requested: Mutex::new(Vec::new()),
            }
        }

        fn requested(&self) -> Vec<String> {
            self.requested.lock().expect("stub lock").clone()
        }
    }

    impl BytesSource for StubSource {
        fn get<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
            Box::pin(async move {
                self.requested
                    .lock()
                    .expect("stub lock")
                    .push(url.to_owned());
                if url.contains("history=0") {
                    Ok(CURRENT_RUN.to_vec())
                } else {
                    Ok(self.with_history.to_vec())
                }
            })
        }
    }

    fn answer_for(with_history: &'static [u8]) -> (Vec<u8>, Vec<String>) {
        let stub = StubSource::answering(with_history);
        let urls = MetadataUrls::under("https://image.example");
        let body = block_on(metadata_of_a_current_run(&stub, &urls, sample_instant()))
            .expect("the stub never fails");
        (body, stub.requested())
    }

    #[test]
    fn a_stale_answer_is_replaced_by_the_current_run() {
        // Given: the endpoint answers the windowed query from an archived run.
        // When
        let (body, requested) = answer_for(STALE_RUN);

        // Then: the second request drops history, and its body is what returns.
        assert_eq!(body, CURRENT_RUN);
        assert_eq!(requested.len(), 2);
        assert!(requested[0].contains("history=2"), "{}", requested[0]);
        assert!(requested[1].contains("history=0"), "{}", requested[1]);
    }

    #[test]
    fn a_current_answer_is_returned_without_a_second_request() {
        // Given: the endpoint answers the windowed query from the current run.
        // When
        let (body, requested) = answer_for(CURRENT_RUN);

        // Then
        assert_eq!(body, CURRENT_RUN);
        assert_eq!(requested.len(), 1);
        assert!(requested[0].contains("history=2"), "{}", requested[0]);
    }

    #[test]
    fn an_undatable_answer_is_kept_rather_than_refetched() {
        // Given: a body the freshness reader cannot date.
        // When
        let (body, requested) = answer_for(b"not the sprite schema");

        // Then
        assert_eq!(body, b"not the sprite schema");
        assert_eq!(requested.len(), 1);
    }

    #[test]
    fn a_trailing_slash_on_the_base_url_does_not_double_the_path_separator() {
        // Given / When
        let plain = ReqwestRadarApi::new("https://image.example").expect("client should build");
        let trailing = ReqwestRadarApi::new("https://image.example/").expect("client should build");

        // Then
        assert_eq!(plain.urls.with_history, trailing.urls.with_history);
        assert_eq!(plain.urls.current_run, trailing.urls.current_run);
        assert!(
            plain
                .urls
                .with_history
                .starts_with("https://image.example/2.0/"),
            "{}",
            plain.urls.with_history
        );
    }

    #[test]
    fn the_metadata_query_asks_for_a_window_that_outlasts_the_rain_graph() {
        // Given / When
        let api = ReqwestRadarApi::new(DEFAULT_RADAR_URL).expect("client should build");

        // Then: two observed steps back, two and a half hours ahead, and bare
        // sprites. The fallback differs in exactly one parameter, so a stale
        // answer costs the history and nothing else.
        for parameter in [
            "history=2",
            "forecast=30",
            "renderBackground=false",
            "renderText=false",
            "renderBranding=false",
        ] {
            assert!(
                api.urls.with_history.contains(parameter),
                "missing {parameter} in {}",
                api.urls.with_history
            );
        }
        assert_eq!(
            api.urls.current_run,
            api.urls.with_history.replace("history=2", "history=0")
        );
    }
}
