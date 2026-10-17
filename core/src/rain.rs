//! Rain expectation in five-minute steps, from Buienradar's raintext feed.
//!
//! The endpoint returns twenty-four `value|HH:MM` rows — two hours at five
//! minute cadence — where `value` is Buienradar's logarithmic intensity code
//! and `000` means dry. The documented conversion is `mm/h = 10^((v - 109)/32)`.
//!
//! Two views are derived from the same pass: [`RainSoon`], the one-line "is it
//! about to rain" summary the notifier and header consume, and [`RainPoint`],
//! the complete ordered series a graph can plot.

use crate::datetime::shift_clock_label;

/// Cadence between consecutive raintext rows.
pub(crate) const RAIN_STEP_MINUTES: u32 = 5;

/// Summary of the next rain event within the raintext window.
#[derive(Debug, Clone, PartialEq)]
pub struct RainSoon {
    pub minutes_until: u32,
    pub time_label: String,
    pub max_mmh: f64,
}

/// One raintext sample, ordered oldest-first within [`RainSeries::points`].
#[derive(Debug, Clone, PartialEq)]
pub struct RainPoint {
    /// Raw Buienradar intensity code; `0` means dry.
    pub code: i32,
    /// Intensity in millimetres per hour, `10^((code - 109) / 32)`.
    pub mmh: f64,
    /// `HH:MM` exactly as the feed delivered it, i.e. Dutch wall clock.
    pub source_time_label: String,
    /// `HH:MM` converted to the viewer's timezone.
    pub time_label: String,
    /// Whole minutes between the first sample and this one.
    pub minutes_from_start: u32,
}

/// The complete raintext window plus the summary derived from it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RainSeries {
    /// Ordered samples, oldest first. Empty when the feed returned nothing usable.
    pub points: Vec<RainPoint>,
    /// Highest intensity across the window, in millimetres per hour.
    pub peak_mmh: f64,
    /// One-line human summary, e.g. `next 2h: dry`.
    pub line: String,
    /// Present only when at least one sample is wet.
    pub rain_soon: Option<RainSoon>,
}

/// Converts a Buienradar rain intensity code to millimetres per hour.
#[must_use]
pub fn code_to_mmh(code: i32) -> f64 {
    if code <= 0 {
        return 0.0;
    }
    10_f64.powf((f64::from(code) - 109.0) / 32.0)
}

/// Parses a raintext payload into the full series and its summary.
///
/// `clock_shift_minutes` is added to every `HH:MM` label so the series reads in
/// the viewer's timezone; pass `0` to keep the Dutch wall clock.
#[must_use]
pub fn parse_series(raw: &str, clock_shift_minutes: i32) -> RainSeries {
    summarise(parse_points(raw, clock_shift_minutes))
}

/// Parses raintext rows into ordered samples, oldest first.
///
/// Blank and malformed rows are skipped rather than treated as dry, so a
/// partially broken payload still produces a usable graph.
fn parse_points(raw: &str, clock_shift_minutes: i32) -> Vec<RainPoint> {
    let mut points: Vec<RainPoint> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split('|');
        let Some(code) = parts
            .next()
            .and_then(|part| part.trim().parse::<i32>().ok())
        else {
            continue;
        };
        let source_time_label = parts.next().unwrap_or("?").trim().to_owned();
        let minutes_from_start = u32::try_from(points.len())
            .map_or(u32::MAX, |index| index.saturating_mul(RAIN_STEP_MINUTES));
        points.push(RainPoint {
            code,
            mmh: code_to_mmh(code),
            time_label: shift_clock_label(&source_time_label, clock_shift_minutes),
            source_time_label,
            minutes_from_start,
        });
    }
    points
}

/// Finds the peak and the first wet sample, and builds the one-line summary.
fn summarise(points: Vec<RainPoint>) -> RainSeries {
    let peak = points
        .iter()
        .max_by(|left, right| left.code.cmp(&right.code));
    let Some(peak) = peak.filter(|peak| peak.code > 0) else {
        return RainSeries {
            points,
            peak_mmh: 0.0,
            line: String::from("next 2h: dry"),
            rain_soon: None,
        };
    };
    let peak_mmh = peak.mmh;
    let peak_label = time_label_or_unknown(&peak.time_label);
    let first = points
        .iter()
        .find(|point| point.code > 0)
        .expect("a positive peak implies a first wet sample");
    let first_label = time_label_or_unknown(&first.time_label).to_owned();
    let line = format!("starts around {first_label}, max {peak_mmh:.2} mm/h at {peak_label}");
    let rain_soon = Some(RainSoon {
        minutes_until: first.minutes_from_start,
        time_label: first_label,
        max_mmh: peak_mmh,
    });
    RainSeries {
        points,
        peak_mmh,
        line,
        rain_soon,
    }
}

