use std::{pin::Pin, sync::Arc};

use allesbuien_core::{
    error_chain,
    radar::{GenerationId, RadarClient, RadarError},
    radar_projection::marker_position,
    radar_timeline::{boundary_visible, observed_fraction},
    retry, status,
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};

use crate::{
    image_provider::{self, FrameBytes},
    radar_frames::{FrameSet, LoadedRefresh},
    radar_state::{RadarControllerConfig, RadarControllerRust},
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
        #[qproperty(QString, status)]
        #[qproperty(i32, refresh_interval_ms)]
        #[qproperty(i32, playback_interval_ms)]
        type RadarController = super::RadarControllerRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn play(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn pause(self: Pin<&mut RadarController>);
        #[qinvokable]
        #[cxx_name = "advanceFrame"]
        fn advance_frame(self: Pin<&mut RadarController>);
        #[qinvokable]
        fn seek(self: Pin<&mut RadarController>, index: i32);
    }

    impl cxx_qt::Threading for RadarController {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_radar_controller"]
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
        let refresh_ms = i32::try_from(config.refresh_interval.as_millis()).unwrap_or(i32::MAX);
        let playback_ms = i32::try_from(config.playback_interval.as_millis()).unwrap_or(i32::MAX);
        let marker = marker_position(config.location.latitude, config.location.longitude);
        let mut rust = pinned.as_mut().rust_mut();
        rust.api = Some(config.api);
        rust.state_dir = config.state_dir;
        rust.runtime = Some(config.runtime);
        rust.configured_refresh_ms = refresh_ms;
        pinned.as_mut().set_refresh_interval_ms(refresh_ms);
        pinned.as_mut().set_playback_interval_ms(playback_ms);
        pinned.as_mut().set_marker_visible(marker.is_some());
        if let Some(marker) = marker {
            pinned.as_mut().set_marker_x_fraction(marker.x);
            pinned.as_mut().set_marker_y_fraction(marker.y);
        }
        pinned.refresh();
    }
    controller
}

impl ffi::RadarController {
    pub fn refresh(mut self: Pin<&mut Self>) {
        let (Some(api), Some(runtime)) = (&self.api, &self.runtime) else {
            return;
        };
        let (api, runtime, state_dir) = (Arc::clone(api), runtime.clone(), self.state_dir.clone());
        let generation = self.generation().saturating_add(1);
        self.as_mut().set_generation(generation);
        self.as_mut().set_frame_count(0);
        self.as_mut().set_current_frame_index(0);
        self.as_mut().set_playing(false);
        self.as_mut().set_timestamps(QStringList::default());
        self.as_mut().set_current_frame_label(QString::default());
        self.as_mut().set_current_frame_url(QString::default());
        self.as_mut().set_status("Loading radar frames".into());
        image_provider::begin_generation(generation);

        let qt_thread = self.qt_thread();
        drop(runtime.spawn(async move {
            let client = RadarClient::new(api, state_dir);
            let mut result = client.refresh(GenerationId(generation)).await;
            for attempt in 2..=retry::MAX_ATTEMPTS {
                match &result {
                    Err(error) if retry::is_retryable(error) => {}
                    _ => break,
                }
                tokio::time::sleep(retry::backoff(attempt)).await;
                result = client.refresh(GenerationId(generation)).await;
            }
            let loaded = match result {
                Ok(refresh) => refresh
                    .frames
                    .iter()
                    .map(|frame| std::fs::read(&frame.path).map(FrameBytes::from))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|frames| LoadedRefresh { refresh, frames })
                    .map_err(RadarError::Cache),
                Err(error) => Err(error),
            };
            let _queue_result = qt_thread.queue(move |controller| {
                controller.apply_result(GenerationId(generation), loaded);
            });
        }));
    }

    fn apply_result(
        mut self: Pin<&mut Self>,
        generation: GenerationId,
        result: Result<LoadedRefresh, RadarError>,
    ) {
        if generation.0 != *self.generation() {
            return;
        }
        match result {
            Ok(loaded) => {
                let set = FrameSet::from_loaded(loaded);
                let configured = self.configured_refresh_ms;
                self.as_mut().apply_frame_set(generation.0, &set);
                self.as_mut().rust_mut().last_good = Some(set);
                self.as_mut().set_status(QString::default());
                self.set_refresh_interval_ms(configured);
            }
            Err(error) => {
                let stale = self.last_good.is_some();
                if let Some(set) = self.last_good.clone() {
                    self.as_mut().apply_frame_set(generation.0, &set);
                }
                eprintln!(
                    "allesbuien: radar refresh failed: {}",
                    error_chain::describe(&error)
                );
                let retry_ms = i32::try_from(retry::RETRY_AFTER_FAILURE.as_millis())
                    .unwrap_or(self.configured_refresh_ms);
                self.as_mut()
                    .set_status(status::radar_message(&error, stale).into());
                self.set_refresh_interval_ms(retry_ms);
            }
        }
    }

    fn apply_frame_set(mut self: Pin<&mut Self>, generation: u64, set: &FrameSet) {
        image_provider::replace_frames(generation, set.frames.clone());
        let timeline = set.timeline();
        let (first, count) = (timeline.first_forecast_index, set.timestamps.len());
        self.as_mut()
            .set_first_forecast_index(i32::try_from(first).unwrap_or(i32::MAX));
        self.as_mut()
            .set_observed_fraction(observed_fraction(first, count));
        self.as_mut()
            .set_forecast_boundary_visible(boundary_visible(first, count));
        self.as_mut().rust_mut().observed = timeline.observed;
        self.as_mut()
            .set_frame_count(i32::try_from(set.timestamps.len()).unwrap_or(i32::MAX));
        self.as_mut().set_current_frame_index(0);
        self.as_mut().set_playing(set.timestamps.len() > 1);
        self.as_mut().set_timestamps(
            set.timestamps
                .iter()
                .map(QString::from)
                .collect::<QStringList>(),
        );
        self.as_mut().update_current_frame();
    }

    pub fn play(mut self: Pin<&mut Self>) {
        let can_play = *self.frame_count() > 1;
        self.as_mut().set_playing(can_play);
    }

    pub fn pause(mut self: Pin<&mut Self>) {
        self.as_mut().set_playing(false);
    }

    pub fn advance_frame(mut self: Pin<&mut Self>) {
        if !*self.playing() || *self.frame_count() == 0 {
            return;
        }
        let next = (*self.current_frame_index() + 1) % *self.frame_count();
        self.as_mut().set_current_frame_index(next);
        self.update_current_frame();
    }

    pub fn seek(mut self: Pin<&mut Self>, index: i32) {
        let last = self.frame_count().saturating_sub(1);
        if last < 0 {
            return;
        }
        self.as_mut().set_playing(false);
        self.as_mut().set_current_frame_index(index.clamp(0, last));
        self.update_current_frame();
    }

    fn update_current_frame(mut self: Pin<&mut Self>) {
        let index = isize::try_from(*self.current_frame_index()).unwrap_or_default();
        let label = self
            .timestamps()
            .get(index)
            .map_or_else(QString::default, Clone::clone);
        let url = format!(
            "image://radar/{}/{}",
            self.generation(),
            self.current_frame_index()
        );
        let observed = usize::try_from(index)
            .ok()
            .and_then(|index| self.observed.get(index).copied())
            .unwrap_or(true);
        self.as_mut().set_current_frame_observed(observed);
        self.as_mut().set_current_frame_label(label);
        self.set_current_frame_url(url.into());
    }
}
