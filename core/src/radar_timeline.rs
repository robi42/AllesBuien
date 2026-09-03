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

/// Reads a radar stamp — naive UTC, per this module's doc — as an instant.
///
/// Unreadable text yields [`None`] rather than a fabricated instant.
///
/// Public so [`crate::radar_freshness`] can date a run's own timestamp under the
/// same convention. Two parsers for "which instant is this stamp" would be one
/// too many: whichever of them a vendor zone change reached first, the frame
/// captions and the staleness verdict would then disagree about the same run.
#[must_use]
pub fn instant(naive_utc: &str) -> Option<Timestamp> {
    let civil: DateTime = naive_utc.trim().parse().ok()?;
    civil
        .to_zoned(TimeZone::UTC)
        .ok()
        .map(|zoned| zoned.timestamp())
}

/// Share of a scrubber track that is observation rather than prediction.
///
/// The slider spans frame `0` to `frame_count - 1`, so the transition sits half
/// a step before the first predicted frame. A run with fewer than two frames has
/// no track to divide and reads as fully observed, which is what the app shows
/// while nothing has loaded.
#[must_use]
pub fn observed_fraction(first_forecast_index: usize, frame_count: usize) -> f64 {
    let Some(last) = frame_count.checked_sub(1).filter(|last| *last > 0) else {
        return 1.0;
    };
    if first_forecast_index == 0 {
        return 0.0;
    }
    if first_forecast_index >= frame_count {
        return 1.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "frame counts are vendor-capped at 24 and lossless in f64"
    )]
    let fraction = (first_forecast_index as f64 - 0.5) / last as f64;
    fraction.clamp(0.0, 1.0)
}

/// Whether the run actually contains both halves, so a divider means something.
///
/// A stale run (everything observed) and a run that has not started (everything
/// predicted) both have their boundary outside the track; drawing a line on the
/// rail's edge there would invent a split the data does not have.
#[must_use]
pub fn boundary_visible(first_forecast_index: usize, frame_count: usize) -> bool {
    frame_count > 1 && first_forecast_index > 0 && first_forecast_index < frame_count
}

#[cfg(test)]
mod tests {
    use super::{RadarTimeline, boundary_visible, observed_fraction};

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
    #[test]
    fn a_live_run_puts_the_boundary_half_a_step_before_the_first_prediction() {
        // Given: the shipped window — 2 observations, 24 predictions.
        // When
        let fraction = observed_fraction(2, 26);

        // Then: 1.5 steps of 25, i.e. just inside the left edge.
        assert!(
            (fraction - 0.06).abs() < 1e-9,
            "unexpected fraction {fraction}"
        );
        assert!(boundary_visible(2, 26));
    }

    #[test]
    fn a_stale_run_is_wholly_observed_and_draws_no_divider() {
        // Given / When: first_forecast_index == frame_count.
        // Then
        assert!((observed_fraction(26, 26) - 1.0).abs() < f64::EPSILON);
        assert!(!boundary_visible(26, 26));
    }

    #[test]
    fn a_run_that_has_not_started_is_wholly_predicted_and_draws_no_divider() {
        // Given / When: first_forecast_index == 0.
        // Then
        assert!(observed_fraction(0, 26).abs() < f64::EPSILON);
        assert!(!boundary_visible(0, 26));
    }

    #[test]
    fn a_run_too_short_to_scrub_has_no_track_to_divide() {
        // Given / When
        // Then
        assert!((observed_fraction(0, 1) - 1.0).abs() < f64::EPSILON);
        assert!((observed_fraction(0, 0) - 1.0).abs() < f64::EPSILON);
        assert!(!boundary_visible(0, 1));
        assert!(!boundary_visible(1, 1));
    }

    #[test]
    fn the_fraction_never_leaves_the_track() {
        // Given / When / Then
        for count in 2usize..40 {
            for first in 0..=count {
                let fraction = observed_fraction(first, count);
                assert!(
                    (0.0..=1.0).contains(&fraction),
                    "fraction {fraction} escaped for {first}/{count}"
                );
            }
        }
    }
}
