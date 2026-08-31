//! Offline coverage for the radar metadata fields the app reads.
//!
//! The fixture is a verbatim capture of the live `history=6&forecast=4` sprite
//! metadata response, so a vendor schema change surfaces here rather than as a
//! two-hour caption drift on someone's desktop.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use allesbuien_core::{
    radar::{GenerationId, RadarApi, RadarClient, RadarFuture},
    radar_timeline::RadarTimeline,
};
use tempfile::tempdir;

const LIVE_METADATA: &[u8] = include_bytes!("fixtures/radar-metadata.json");

const METADATA_WITHOUT_OFFSET: &[u8] = br#"{
    "timestamp":"2026-08-31T18:55:00",
    "times":[{"timestamp":"2026-08-31T18:30:00","url":"fake://frame-0"}]
}"#;

struct FixtureRadarApi(&'static [u8]);

impl RadarApi for FixtureRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async { Ok(self.0.to_vec()) })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async { Ok(b"png".to_vec()) })
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    loop {
        match Pin::new(&mut future).poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn refresh(metadata: &'static [u8]) -> allesbuien_core::radar::RadarRefresh {
    let state = tempdir().expect("temp state");
    let client = RadarClient::new(
        Arc::new(FixtureRadarApi(metadata)),
        state.path().to_path_buf(),
    );
    block_on(client.refresh(GenerationId(1))).expect("fixture refresh should succeed")
}

#[test]
fn the_live_capture_carries_the_vendor_utc_offset_and_ten_frames() {
    // Given / When
    let refreshed = refresh(LIVE_METADATA);

    // Then
    assert_eq!(refreshed.time_offset_hours, Some(2));
    assert_eq!(refreshed.frames.len(), 10);
    assert_eq!(refreshed.frames[0].timestamp, "2026-08-31T18:30:00");
    assert_eq!(refreshed.frames[9].timestamp, "2026-08-31T19:15:00");
}

#[test]
fn metadata_without_the_vendor_field_reports_no_offset() {
    // Given / When
    let refreshed = refresh(METADATA_WITHOUT_OFFSET);

    // Then
    assert_eq!(refreshed.time_offset_hours, None);
    assert_eq!(refreshed.frames.len(), 1);
}

#[test]
fn the_captured_run_splits_into_six_observations_and_four_predictions() {
    // Given: the instant the capture's own run timestamp names.
    let refreshed = refresh(LIVE_METADATA);
    let stamps: Vec<String> = refreshed
        .frames
        .iter()
        .map(|frame| frame.timestamp.clone())
        .collect();

    // When
    let timeline = RadarTimeline::at(&stamps, "2026-08-31T18:55:00");

    // Then
    assert_eq!(timeline.first_forecast_index, 6);
    assert_eq!(timeline.observed.len(), 10);
}
