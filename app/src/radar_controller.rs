//! Radar bridge: playback state and frame URLs for QML.
//!
//! Every refresh carries a [`GenerationId`], so a reply that lands after the
//! viewer has moved on is recognised and dropped rather than repainting the
//! window with frames from a run nobody is watching.
//!
//! How far into a run the transport reaches — up to the rain graph's last bar —
//! is [`crate::radar_reach`]'s, which cuts each run as it is adopted here.
//!
//! `#[auto_cxx_name]` on the `extern "RustQt"` block is load-bearing: without
//! it, each multi-word `#[qproperty]` is exported under its `snake_case` Rust
//! name and the camelCase binding in QML silently resolves to `undefined`.

use std::{pin::Pin, sync::Arc};

use allesbuien_core::{
    error_chain,
    location::LocationConfig,
    location_notice,
    radar::{GenerationId, RadarError},
    radar_projection::marker_position,
    radar_timeline::RadarTimeline,
    retry, status,
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};

use crate::{
    image_provider, location_follow,
    qt_values::{interval_ms, or_empty},
    radar_frames::{FrameSet, LoadedRefresh},
    radar_reach::AdoptedRun,
    radar_state::{RadarControllerConfig, RadarControllerRust},
    refresh_job,
};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, frame_count)]
        #[qproperty(i32, current_frame_index)]
        #[qproperty(bool, playing)]
        #[qproperty(u64, generation)]
        #[qproperty(QStringList, timestamps)]
        #[qproperty(QString, current_frame_label)]
        #[qproperty(QString, current_frame_url)]
        #[qproperty(i32, first_forecast_index)]
        #[qproperty(bool, current_frame_observed)]
        #[qproperty(f64, observed_fraction)]
        #[qproperty(bool, forecast_boundary_visible)]
        #[qproperty(f64, marker_x_fraction)]
        #[qproperty(f64, marker_y_fraction)]
        #[qproperty(bool, marker_visible)]
        #[qproperty(QStringList, legend_swatches)]
        #[qproperty(QStringList, legend_buckets)]
        #[qproperty(QString, legend_unit)]
        #[qproperty(QString, coverage_notice)]
        #[qproperty(QString, loading_notice)]
        #[qproperty(QString, status)]
        #[qproperty(i32, refresh_interval_ms)]
        #[qproperty(i32, playback_interval_ms)]
        type RadarController = super::RadarControllerRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn poll_location(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn play(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn pause(self: Pin<&mut RadarController>);
        #[qinvokable]
        #[cxx_name = "advanceFrame"]
        fn advance_frame(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn seek(self: Pin<&mut RadarController>, index: i32);
        #[qinvokable]
        fn track_rain_end(self: Pin<&mut RadarController>, label: &QString);
    }

    impl cxx_qt::Threading for RadarController {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_radar_controller"]
        #[must_use]
        fn make_unique() -> UniquePtr<RadarController>;
    }

    unsafe extern "C++" {
        include!("radar_controller.h");
        #[rust_name = "set_radar_controller"]
        fn setRadarController(
            engine: Pin<&mut QQmlApplicationEngine>,
            controller: Pin<&mut RadarController>,
        );
    }
}

pub(crate) fn create(config: RadarControllerConfig) -> cxx::UniquePtr<ffi::RadarController> {
    let mut controller = ffi::new_radar_controller();
    if let Some(mut pinned) = controller.as_mut() {
        let refresh_ms = interval_ms(config.intervals.refresh);
        let playback_ms = interval_ms(config.intervals.playback);
        let location = config.location.resolve();
        let mut rust = pinned.as_mut().rust_mut();
        rust.api = Some(config.api);
        rust.state_dir = config.state_dir;
        rust.location_settings = Some(config.location);
        rust.runtime = Some(config.runtime);
        rust.configured_refresh_ms = refresh_ms;
        pinned.as_mut().set_refresh_interval_ms(refresh_ms);
        pinned.as_mut().set_playback_interval_ms(playback_ms);
        pinned.as_mut().adopt_location(&location);
        pinned.refresh();
    }
    controller
}

impl ffi::RadarController {
    /// Re-asks the desktop where the viewer is, and re-runs the radar when it moved.
    ///
    /// [`location_follow`] says why this is a poll and why each bridge polls alone.
    pub fn poll_location(mut self: Pin<&mut Self>) {
        let Some(moved) =
            location_follow::moved_to(self.location_settings.as_deref(), self.location.as_ref())
        else {
            return;
        };
        self.as_mut().adopt_location(&moved);
        self.refresh();
    }

    /// Moves the marker onto the new coordinates, and says so when it cannot.
    fn adopt_location(mut self: Pin<&mut Self>, location: &LocationConfig) {
        let marker = marker_position(location.latitude, location.longitude);
        self.as_mut().set_marker_visible(marker.is_some());
        if let Some(marker) = marker {
            self.as_mut().set_marker_x_fraction(marker.x);
            self.as_mut().set_marker_y_fraction(marker.y);
        }
        let notice = location_notice::coverage_notice(location);
        self.as_mut().set_coverage_notice(or_empty(notice));
        self.as_mut().rust_mut().location = Some(location.clone());
    }

    fn reset_for_refresh(mut self: Pin<&mut Self>, generation: u64) {
        self.as_mut().set_generation(generation);
        self.as_mut().set_frame_count(0);
        self.as_mut().set_current_frame_index(0);
        self.as_mut().set_playing(false);
        self.as_mut().set_timestamps(QStringList::default());
        self.as_mut().set_current_frame_label(QString::default());
        self.as_mut().set_current_frame_url(QString::default());
        self.as_mut().set_status(QString::default());
        self.as_mut()
            .set_loading_notice(status::RADAR_LOADING.into());
        // No run to cut until this refresh answers. The rain graph's end stays:
        // it is the weather's, and an unchanged end is never handed over again.
        self.rust_mut().adopted = AdoptedRun::default();
    }

    pub fn refresh(mut self: Pin<&mut Self>) {
        let (Some(api), Some(runtime)) = (&self.api, &self.runtime) else {
            return;
        };
        let (api, runtime, state_dir) = (Arc::clone(api), runtime.clone(), self.state_dir.clone());
        let generation = self.generation().saturating_add(1);
        self.as_mut().reset_for_refresh(generation);
        image_provider::begin_generation(generation);

        let previous = self.as_mut().rust_mut().pending_refresh.take();
        if let Some(previous) = previous.as_ref() {
            previous.abort();
        }
        let qt_thread = self.qt_thread();
        let pending = runtime.spawn(async move {
            if let Some(previous) = previous {
                drop(previous.await);
            }
            let loaded =
                refresh_job::load_radar_run(api, state_dir, GenerationId(generation)).await;
            if let Err(error) = qt_thread.queue(move |controller| {
                controller.apply_result(GenerationId(generation), loaded);
            }) {
                eprintln!("allesbuien: radar update never reached the window: {error}");
            }
        });
        self.as_mut().rust_mut().pending_refresh = Some(pending);
    }

    fn apply_result(
        mut self: Pin<&mut Self>,
        generation: GenerationId,
        result: Result<LoadedRefresh, RadarError>,
    ) {
        if generation.0 != *self.generation() {
            return;
        }
        self.as_mut().set_loading_notice(QString::default());
        match result {
            Ok(loaded) => {
                let set = FrameSet::from_loaded(loaded);
                let configured = self.configured_refresh_ms;
                self.as_mut().apply_frame_set(generation.0, &set);
                self.as_mut().rust_mut().last_good = Some(set);
                self.as_mut().set_status(QString::default());
                self.set_refresh_interval_ms(configured);
            }
            Err(error) => self.apply_failure(generation.0, &error),
        }
    }

    /// Keeps the last good run on screen, states the failure, and retries sooner.
    fn apply_failure(mut self: Pin<&mut Self>, generation: u64, error: &RadarError) {
        let stale = self.last_good.is_some();
        if let Some(set) = self.last_good.clone() {
            self.as_mut().apply_frame_set(generation, &set);
        }
        eprintln!(
            "allesbuien: radar refresh failed: {}",
            error_chain::describe(error)
        );
        self.as_mut()
            .set_status(status::radar_message(error, stale).into());
        self.set_refresh_interval_ms(interval_ms(retry::RETRY_AFTER_FAILURE));
    }

    /// Puts `set` on screen from its first frame, cut where the rain graph ends.
    fn apply_frame_set(mut self: Pin<&mut Self>, generation: u64, set: &FrameSet) {
        image_provider::replace_frames(generation, set.frames.clone());
        let RadarTimeline {
            first_forecast_index,
            observed,
        } = set.timeline();
        self.as_mut().rust_mut().observed_by_frame = observed;
        self.adopt_run(AdoptedRun {
            labels: set.timestamps.clone(),
            first_forecast_index,
        });
    }
}
