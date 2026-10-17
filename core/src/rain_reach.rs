//! How far into a radar run the rain graph reaches, and so how much of the run
//! the animation plays.
//!
//! The radar asks for half an hour more forecast than raintext covers, so that
//! a run trailing the present still reaches the graph's last bar —
//! `radar_transport`'s `FORECAST_STEPS` argues the number. A fresh run
//! therefore carries up to six frames past that bar, and the graph's cursor has
//! nowhere to stand on any of them. [`frames_through`] stops the animation at
//! the frame on that bar instead, the last one
//! [`crate::rain_cursor::frame_cursor`] places. The frames past it still
//! download, because each raintext refresh moves the end on and brings them
//! back.
//!
//! The graph bounds a run only while it reaches into the run's predictions.
//! The observed frames before it always stay, and a run is never emptied: an
//! end the run's present has already passed — a graph kept through failed
//! weather refreshes while the radar moved on, or one older than a run fetched
//! first after a resume — cuts nothing, because cutting there would hide every
//! prediction of a fresh run, or every frame of one that has not begun.
//!
//! Captions are compared one at a time, in the order the metadata lists them,
//! on the cursor's own dial. Nothing here assumes a cadence or a frame count:
//! the vendor sets both, its live answer once held 21 frames where more were
//! asked for, and captions can step back across a daylight-saving change.

use crate::{datetime::minutes_of_day, rain_cursor::wrapped_difference};

/// How many leading frames of a radar run the animation plays.
///
/// `frame_labels` are the run's captions in metadata order, `end_label` is the
/// rain graph's end as [`crate::rain_cursor::series_end`] reads it, and
/// `first_forecast_index` is the run's `RadarTimeline` split. The run stops
/// before its first frame past the end, so a frame exactly at the end plays,
/// and an end between two captions keeps the earlier one. The whole run plays
/// when the end is empty or not a clock reading, when no frame lies past it, or
/// when the first frame past it is no later than the run's first prediction. A
/// caption that is not a clock reading never ends the run.
///
/// The answer is therefore the whole run, or more than `first_forecast_index`
/// frames and fewer than the whole run: never fewer than the run's
/// observations, and never none of a run that has frames.
#[must_use]
pub fn frames_through<S: AsRef<str>>(
    frame_labels: &[S],
    end_label: &str,
    first_forecast_index: usize,
) -> usize {
    let whole = frame_labels.len();
    let Some(end) = minutes_of_day(end_label) else {
        return whole;
    };
    frame_labels
        .iter()
        .position(|label| {
            minutes_of_day(label.as_ref()).is_some_and(|frame| wrapped_difference(end, frame) > 0)
        })
        .filter(|cut| *cut > first_forecast_index)
        .unwrap_or(whole)
}

#[cfg(test)]
mod tests {
    use crate::{
        datetime::shift_clock_label,
        rain::{RainPoint, parse_series},
        rain_cursor::{frame_cursor, series_end},
    };

    use super::frames_through;

    /// `count` captions five minutes apart from `first`, as a run lists them.
    fn run(first: &str, count: i32) -> Vec<String> {
        (0..count)
            .map(|step| shift_clock_label(first, step * 5))
            .collect()
    }

    /// The shipped rain window: 24 dry samples from `first`.
    fn graph(first: &str) -> Vec<RainPoint> {
        let rows: Vec<String> = run(first, 24)
            .iter()
            .map(|label| format!("000|{label}"))
            .collect();
        parse_series(&rows.join("\n"), 0).points
    }

    #[test]
    fn a_fresh_run_plays_up_to_the_frame_on_the_graphs_last_bar() {
        // Given: a run starting 15 minutes before a graph that crosses
        // midnight, with two observations, and the graph's end.
        let points = graph("23:50");
        let frames = run("23:35", 32);
        let end = series_end(&points).expect("a plottable graph has an end");

        // When
        let shown = frames_through(&frames, &end, 2);

        // Then: the frames before the graph stay, every frame shown past them
        // has a place on the graph, and none of the hidden ones does.
        assert_eq!(shown, 27);
        assert_eq!(frames[shown - 1], "01:45");
        let placed: Vec<bool> = frames
            .iter()
            .map(|frame| frame_cursor(&points, frame).is_some())
            .collect();
        assert_eq!(placed[..3], [false; 3], "the frames before the graph stay");
        assert!(placed[3..shown].iter().all(|placed| *placed));
        assert!(!placed[shown..].iter().any(|placed| *placed));
    }

    #[test]
    fn an_end_between_two_frames_keeps_the_one_before_it() {
        // Given
        let frames = run("10:00", 32);

        // When / Then: 12:05 is frame 25, and 12:07 still stops after it.
        assert_eq!(frames_through(&frames, "12:05", 2), 26);
        assert_eq!(frames_through(&frames, "12:07", 2), 26);
    }

    #[test]
    fn without_an_end_inside_the_run_nothing_is_cut() {
        // Given
        let frames = run("10:00", 32);

        // When / Then
        for unreadable in ["", "?", "24:00", "soon"] {
            assert_eq!(frames_through(&frames, unreadable, 2), 32, "{unreadable:?}");
        }
        assert_eq!(frames_through(&frames, "12:35", 2), 32, "the last frame");
        assert_eq!(frames_through(&frames, "13:30", 2), 32, "past the run");
        assert_eq!(frames_through::<&str>(&[], "10:00", 0), 0, "no run at all");
    }

    #[test]
    fn an_end_that_reaches_no_prediction_cuts_nothing() {
        // Given
        let frames = run("10:00", 32);

        // When / Then
        assert_eq!(
            frames_through(&frames, "10:05", 2),
            32,
            "the last observation"
        );
        assert_eq!(frames_through(&frames, "09:30", 2), 32, "before the run");
        assert_eq!(frames_through(&frames, "22:05", 2), 32, "half a day off");
        assert_eq!(
            frames_through(&frames, "10:10", 2),
            3,
            "the first prediction"
        );
        assert_eq!(
            frames_through(&frames, "10:10", 32),
            32,
            "a wholly observed run"
        );
        assert_eq!(
            frames_through(&frames, "10:00", 0),
            1,
            "a run of predictions"
        );
        assert_eq!(
            frames_through(&frames, "09:55", 0),
            32,
            "before any of them"
        );
    }

    #[test]
    fn the_cut_reads_each_caption_in_run_order() {
        // Given: an unreadable caption, a run of them, and captions stepping
        // back an hour as clocks do when daylight saving ends.
        let gapped = ["10:00", "10:05", "not-a", "10:15", "10:20"];
        let unreadable = ["not-a", ""];
        let fall_back = ["02:40", "02:45", "02:50", "02:55", "02:00", "02:05"];

        // When / Then
        assert_eq!(frames_through(&gapped, "10:15", 1), 4);
        assert_eq!(frames_through(&unreadable, "10:00", 0), 2);
        assert_eq!(frames_through(&fall_back, "02:50", 1), 3);
    }

    #[test]
    fn no_end_empties_a_run_or_hides_one_of_its_observations() {
        // Given: ends before, across, and after every run below, and none.
        let mut ends = run("09:40", 50);
        ends.push(String::new());

        // When / Then
        for count in 1..=40 {
            let frames = run("10:00", count);
            for first in 0..=frames.len() {
                for end in &ends {
                    let shown = frames_through(&frames, end, first);
                    assert!(
                        shown == frames.len() || first < shown,
                        "{count} frames, split {first}, end {end:?}: {shown}"
                    );
                }
            }
        }
    }
}
