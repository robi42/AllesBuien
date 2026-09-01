//! Locates an animating radar frame inside the rain expectation series.
//!
//! The two feeds are independent but their windows now overlap: the radar run is
//! 26 frames from roughly five minutes back to two hours ahead, and raintext is
//! 24 samples covering two hours forward, so they share a right edge and differ
//! only at the handful of already-observed radar frames.
//!
//! Both sides label their moments as viewer-local `HH:MM`, so wall clock is the
//! only basis the two share — neither carries a date. A difference of more than
//! twelve hours is therefore read as the series having wrapped past midnight
//! rather than as a frame half a day away.

use crate::rain::RainPoint;

const MINUTES_PER_DAY: i32 = 24 * 60;

/// Where a radar frame sits across the rain plot, as a fraction of its width.
///
/// The result is centred on the sample it names, matching the `(index + 0.5) / n`
/// convention the axis captions are laid out with, so the cursor and the caption
/// beneath it line up.
///
/// Returns `None` when the frame lies outside the series — the early observed
/// frames that precede the first sample, and the trailing frames that outrun the
/// last one. Callers must hide the cursor for those rather than clamp it to an
/// edge, which would claim a moment the graph does not cover.
#[must_use]
pub fn frame_cursor(points: &[RainPoint], frame_label: &str) -> Option<f64> {
    let count = u32::try_from(points.len()).ok()?;
    let steps = count.checked_sub(1).filter(|steps| *steps > 0)?;
    let span = i32::try_from(points.last()?.minutes_from_start)
        .ok()
        .filter(|span| *span > 0)?;
    let start = minutes_of_day(&points.first()?.time_label)?;
    let offset = wrapped_difference(start, minutes_of_day(frame_label)?);
    if !(0..=span).contains(&offset) {
        return None;
    }
    let index = f64::from(offset) * f64::from(steps) / f64::from(span);
    Some((index + 0.5) / f64::from(count))
}

/// Signed minute distance from `from` to `to` on a 24-hour dial.
fn wrapped_difference(from: i32, to: i32) -> i32 {
    let forward = (to - from).rem_euclid(MINUTES_PER_DAY);
    if forward > MINUTES_PER_DAY / 2 {
        forward - MINUTES_PER_DAY
    } else {
        forward
    }
}

fn minutes_of_day(label: &str) -> Option<i32> {
    let (hours, minutes) = label.trim().split_once(':')?;
    let hours = hours.parse::<i32>().ok().filter(|h| (0..24).contains(h))?;
    let minutes = minutes
        .parse::<i32>()
        .ok()
        .filter(|m| (0..60).contains(m))?;
    Some(hours * 60 + minutes)
}

#[cfg(test)]
mod tests {
    use crate::rain::{RainPoint, parse_series};

    use super::frame_cursor;

    /// The shipped window: 24 dry samples at the five-minute cadence.
    fn window(first_hour: u32, first_minute: u32) -> Vec<RainPoint> {
        let raw: String = (0..24)
            .map(|step| {
                let total = first_hour * 60 + first_minute + step * 5;
                format!("000|{:02}:{:02}\n", (total / 60) % 24, total % 60)
            })
            .collect();
        parse_series(&raw, 0).points
    }

    #[test]
    fn the_first_sample_puts_the_cursor_on_the_first_bar_centre() {
        // Given
        let points = window(23, 50);

        // When
        let cursor = frame_cursor(&points, "23:50").expect("inside the series");

        // Then
        assert!((cursor - 0.5 / 24.0).abs() < 1e-12, "cursor {cursor}");
    }

    #[test]
    fn the_last_sample_puts_the_cursor_on_the_last_bar_centre() {
        // Given
        let points = window(23, 50);

        // When: 23:50 plus 115 minutes.
        let cursor = frame_cursor(&points, "01:45").expect("inside the series");

        // Then
        assert!((cursor - 23.5 / 24.0).abs() < 1e-12, "cursor {cursor}");
    }

    #[test]
    fn a_frame_before_the_series_starts_is_reported_as_outside_it() {
        // Given: the observed radar frames that precede the raintext window.
        let points = window(23, 50);

        // When / Then
        assert_eq!(frame_cursor(&points, "23:45"), None, "five minutes early");
        assert_eq!(
            frame_cursor(&points, "22:10"),
            None,
            "well before the start"
        );
    }

    #[test]
    fn a_frame_that_outruns_the_last_sample_is_reported_as_outside_it() {
        // Given: the radar window extends ten minutes past raintext's right edge.
        let points = window(23, 50);

        // When / Then
        assert_eq!(frame_cursor(&points, "01:50"), None, "five minutes late");
        assert_eq!(frame_cursor(&points, "01:55"), None, "ten minutes late");
    }

    #[test]
    fn a_window_crossing_midnight_places_interior_frames_by_wall_clock() {
        // Given: 23:50 through 01:45, the live case the feature was measured on.
        let points = window(23, 50);

        // When: 00:20 is 30 minutes in, sample index 6 of 24.
        let cursor = frame_cursor(&points, "00:20").expect("inside the series");

        // Then
        assert!((cursor - 6.5 / 24.0).abs() < 1e-12, "cursor {cursor}");
    }

    #[test]
    fn the_cursor_advances_monotonically_across_the_window() {
        // Given
        let points = window(10, 15);

        // When
        let early = frame_cursor(&points, "10:20").expect("inside");
        let middle = frame_cursor(&points, "11:15").expect("inside");
        let late = frame_cursor(&points, "12:05").expect("inside");

        // Then
        assert!(early < middle && middle < late, "{early} {middle} {late}");
        assert!(early > 0.0 && late < 1.0, "stayed inside the plot");
    }

    #[test]
    fn an_unusable_label_produces_no_cursor_rather_than_a_guess() {
        // Given
        let points = window(10, 15);

        // When / Then
        assert_eq!(frame_cursor(&points, ""), None, "no frame loaded yet");
        assert_eq!(frame_cursor(&points, "?"), None, "malformed feed row");
        assert_eq!(frame_cursor(&points, "25:00"), None, "impossible hour");
        assert_eq!(frame_cursor(&points, "10:75"), None, "impossible minute");
    }

    #[test]
    fn a_series_too_short_to_plot_has_no_cursor_position() {
        // Given
        let single = parse_series("000|10:15\n", 0).points;

        // When / Then
        assert_eq!(frame_cursor(&[], "10:15"), None, "no series at all");
        assert_eq!(
            frame_cursor(&single, "10:15"),
            None,
            "one sample spans zero"
        );
    }

    #[test]
    fn an_unparseable_series_start_produces_no_cursor() {
        // Given: raintext delivered rows without usable clock labels.
        let points = parse_series("000|\n010|\n020|\n", 0).points;

        // When / Then
        assert_eq!(points.len(), 3, "the rows themselves still parsed");
        assert_eq!(frame_cursor(&points, "10:15"), None);
    }
}
