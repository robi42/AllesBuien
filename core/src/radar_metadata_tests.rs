//! Offline coverage for the radar metadata fields the app reads.
//!
//! The fixture is a verbatim capture of the live `history=0&forecast=24` sprite
//! metadata response, so a vendor schema change surfaces here rather than as a
//! two-hour caption drift on someone's desktop.
//!
//! It exercises only the public [`RadarClient`] surface, as the app does. It sits
//! inside the crate rather than under `tests/` so it shares [`crate::test_support`]'s
//! executor and radar fake instead of carrying private copies of both.

use std::sync::Arc;

use tempfile::tempdir;

use crate::{
    radar::{GenerationId, RadarClient, RadarRefresh},
    radar_timeline::RadarTimeline,
    test_support::{StaticRadarApi, block_on},
};

const LIVE_METADATA: &[u8] = include_bytes!("../tests/fixtures/radar-metadata.json");

const METADATA_WITHOUT_OFFSET: &[u8] = br#"{
    "timestamp":"2026-08-31T18:55:00",
    "times":[{"timestamp":"2026-08-31T18:30:00","url":"fake://frame-0"}]
}"#;

fn refresh(metadata: &'static [u8]) -> RadarRefresh {
    let state = tempdir().expect("temp state");
    let client = RadarClient::new(
        Arc::new(StaticRadarApi::serving(metadata)),
        state.path().to_path_buf(),
    );
    block_on(client.refresh(GenerationId(1))).expect("fixture refresh should succeed")
}

#[test]
fn the_live_capture_carries_the_vendor_utc_offset_and_twenty_one_frames() {
    // Given / When
    let refreshed = refresh(LIVE_METADATA);

    // Then
    assert_eq!(refreshed.time_offset_hours, Some(2));
    assert_eq!(refreshed.frames.len(), 21);
    assert_eq!(refreshed.frames[0].timestamp, "2026-09-02T13:35:00");
    assert_eq!(refreshed.frames[20].timestamp, "2026-09-02T15:15:00");
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
fn the_captured_run_opens_at_the_present_with_everything_after_it_predicted() {
    // Given: the instant the capture's own run timestamp names.
    let refreshed = refresh(LIVE_METADATA);
    let stamps: Vec<String> = refreshed
        .frames
        .iter()
        .map(|frame| frame.timestamp.clone())
        .collect();

    // When
    let timeline = RadarTimeline::at(&stamps, "2026-09-02T13:35:00");

    // Then
    assert_eq!(timeline.first_forecast_index, 1);
    assert_eq!(timeline.observed.len(), 21);
}
