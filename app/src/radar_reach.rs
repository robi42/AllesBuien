//! How far into a radar run the transport reaches: up to the rain graph's last
//! bar.
//!
//! The radar downloads half an hour more forecast than the rain graph covers,
//! so that a run trailing the present still reaches that bar, which leaves a
//! fresh run up to six frames past it where the graph's cursor has nowhere to
//! stand. The bridge publishes the run only as far as the frame on that bar,
//! the frames before the graph included, and everything QML reads about the run
//! follows from that one count: `frameCount` bounds the playback loop, the
//! slider, End, and Right, `timestamps`, which `currentFrameLabel` is read from,
//! holds the shown frames' captions, and the observed/forecast divider is
//! measured against the track the slider spans. [`allesbuien_core::rain_reach`]
//! decides the count, and leaves the whole run when the graph ends before the
//! run's predictions begin.
//!
//! A cut only ever moves that count. Every frame of the run stays downloaded
//! and in the image store under its own `image://radar/<generation>/<index>`,
//! so when a later raintext refresh moves the graph's end on, a hidden frame
//! comes back without a fetch and without a new generation.
//!
//! Either feed may answer first. The graph's end is the weather's and outlives
//! radar refreshes, because `Main.qml` hands it over only when it changes, and
//! each run is cut at it as it is adopted; the adopted run is kept whole, so a
//! later end can cut it again either way. While a refresh is in flight there is
//! no run to cut, and a new end only waits for the adoption: a frame URL
//! published then would be the very string the adoption publishes next, and a
//! setter handed the same string emits nothing, so the overlay would never ask
//! the provider for that frame again.
//!
//! A new end is applied on the next turn of the event loop rather than inside
//! the call that hands it over. That call arrives while the weather bridge is
//! still publishing the graph whose end moved, and a cut can move the frame on
//! screen, whose caption `Main.qml` hands straight back to the weather bridge,
//! which would be called into while its own refresh still holds it. The final
//! state is the same either way, so no test can see the difference; this
//! paragraph is the guard.
//!
//! Split out of the bridge because this is the part of the radar surface that
//! answers to the other bridge, and its decisions are plain values a unit test
//! can read without a `QGuiApplication`.

use std::pin::Pin;

use allesbuien_core::{
    radar_timeline::{boundary_visible, observed_fraction},
    rain_reach::frames_through,
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};

use crate::radar_controller::ffi;

/// The run on screen, whole: what a new rain end cuts again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AdoptedRun {
    /// Every frame's caption in metadata order, the hidden ones included.
    pub(crate) labels: Vec<String>,
    /// The run's first prediction, as split when it was applied.
    pub(crate) first_forecast_index: usize,
}

impl AdoptedRun {
    /// The part of this run the transport reaches when the rain graph ends at
    /// `rain_end`; an empty `rain_end` reaches the whole run.
    pub(crate) fn reach(&self, rain_end: &str) -> Reach {
        let first = self.first_forecast_index;
        let shown = frames_through(&self.labels, rain_end, first);
        Reach {
            captions: self.labels.iter().take(shown).cloned().collect(),
            first_forecast_index: first,
            observed_fraction: observed_fraction(first, shown),
            boundary_visible: boundary_visible(first, shown),
        }
    }
}

/// Everything the transport is shown of a run, as plain values.
#[derive(Debug)]
pub(crate) struct Reach {
    /// The shown frames' captions; their count is `frameCount`.
    pub(crate) captions: Vec<String>,
    /// The run's first prediction, never past the shown frames.
    pub(crate) first_forecast_index: usize,
    /// The share of the slider's track that is observation, measured against
    /// the shown frames rather than the whole run.
    pub(crate) observed_fraction: f64,
    /// Whether the shown frames hold both halves, so the divider means something.
    pub(crate) boundary_visible: bool,
}

impl Reach {
    /// The frame a viewer on `index` is left on: the same one while it is still
    /// shown, else the last one that is, where End would take them. `min` and
    /// `max` rather than `clamp`, whose bounds cross on an empty reach.
    pub(crate) fn kept(&self, index: i32) -> i32 {
        let shown = i32::try_from(self.captions.len()).unwrap_or(i32::MAX);
        index.min(shown - 1).max(0)
    }

    /// Whether the shown frames can play: `play`'s rule, and QML's `playable`.
    pub(crate) fn playable(&self) -> bool {
        self.captions.len() > 1
    }
}

