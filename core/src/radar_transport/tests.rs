//! Offline coverage for the radar transport: the stale-run fallback driven
//! through a stub source, the two metadata windows, and the limit each kind of
//! answer is read under.
//!
//! Answers are built in memory and handed to the same per-kind readers the
//! transport calls, so a limit is pinned to its kind of answer without a
//! socket.

use std::sync::Mutex;

use super::{
    BytesSource, CURRENT_RUN_HISTORY, FORECAST_STEPS, MAX_FRAME_BYTES, MAX_METADATA_BYTES,
    MetadataUrls, PREFERRED_HISTORY, ReqwestRadarApi, frame_body, metadata_body,
    metadata_of_a_current_run,
};
use crate::{
    config::DEFAULT_RADAR_URL,
    radar::{MAX_RUN_FRAMES, RadarError, RadarFuture},
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

#[test]
fn a_run_accepts_the_preferred_window_and_the_fallback_asks_for_no_more() {
    // Given / When
    let preferred = usize::from(PREFERRED_HISTORY + FORECAST_STEPS);
    let fallback = usize::from(CURRENT_RUN_HISTORY + FORECAST_STEPS);

    // Then
    assert_eq!(preferred, MAX_RUN_FRAMES);
    assert!(fallback <= MAX_RUN_FRAMES, "{fallback}");
}

/// An answer of `length` bytes that declares no length, so a reader can
/// only refuse it as it streams.
fn answered(status: u16, length: usize) -> reqwest::Response {
    http::Response::builder()
        .status(status)
        .body(vec![b'7'; length])
        .expect("an in-memory response")
        .into()
}

#[test]
fn each_answer_is_refused_one_byte_past_its_own_limit() {
    // Given / When
    let metadata = block_on(metadata_body(answered(200, MAX_METADATA_BYTES + 1)));
    let frame = block_on(frame_body(answered(200, MAX_FRAME_BYTES + 1)));

    // Then: each carries its own limit, so the two cannot trade places.
    assert!(
        matches!(
            metadata,
            Err(RadarError::BodyTooLarge {
                limit: MAX_METADATA_BYTES
            })
        ),
        "{metadata:?}"
    );
    assert!(
        matches!(
            frame,
            Err(RadarError::BodyTooLarge {
                limit: MAX_FRAME_BYTES
            })
        ),
        "{frame:?}"
    );
}

#[test]
fn each_answer_at_its_own_limit_is_returned_whole() {
    // Given / When
    let metadata = block_on(metadata_body(answered(200, MAX_METADATA_BYTES)));
    let frame = block_on(frame_body(answered(200, MAX_FRAME_BYTES)));

    // Then
    assert_eq!(
        metadata.expect("metadata at its limit").len(),
        MAX_METADATA_BYTES
    );
    assert_eq!(frame.expect("a frame at its limit").len(), MAX_FRAME_BYTES);
}

#[test]
fn a_refused_status_is_still_reported_before_any_body_is_read() {
    // Given / When
    let metadata = block_on(metadata_body(answered(503, MAX_METADATA_BYTES + 1)));
    let frame = block_on(frame_body(answered(503, MAX_FRAME_BYTES + 1)));

    // Then
    for answer in [metadata, frame] {
        assert!(
            matches!(answer, Err(RadarError::HttpStatus { status: 503 })),
            "{answer:?}"
        );
    }
}
