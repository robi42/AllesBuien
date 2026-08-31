//! Splits a radar run's frames into observed history and forecast.
//!
//! Buienradar's sprite metadata packs both halves into one `times[]` array —
//! the measured past and the extrapolated near future — and marks neither. The
//! boundary is purely temporal: a frame whose stamp has already passed is an
//! observation, everything after it is a prediction. It therefore *moves* while
//! a run ages, so it is recomputed on every read rather than stored with the run.
//!
//! Radar stamps are UTC, unlike the JSON weather feed's naive Amsterdam
//! stamps (see [`crate::datetime`]), so the comparison instant is UTC too.

use jiff::{Timestamp, civil::DateTime, tz::TimeZone};

/// Where a radar run stops reporting and starts predicting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RadarTimeline {
    /// Index of the first predicted frame.
    ///
    /// Equals the frame count for a stale run whose every frame is already in
    /// the past — nothing is a forecast — and `0` when no frame has happened
    /// yet, which makes the whole run a forecast. An empty run yields `0`,
    /// indistinguishable from "no observations" only because it has no frames.
    pub first_forecast_index: usize,
    /// One flag per input frame, `true` while that frame is an observation.
    pub observed: Vec<bool>,
}

impl RadarTimeline {
    /// Splits `frame_stamps` against this machine's current instant.
    #[must_use]
    pub fn now<S: AsRef<str>>(frame_stamps: &[S]) -> Self {
        Self::split(frame_stamps, Some(Timestamp::now()))
    }

    /// Splits `frame_stamps` against an explicit naive-UTC instant.
    ///
    /// Exposed so callers — and tests — pin the boundary rather than inherit
    /// the host clock. An unreadable `now_utc` leaves every frame observed.
    #[must_use]
    pub fn at<S: AsRef<str>>(frame_stamps: &[S], now_utc: &str) -> Self {
        Self::split(frame_stamps, instant(now_utc))
    }

    fn split<S: AsRef<str>>(frame_stamps: &[S], now: Option<Timestamp>) -> Self {
        let observed: Vec<bool> = frame_stamps
            .iter()
            .map(|stamp| match (instant(stamp.as_ref()), now) {
                (Some(frame), Some(now)) => frame <= now,
                // An undatable frame is never announced as a prediction.
                _ => true,
            })
            .collect();
        Self {
            first_forecast_index: observed
                .iter()
                .position(|observed| !observed)
                .unwrap_or(observed.len()),
            observed,
        }
    }
}

fn instant(naive_utc: &str) -> Option<Timestamp> {
    let civil: DateTime = naive_utc.trim().parse().ok()?;
    civil
        .to_zoned(TimeZone::UTC)
        .ok()
        .map(|zoned| zoned.timestamp())
}

#[cfg(test)]
mod tests {
    use super::RadarTimeline;

    /// A ten-frame run at the endpoint's fixed 5-minute cadence. The split is
    /// window-agnostic, so this stays smaller than the live 26-frame window.
    const RUN: [&str; 10] = [
        "2026-08-31T18:30:00",
        "2026-08-31T18:35:00",
        "2026-08-31T18:40:00",
        "2026-08-31T18:45:00",
        "2026-08-31T18:50:00",
        "2026-08-31T18:55:00",
        "2026-08-31T19:00:00",
        "2026-08-31T19:05:00",
        "2026-08-31T19:10:00",
        "2026-08-31T19:15:00",
    ];

    #[test]
    fn a_live_run_splits_at_the_supplied_instant() {
        // Given: the measured run, read at the UTC instant it was measured at.
        let now = "2026-08-31T19:05:00";

        // When
        let timeline = RadarTimeline::at(&RUN, now);

        // Then: the frame stamped exactly now still counts as observed.
        assert_eq!(timeline.first_forecast_index, 8);
        assert!(timeline.observed[7]);
        assert!(!timeline.observed[8]);
        assert_eq!(timeline.observed.len(), RUN.len());
    }

    #[test]
    fn the_boundary_advances_as_the_same_run_ages() {
        // Given / When
        let fresh = RadarTimeline::at(&RUN, "2026-08-31T18:32:00");
        let older = RadarTimeline::at(&RUN, "2026-08-31T18:58:00");

        // Then
        assert_eq!(fresh.first_forecast_index, 1);
        assert_eq!(older.first_forecast_index, 6);
    }

    #[test]
    fn a_stale_run_reports_every_frame_as_observed() {
        // Given / When
        let timeline = RadarTimeline::at(&RUN, "2026-08-31T23:59:00");

        // Then
        assert_eq!(timeline.first_forecast_index, RUN.len());
        assert!(timeline.observed.iter().all(|observed| *observed));
    }

    #[test]
    fn a_run_that_has_not_started_reports_every_frame_as_forecast() {
        // Given / When
        let timeline = RadarTimeline::at(&RUN, "2026-08-31T17:00:00");

        // Then
        assert_eq!(timeline.first_forecast_index, 0);
        assert!(timeline.observed.iter().all(|observed| !observed));
    }

    #[test]
    fn an_undatable_stamp_is_never_announced_as_a_prediction() {
        // Given
        let stamps = ["not-a-timestamp", "2026-08-31T19:10:00"];

        // When
        let unreadable_frame = RadarTimeline::at(&stamps, "2026-08-31T19:05:00");
        let unreadable_now = RadarTimeline::at(&RUN, "");

        // Then
        assert_eq!(unreadable_frame.observed, [true, false]);
        assert_eq!(unreadable_frame.first_forecast_index, 1);
        assert_eq!(unreadable_now.first_forecast_index, RUN.len());
    }

    #[test]
    fn an_empty_run_has_no_frames_to_split() {
        // Given / When
        let timeline = RadarTimeline::at::<&str>(&[], "2026-08-31T19:05:00");

        // Then
        assert_eq!(timeline.first_forecast_index, 0);
        assert!(timeline.observed.is_empty());
    }

    #[test]
    fn the_host_clock_reading_separates_the_distant_past_from_the_distant_future() {
        // Given
        let stamps = ["2000-01-01T00:00:00", "2999-01-01T00:00:00"];

        // When
        let timeline = RadarTimeline::now(&stamps);

        // Then
        assert_eq!(timeline.observed, [true, false]);
        assert_eq!(timeline.first_forecast_index, 1);
    }
}