impl ffi::RadarController {
    /// Ends the animation at the rain graph's last bar, `label`; an empty label
    /// cuts nothing. The end is stored at once, so that a run adopted next is
    /// cut at it, and the run on screen is cut again on the next turn of the
    /// event loop, for the reason the module doc gives.
    pub fn track_rain_end(mut self: Pin<&mut Self>, label: &QString) {
        self.as_mut().rust_mut().rain_end_label = label.to_string();
        if let Err(error) = self.qt_thread().queue(Self::recut) {
            eprintln!("allesbuien: the rain graph's end never reached the radar: {error}");
        }
    }

    /// Shows `run` from its first frame, cut at the rain graph's end known now,
    /// playing when more than one frame is shown.
    pub(crate) fn adopt_run(mut self: Pin<&mut Self>, run: AdoptedRun) {
        let reach = run.reach(&self.rain_end_label);
        self.as_mut().rust_mut().adopted = run;
        self.as_mut().set_current_frame_index(0);
        self.as_mut().publish_reach(&reach);
        self.set_playing(reach.playable());
    }

    /// Cuts the adopted run again at the end stored last, keeping the viewer's
    /// frame while it is still shown and never starting playback; nothing while
    /// a refresh is in flight, whose adoption reads the end itself.
    fn recut(mut self: Pin<&mut Self>) {
        if self.adopted.labels.is_empty() {
            return;
        }
        let reach = self.adopted.reach(&self.rain_end_label);
        let index = reach.kept(*self.current_frame_index());
        let playing = *self.playing() && reach.playable();
        // The frame moves before the count shrinks, so that the slider's value
        // never passes its `to`.
        self.as_mut().set_current_frame_index(index);
        self.as_mut().publish_reach(&reach);
        self.set_playing(playing);
    }

    /// Publishes `reach` and the frame on screen within it; the caller has
    /// already put the current index inside `reach`.
    fn publish_reach(mut self: Pin<&mut Self>, reach: &Reach) {
        let shown = i32::try_from(reach.captions.len()).unwrap_or(i32::MAX);
        let first = i32::try_from(reach.first_forecast_index).unwrap_or(i32::MAX);
        let captions: QStringList = reach.captions.iter().map(QString::from).collect();
        self.as_mut().set_first_forecast_index(first);
        self.as_mut().set_observed_fraction(reach.observed_fraction);
        self.as_mut()
            .set_forecast_boundary_visible(reach.boundary_visible);
        self.as_mut().set_frame_count(shown);
        self.as_mut().set_timestamps(captions);
        self.update_current_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::AdoptedRun;

    /// Ten frames, 10:00 to 10:45, two of them observed.
    fn run() -> AdoptedRun {
        AdoptedRun {
            labels: (0..10).map(|step| format!("10:{:02}", step * 5)).collect(),
            first_forecast_index: 2,
        }
    }

    #[test]
    fn without_a_rain_graph_the_whole_run_is_shown() {
        // Given / When
        let reach = run().reach("");

        // Then
        assert_eq!(reach.captions, run().labels);
        assert!((reach.observed_fraction - 1.5 / 9.0).abs() < 1e-12);
        assert!(reach.boundary_visible);
    }

    #[test]
    fn the_divider_is_measured_against_the_frames_shown() {
        // Given / When: a graph ending on frame 6.
        let reach = run().reach("10:30");

        // Then: a track of seven frames, not ten.
        assert_eq!(reach.captions.len(), 7);
        assert_eq!(reach.captions.last().map(String::as_str), Some("10:30"));
        assert_eq!(reach.first_forecast_index, 2);
        let fraction = reach.observed_fraction;
        assert!((fraction - 1.5 / 6.0).abs() < 1e-12, "fraction {fraction}");
        assert!(reach.boundary_visible);
    }

    #[test]
    fn a_viewer_keeps_their_frame_unless_the_cut_hides_it() {
        // Given
        let reach = run().reach("10:30");

        // When / Then
        assert_eq!(reach.kept(2), 2);
        assert_eq!(reach.kept(6), 6);
        assert_eq!(
            reach.kept(9),
            6,
            "a hidden frame leaves them on the last shown"
        );
        assert_eq!(
            AdoptedRun::default().reach("10:30").kept(0),
            0,
            "no run at all"
        );
    }

    #[test]
    fn a_reach_of_one_frame_cannot_play() {
        // Given: a run of predictions whose graph ends on its first frame.
        let predictions = AdoptedRun {
            first_forecast_index: 0,
            ..run()
        };

        // When / Then
        assert!(!predictions.reach("10:00").playable());
        assert!(run().reach("10:30").playable());
    }
}