/// A sample's clock label, or `?` for a row that carried none.
fn time_label_or_unknown(label: &str) -> &str {
    if label.is_empty() { "?" } else { label }
}

#[cfg(test)]
mod tests {
    use super::{RAIN_STEP_MINUTES, code_to_mmh, parse_series};

    const RAINTEXT_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/raintext.txt"
    ));

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "the dry path assigns the literal 0.0, and exactly zero is the contract; an epsilon window would pass on a near-zero intensity that is not dry"
    )]
    fn dry_window_still_exposes_every_sample() {
        // Given
        let raw = "000|12:00\n000|12:05\n";

        // When
        let series = parse_series(raw, 0);

        // Then
        assert_eq!(series.line, "next 2h: dry");
        assert!(series.rain_soon.is_none());
        assert_eq!(series.points.len(), 2);
        assert_eq!(series.peak_mmh, 0.0);
        assert!(series.points.iter().all(|point| point.mmh == 0.0));
    }

    #[test]
    fn series_preserves_feed_order_and_five_minute_cadence() {
        // Given
        let raw = "000|12:00\n050|12:05\n080|12:10\n060|12:15\n";

        // When
        let series = parse_series(raw, 0);

        // Then
        let labels: Vec<&str> = series
            .points
            .iter()
            .map(|point| point.time_label.as_str())
            .collect();
        assert_eq!(labels, ["12:00", "12:05", "12:10", "12:15"]);
        let offsets: Vec<u32> = series
            .points
            .iter()
            .map(|point| point.minutes_from_start)
            .collect();
        assert_eq!(offsets, [0, 5, 10, 15]);
        assert_eq!(RAIN_STEP_MINUTES, 5);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "both sides come from the same deterministic conversion applied to the same code, so they are bit-identical; an epsilon window would also accept the neighbouring sample this test exists to rule out"
    )]
    fn peak_is_the_maximum_intensity_not_the_last_wet_sample() {
        // Given
        let raw = "000|12:00\n050|12:05\n080|12:10\n060|12:15\n";

        // When
        let series = parse_series(raw, 0);

        // Then
        assert_eq!(series.peak_mmh, code_to_mmh(80));
        assert!(series.line.contains("at 12:10"));
        let rain = series.rain_soon.expect("expected rain_soon");
        assert_eq!(rain.minutes_until, 5);
        assert_eq!(rain.time_label, "12:05");
        assert_eq!(rain.max_mmh, code_to_mmh(80));
    }

    #[test]
    fn series_labels_are_shifted_into_the_viewer_timezone() {
        // Given
        let raw = "000|23:50\n090|23:55\n";

        // When
        let series = parse_series(raw, -120);

        // Then
        assert_eq!(series.points[0].source_time_label, "23:50");
        assert_eq!(series.points[0].time_label, "21:50");
        assert_eq!(series.points[1].time_label, "21:55");
        let rain = series.rain_soon.expect("expected rain_soon");
        assert_eq!(rain.time_label, "21:55");
    }

    #[test]
    fn malformed_rows_are_skipped_without_breaking_the_series() {
        // Given
        let raw = "\n  \nnot-a-code|12:00\n070|12:05\n090\n";

        // When
        let series = parse_series(raw, 0);

        // Then
        assert_eq!(series.points.len(), 2);
        assert_eq!(series.points[0].time_label, "12:05");
        assert_eq!(series.points[1].source_time_label, "?");
        assert!(series.line.contains("at ?"));
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "clamping is the behaviour under test and it returns the literal 0.0; the neighbouring assertion already uses a tolerance where the value really is computed"
    )]
    fn conversion_is_monotonic_and_clamps_dry_codes_to_zero() {
        // Given / When / Then
        assert!(code_to_mmh(90) > code_to_mmh(70));
        assert_eq!(code_to_mmh(0), 0.0);
        assert_eq!(code_to_mmh(-5), 0.0);
        assert!((code_to_mmh(109) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn live_fixture_yields_the_full_two_hour_window() {
        // Given / When
        let series = parse_series(RAINTEXT_FIXTURE, 0);

        // Then
        assert_eq!(series.points.len(), 24);
        assert_eq!(series.points[0].minutes_from_start, 0);
        assert_eq!(series.points[23].minutes_from_start, 115);
        assert!(series.peak_mmh > 0.0);
    }
}
