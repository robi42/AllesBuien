//! The radar run the controller replays, and where it stops being observed.
//!
//! Captioning and the observed/forecast split live here rather than in the
//! controller so they can be asserted without a `QGuiApplication`.

use allesbuien_core::{radar::RadarRefresh, radar_timeline::RadarTimeline};

use crate::{image_provider::FrameBytes, radar_clock::frame_label};

/// One metadata run whose frame PNGs have been read off the cache.
pub(crate) struct LoadedRefresh {
    pub(crate) refresh: RadarRefresh,
    pub(crate) frames: Vec<FrameBytes>,
}

/// One radar run, captioned on this machine's clock and ready to replay.
#[derive(Clone)]
pub(crate) struct FrameSet {
    pub(crate) timestamps: Vec<String>,
    pub(crate) utc_stamps: Vec<String>,
    pub(crate) frames: Vec<FrameBytes>,
}

impl FrameSet {
    pub(crate) fn from_loaded(loaded: LoadedRefresh) -> Self {
        let offset_hours = loaded.refresh.time_offset_hours;
        let utc_stamps: Vec<String> = loaded
            .refresh
            .frames
            .into_iter()
            .map(|frame| frame.timestamp)
            .collect();
        Self {
            timestamps: utc_stamps
                .iter()
                .map(|stamp| frame_label(stamp, offset_hours))
                .collect(),
            utc_stamps,
            frames: loaded.frames,
        }
    }

    /// Splits this run at the current instant.
    ///
    /// Recomputed on every application rather than cached with the run: a
    /// replayed last-good set has aged and its boundary has moved with it.
    pub(crate) fn timeline(&self) -> RadarTimeline {
        RadarTimeline::now(&self.utc_stamps)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use allesbuien_core::{
        datetime::LocalTimestamp,
        radar::{GenerationId, RadarFrame, RadarRefresh},
    };

    use super::{FrameSet, LoadedRefresh};

    fn loaded(stamps: &[&str], time_offset_hours: Option<i32>) -> LoadedRefresh {
        LoadedRefresh {
            refresh: RadarRefresh {
                generation: GenerationId(1),
                frames: stamps
                    .iter()
                    .map(|stamp| RadarFrame {
                        timestamp: (*stamp).to_owned(),
                        url: String::new(),
                        path: PathBuf::new(),
                    })
                    .collect(),
                time_offset_hours,
            },
            frames: stamps.iter().map(|_| Vec::new().into()).collect(),
        }
    }

    fn minutes_of(label: &str) -> i32 {
        let (hours, minutes) = label.split_once(':').expect("expected a HH:MM label");
        hours.parse::<i32>().expect("hours") * 60 + minutes.parse::<i32>().expect("minutes")
    }

    #[test]
    fn captions_are_shifted_by_the_runs_own_vendor_offset() {
        // Given: a winter run whose vendor declares two hours, not the CET one.
        let stamp = "2026-01-15T12:00:00";

        // When
        let set = FrameSet::from_loaded(loaded(&[stamp], Some(2)));

        // Then: pinning the shift rather than the digits keeps this host-agnostic.
        assert_eq!(
            (minutes_of(&set.timestamps[0])
                - minutes_of(&LocalTimestamp::from_feed(stamp).time_label))
            .rem_euclid(24 * 60),
            120
        );
        assert_eq!(set.utc_stamps, [stamp]);
    }

    #[test]
    fn the_raw_utc_stamps_survive_captioning() {
        // Given / When
        let set = FrameSet::from_loaded(loaded(
            &["2026-08-31T18:30:00", "2026-08-31T18:35:00"],
            Some(2),
        ));

        // Then
        assert_eq!(set.utc_stamps.len(), 2);
        assert_eq!(set.timestamps.len(), 2);
        assert_eq!(set.frames.len(), 2);
    }

    #[test]
    fn a_run_that_has_already_happened_is_observed_end_to_end() {
        // Given
        let set = FrameSet::from_loaded(loaded(
            &["2000-01-01T00:00:00", "2000-01-01T00:05:00"],
            Some(1),
        ));

        // When
        let timeline = set.timeline();

        // Then
        assert_eq!(timeline.first_forecast_index, 2);
        assert_eq!(timeline.observed, [true, true]);
    }

    #[test]
    fn a_run_that_has_not_happened_yet_is_forecast_end_to_end() {
        // Given
        let set = FrameSet::from_loaded(loaded(&["2999-01-01T00:00:00"], None));

        // When
        let timeline = set.timeline();

        // Then
        assert_eq!(timeline.first_forecast_index, 0);
        assert_eq!(timeline.observed, [false]);
    }
}
