//! Plottable view of the two-hour raintext window.
//!
//! Buienradar intensities span three orders of magnitude, so a linear plot
//! hides drizzle whenever one heavy sample sets the scale. Bars are therefore
//! square-root compressed against a fixed full-scale reference, raised to the
//! observed peak only when that peak exceeds the reference. A dry window keeps
//! every bar at zero, which the panel renders as a flat baseline.
//!
//! The bridge methods that publish the plot are here too, cursor and end
//! included. The cursor is the graph's own read of which bar the radar is
//! currently over, and the end is the moment its last bar stands for, where the
//! radar's animation stops; keeping both beside the scale they are read against
//! is what stops the three from being changed apart.

use std::pin::Pin;

use allesbuien_core::{
    rain::RainPoint,
    rain_cursor::{frame_cursor, series_end},
    status::RAIN_UNAVAILABLE,
};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};

use crate::{qt_values::or_empty, weather_controller::ffi};

impl ffi::WeatherController {
    /// Points the rain graph's cursor at the radar frame QML is currently showing.
    pub fn track_radar_frame(mut self: Pin<&mut Self>, label: &QString) {
        self.as_mut().rust_mut().radar_frame_label = label.to_string();
        self.update_rain_cursor();
    }

    pub(crate) fn apply_rain(mut self: Pin<&mut Self>, line: &str, points: &[RainPoint]) {
        let graph = RainGraph::build(points, line);
        self.as_mut().set_rain_summary((&graph.summary).into());
        self.as_mut().set_rain_wet(graph.wet);
        self.as_mut()
            .set_rain_levels(graph.levels.iter().collect::<QList<f64>>());
        self.as_mut().set_rain_axis_labels(
            graph
                .axis_labels
                .iter()
                .map(QString::from)
                .collect::<QStringList>(),
        );
        self.as_mut().rust_mut().rain_points = points.to_vec();
        self.as_mut().update_rain_cursor();
        // Last, once everything it is read against is published: `Main.qml`
        // hands it straight to the radar bridge, which ends its animation there.
        self.set_rain_end_label(or_empty(series_end(points)));
    }

    fn update_rain_cursor(mut self: Pin<&mut Self>) {
        let cursor = frame_cursor(&self.rain_points, &self.radar_frame_label);
        self.as_mut().set_rain_cursor_visible(cursor.is_some());
        if let Some(fraction) = cursor {
            self.set_rain_cursor_fraction(fraction);
        }
    }
}

/// Intensity that fills the plot when nothing wetter is expected, in mm/h.
const FULL_SCALE_MMH: f64 = 5.0;
/// Minutes between two captioned ticks on the time axis.
const AXIS_TICK_MINUTES: u32 = 30;

/// Presentation-ready rain series: one bar per sample, plus its axis and caption.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RainGraph {
    /// Bar heights as a fraction of the plot height, one per sample.
    pub levels: Vec<f64>,
    /// Axis captions aligned to [`Self::levels`]; empty at unlabelled positions.
    pub axis_labels: Vec<String>,
    /// One-line rain sentence shown beside the plot; never empty.
    pub summary: String,
    /// True when at least one sample carries rain.
    pub wet: bool,
}

impl RainGraph {
    /// Converts an ordered raintext window and its sentence into a plottable view.
    #[must_use]
    pub fn build(points: &[RainPoint], line: &str) -> Self {
        let line = line.trim();
        let summary = if line.is_empty() {
            RAIN_UNAVAILABLE
        } else {
            line
        }
        .to_owned();
        let Some(last) = points.len().checked_sub(1) else {
            return Self {
                summary,
                ..Self::default()
            };
        };
        let peak_mmh = points.iter().map(|point| point.mmh).fold(0.0_f64, f64::max);
        let scale = peak_mmh.max(FULL_SCALE_MMH);
        Self {
            levels: points
                .iter()
                .map(|point| (point.mmh / scale).sqrt().clamp(0.0, 1.0))
                .collect(),
            axis_labels: points
                .iter()
                .enumerate()
                .map(|(index, point)| caption(index == last, point))
                .collect(),
            summary,
            wet: peak_mmh > 0.0,
        }
    }
}

