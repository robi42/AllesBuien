//! Locates an animating radar frame inside the rain expectation series, and
//! names the moment the series ends.
//!
//! The two feeds are independent but their windows overlap: raintext is 24
//! samples covering two hours forward from about the present, and the radar
//! downloads 32 frames, from five minutes before the vendor's run to two and a
//! half hours after it. The run trails the present by several minutes, and the
//! radar asks for the extra half hour so that a late run still reaches the last
//! sample — `radar_transport`'s `FORECAST_STEPS` argues the number — so a fresh
//! run overhangs the series on both sides. The already-observed frames before
//! the first sample stay on screen without a cursor; the forecasts past the
//! last are downloaded but not played, because [`crate::rain_reach`] ends the
//! animation at [`series_end`].
//!
//! Both sides label their moments as viewer-local `HH:MM`, so wall clock is the
//! only basis the two share — neither carries a date. A difference of more than
//! twelve hours is therefore read as the series having wrapped past midnight
//! rather than as a frame half a day away, by the cursor and the cut alike.

use crate::{
    datetime::{minutes_of_day, shift_clock_label},
    rain::RainPoint,
};

const MINUTES_PER_DAY: i32 = 24 * 60;

/// Where a radar frame sits across the rain plot, as a fraction of its width.
///
/// The result is centred on the sample it names, matching the `(index + 0.5) / n`
/// convention the axis captions are laid out with, so the cursor and the caption
/// beneath it line up.
///
/// Returns `None` when the frame lies outside the series — the early observed
/// frames that precede the first sample, and the trailing frames that outrun
/// the last one, which the radar plays only while the series cannot bound its
/// run ([`crate::rain_reach`]). Callers must hide the cursor for those rather
/// than clamp it to an edge, which would claim a moment the graph does not
/// cover.
#[must_use]
pub fn frame_cursor(points: &[RainPoint], frame_label: &str) -> Option<f64> {
    let (start, span) = extent(points)?;
    let count = u32::try_from(points.len()).ok()?;
    let steps = count.checked_sub(1)?;
    let offset = wrapped_difference(start, minutes_of_day(frame_label)?);
    if !(0..=span).contains(&offset) {
        return None;
    }
    let index = f64::from(offset) * f64::from(steps) / f64::from(span);
    Some((index + 0.5) / f64::from(count))
}

/// The moment the series' last bar stands for, as `HH:MM`: the latest frame
/// label [`frame_cursor`] still places, on that bar's centre.
///
/// It is read the way the cursor reads the series, the first sample's label
/// moved on by the last sample's offset, rather than off the last row. The
/// offsets count the rows `rain` kept, five minutes apiece, so after a
/// malformed row it skipped, or under a last row without a time, the last row's
/// own label is somewhere the cursor does not reach. For a well-formed window
/// the two agree. `None` exactly when [`frame_cursor`] places no frame at all.
#[must_use]
pub fn series_end(points: &[RainPoint]) -> Option<String> {
    let (_, span) = extent(points)?;
    let first = &points.first()?.time_label;
    // Reduced to one day first, so that no span can overflow the shift.
    Some(shift_clock_label(first, span.rem_euclid(MINUTES_PER_DAY)))
}

/// Where the series starts on the dial, and how many minutes later its last
/// sample stands; `None` for a series the cursor cannot plot against: fewer
/// than two samples, a zero span, or a first sample without a clock label.
///
/// The one reading of the series' extent the cursor and its end share, so the
/// two cannot drift apart.
fn extent(points: &[RainPoint]) -> Option<(i32, i32)> {
    let [first, .., last] = points else {
        return None;
    };
    let span = i32::try_from(last.minutes_from_start)
        .ok()
        .filter(|span| *span > 0)?;
    Some((minutes_of_day(&first.time_label)?, span))
}

/// Signed minute distance from `from` to `to` on a 24-hour dial.
///
/// Shared with [`crate::rain_reach`], so that the cursor and the cut read the
/// dial alike.
pub(crate) fn wrapped_difference(from: i32, to: i32) -> i32 {
    let forward = (to - from).rem_euclid(MINUTES_PER_DAY);
    if forward > MINUTES_PER_DAY / 2 {
        forward - MINUTES_PER_DAY
    } else {
        forward
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        datetime::shift_clock_label,
        rain::{RainPoint, parse_series},
    };

    use super::{frame_cursor, series_end};

    /// The shipped window: 24 dry samples at the five-minute cadence.
    #[expect(
        clippy::format_collect,
        reason = "the suggested fold trades a legible one-expression fixture for a mutable accumulator and a swallowed write! result, to save one allocation of a 24-row string"
    )]
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
        // Given: frames past raintext's right edge, which the radar downloads,
        // and plays only while the series cannot bound its run.
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

    #[test]
    fn the_series_ends_where_the_cursor_puts_a_frame_on_the_last_bar() {
        // Given: 23:50 through 01:45, across midnight.
        let points = window(23, 50);

        // When
        let end = series_end(&points).expect("a plottable series has an end");

        // Then: the last sample's own label, on the last bar's centre, and
        // five minutes later the cursor has gone.
        assert_eq!(end, "01:45");
        let cursor = frame_cursor(&points, &end).expect("the end is inside the series");
        assert!((cursor - 23.5 / 24.0).abs() < 1e-12, "cursor {cursor}");
        assert_eq!(frame_cursor(&points, &shift_clock_label(&end, 5)), None);
    }

    #[test]
    fn the_end_is_measured_from_the_kept_rows_rather_than_read_off_the_last_one() {
        // Given: a malformed row the parser skipped, and a last row without a
        // time.
        let skipped = parse_series("000|10:00\n000|10:05\nbad|10:10\n000|10:15\n", 0).points;
        let untimed = parse_series("000|10:00\n000|10:05\n000\n", 0).points;

        // When / Then: three kept rows span ten minutes, whatever the last says.
        assert_eq!(series_end(&skipped).as_deref(), Some("10:10"));
        assert_eq!(
            frame_cursor(&skipped, "10:15"),
            None,
            "the last row's label"
        );
        assert_eq!(series_end(&untimed).as_deref(), Some("10:10"));
    }

    #[test]
    fn a_series_the_cursor_cannot_plot_has_no_end() {
        // Given
        let single = parse_series("000|10:15\n", 0).points;
        let unlabelled = parse_series("000|\n010|\n020|\n", 0).points;

        // When / Then
        assert_eq!(series_end(&[]), None, "no series at all");
        assert_eq!(series_end(&single), None, "one sample spans zero");
        assert_eq!(series_end(&unlabelled), None, "no clock to start from");
    }
}
