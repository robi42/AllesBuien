//! Construction inputs and stored state for the radar bridge.
//!
//! Split out of `radar_controller` so that file stays the QML property surface
//! and the behaviour behind it, rather than also carrying the field list.

use std::{path::PathBuf, sync::Arc, time::Duration};

use allesbuien_core::{
    location::LocationConfig,
    location_settings::LocationSettings,
    radar::RadarApi,
    radar_legend::{self, UNIT},
};
use cxx_qt_lib::{QString, QStringList};
use tokio::{runtime::Handle, task::JoinHandle};

use crate::radar_frames::FrameSet;

pub(crate) struct RadarControllerConfig {
    pub(crate) api: Arc<dyn RadarApi>,
    pub(crate) state_dir: PathBuf,
    pub(crate) location: Arc<LocationSettings>,
    pub(crate) runtime: Handle,
    pub(crate) intervals: RadarIntervals,
}

/// The three cadences QML drives this controller with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RadarIntervals {
    pub(crate) refresh: Duration,
    pub(crate) playback: Duration,
    pub(crate) location_poll: Duration,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "these booleans are independent QML properties for playback, observation, forecast-boundary, and marker visibility state"
)]
pub struct RadarControllerRust {
    pub(crate) frame_count: i32,
    pub(crate) current_frame_index: i32,
    pub(crate) playing: bool,
    pub(crate) generation: u64,
    pub(crate) timestamps: QStringList,
    pub(crate) current_frame_label: QString,
    pub(crate) current_frame_url: QString,
    pub(crate) first_forecast_index: i32,
    pub(crate) current_frame_observed: bool,
    pub(crate) observed_fraction: f64,
    pub(crate) forecast_boundary_visible: bool,
    pub(crate) marker_x_fraction: f64,
    pub(crate) marker_y_fraction: f64,
    pub(crate) marker_visible: bool,
    pub(crate) legend_swatches: QStringList,
    pub(crate) legend_buckets: QStringList,
    pub(crate) legend_unit: QString,
    pub(crate) coverage_notice: QString,
    /// Non-empty exactly while a refresh is in flight; never a failure.
    pub(crate) loading_notice: QString,
    /// Non-empty exactly when the last refresh failed; never a wait.
    pub(crate) status: QString,
    pub(crate) refresh_interval_ms: i32,
    pub(crate) location_poll_interval_ms: i32,
    /// The cadence from config, restored after a failure shortens it.
    pub(crate) configured_refresh_ms: i32,
    pub(crate) playback_interval_ms: i32,
    pub(crate) api: Option<Arc<dyn RadarApi>>,
    pub(crate) state_dir: PathBuf,
    /// The location the marker currently sits on, compared against on every poll.
    pub(crate) location: Option<LocationConfig>,
    pub(crate) location_settings: Option<Arc<LocationSettings>>,
    pub(crate) runtime: Option<Handle>,
    pub(crate) pending_refresh: Option<JoinHandle<()>>,
    pub(crate) observed: Vec<bool>,
    pub(crate) last_good: Option<FrameSet>,
}

impl Default for RadarControllerRust {
    fn default() -> Self {
        Self {
            frame_count: 0,
            current_frame_index: 0,
            playing: false,
            generation: 0,
            timestamps: QStringList::default(),
            current_frame_label: QString::default(),
            current_frame_url: QString::default(),
            first_forecast_index: 0,
            current_frame_observed: true,
            observed_fraction: 1.0,
            forecast_boundary_visible: false,
            marker_x_fraction: 0.0,
            marker_y_fraction: 0.0,
            marker_visible: false,
            legend_swatches: radar_legend::ramp()
                .into_iter()
                .map(QString::from)
                .collect(),
            legend_buckets: radar_legend::labels()
                .into_iter()
                .map(QString::from)
                .collect(),
            legend_unit: UNIT.into(),
            coverage_notice: QString::default(),
            loading_notice: QString::default(),
            status: QString::default(),
            refresh_interval_ms: 300_000,
            location_poll_interval_ms: 2_000,
            configured_refresh_ms: 300_000,
            playback_interval_ms: 400,
            api: None,
            state_dir: PathBuf::new(),
            location: None,
            location_settings: None,
            runtime: None,
            pending_refresh: None,
            observed: Vec::new(),
            last_good: None,
        }
    }
}