fn caption(is_last: bool, point: &RainPoint) -> String {
    if is_last || point.minutes_from_start.is_multiple_of(AXIS_TICK_MINUTES) {
        point.time_label.clone()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use allesbuien_core::rain::{RainPoint, code_to_mmh, parse_series};

    use super::RainGraph;

    fn series(raw: &str) -> Vec<RainPoint> {
        parse_series(raw, 0).points
    }

    #[test]
    fn dry_window_flattens_every_bar_without_dropping_samples() {
        // Given
        let points = series("000|12:00\n000|12:05\n000|12:10\n");

        // When
        let graph = RainGraph::build(&points, "next 2h: dry");

        // Then
        assert_eq!(graph.levels, [0.0, 0.0, 0.0]);
        assert_eq!(graph.axis_labels.len(), 3);
        assert!(!graph.wet);
    }

    #[test]
    fn a_missing_rain_sentence_falls_back_to_an_explicit_caption() {
        // Given / When
        let blank = RainGraph::build(&series("000|12:00\n"), "   ");
        let carried = RainGraph::build(&[], "starts around 14:10");

        // Then
        assert_eq!(blank.summary, "Rain expectation unavailable");
        assert_eq!(carried.summary, "starts around 14:10");
    }

    #[test]
    fn empty_window_yields_no_bars_and_no_captions() {
        // Given / When
        let graph = RainGraph::build(&[], "");

        // Then
        assert_eq!(graph.levels, [] as [f64; 0]);
        assert_eq!(graph.axis_labels, [] as [String; 0]);
        assert!(!graph.wet);
    }

    #[test]
    fn light_rain_stays_visible_against_the_fixed_full_scale() {
        // Given: 0.04 mm/h, two orders of magnitude below the 5 mm/h reference.
        let points = series("000|12:00\n065|12:05\n");

        // When
        let graph = RainGraph::build(&points, "next 2h: dry");

        // Then
        // Compared exactly: encoded rain value zero must map to the exact
        // zero-height baseline asserted by this regression test.
        assert_eq!(graph.levels[0], 0.0);
        assert!(
            graph.levels[1] > 0.05,
            "drizzle collapsed to {}",
            graph.levels[1]
        );
        assert!(graph.levels[1] < 0.5, "drizzle exaggerated");
    }

    #[test]
    fn a_peak_above_the_reference_reaches_the_top_of_the_plot() {
        // Given: code 145 is far above the 5 mm/h reference.
        let points = series("000|12:00\n145|12:05\n");

        // When
        let graph = RainGraph::build(&points, "next 2h: dry");

        // Then
        assert!(code_to_mmh(145) > 5.0);
        assert!((graph.levels[1] - 1.0).abs() < 1e-9);
        assert!(graph.wet);
    }

    #[test]
    fn bar_heights_rank_samples_by_their_intensity() {
        // Given
        let points = series("000|12:00\n090|12:05\n110|12:10\n080|12:15\n");

        // When
        let graph = RainGraph::build(&points, "next 2h: dry");

        // Then
        assert!(graph.levels[2] > graph.levels[1]);
        assert!(graph.levels[1] > graph.levels[3]);
        assert!(graph.levels[3] > graph.levels[0]);
        assert!(graph.wet);
    }

    #[test]
    #[expect(
        clippy::format_collect,
        reason = "this eight-entry test fixture is clearest as a compact formatted iterator comprehension"
    )]
    fn axis_captions_appear_every_thirty_minutes_and_at_the_final_sample() {
        // Given: eight samples spanning 35 minutes at the five-minute cadence.
        let raw: String = (0..8).map(|step| format!("000|12:{step:02}\n")).collect();
        let points = series(&raw);

        // When
        let graph = RainGraph::build(&points, "next 2h: dry");

        // Then
        let captioned: Vec<usize> = graph
            .axis_labels
            .iter()
            .enumerate()
            .filter(|(_, label)| !label.is_empty())
            .map(|(index, _)| index)
            .collect();
        assert_eq!(captioned, [0, 6, 7]);
        assert_eq!(graph.axis_labels.len(), graph.levels.len());
    }
}
