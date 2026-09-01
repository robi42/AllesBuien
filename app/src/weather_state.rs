//! Construction inputs and stored state for the weather bridge.
//!
//! Split out of `weather_controller` so that file stays the QML property surface
//! and the behaviour behind it, rather than also carrying the field list.

use std::{sync::Arc, time::Duration};

use allesbuien_core::{
    config::LocationConfig,
    rain::RainPoint,
    weather::{FeedApi, ForecastDay},
};
use cxx_qt_lib::{QList, QString, QStringList};
use tokio::runtime::Handle;

use crate::{icon_cache::IconCache, services::NotificationService};

pub(crate) struct WeatherControllerConfig {
    pub(crate) feed: Arc<dyn FeedApi>,
    pub(crate) icons: IconCache,
    pub(crate) location: LocationConfig,
    pub(crate) notifications: NotificationService,
    pub(crate) runtime: Handle,
    pub(crate) refresh_interval: Duration,
}

pub struct WeatherControllerRust {
    pub(crate) location_name: QString,
    pub(crate) station_name: QString,
    pub(crate) temperature: QString,
    pub(crate) feels_like: QString,
    pub(crate) wind: QString,
    pub(crate) humidity: QString,
    pub(crate) condition: QString,
    pub(crate) condition_icon_url: QString,
    pub(crate) observed_label: QString,
    pub(crate) rain_summary: QString,
    pub(crate) rain_wet: bool,
    pub(crate) rain_levels: QList<f64>,
    pub(crate) rain_axis_labels: QStringList,
    pub(crate) rain_cursor_fraction: f64,
    pub(crate) rain_cursor_visible: bool,
    pub(crate) status: QString,
    pub(crate) refresh_interval_ms: i32,
    pub(crate) forecast: Vec<ForecastDay>,
    /// Retained so the radar cursor can be replaced without refetching raintext.
    pub(crate) rain_points: Vec<RainPoint>,
    pub(crate) radar_frame_label: String,
    pub(crate) feed: Option<Arc<dyn FeedApi>>,
    pub(crate) icons: IconCache,
    pub(crate) location: Option<LocationConfig>,
    pub(crate) runtime: Option<Handle>,
    pub(crate) notifications: Option<NotificationService>,
}

impl Default for WeatherControllerRust {
    fn default() -> Self {
        Self {
            location_name: QString::default(),
            station_name: "Unavailable".into(),
            temperature: "Unavailable".into(),
            feels_like: QString::default(),
            wind: "Unavailable".into(),
            humidity: QString::default(),
            condition: "Unavailable".into(),
            condition_icon_url: QString::default(),
            observed_label: QString::default(),
            rain_summary: "Rain forecast unavailable".into(),
            rain_wet: false,
            rain_levels: QList::<f64>::default(),
            rain_axis_labels: QStringList::default(),
            rain_cursor_fraction: 0.0,
            rain_cursor_visible: false,
            status: "Weather unavailable".into(),
            refresh_interval_ms: 600_000,
            forecast: Vec::new(),
            rain_points: Vec::new(),
            radar_frame_label: String::new(),
            feed: None,
            icons: IconCache::disabled(),
            location: None,
            runtime: None,
            notifications: None,
        }
    }
}
