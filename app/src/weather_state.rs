//! Construction inputs and stored state for the weather bridge.
//!
//! Split out of `weather_controller` so that file stays the QML property surface
//! and the behaviour behind it, rather than also carrying the field list.

use std::{sync::Arc, time::Duration};

use allesbuien_core::{
    location::LocationConfig,
    location_settings::LocationSettings,
    rain::RainPoint,
    status::RAIN_UNAVAILABLE,
    weather::{FeedApi, ForecastDay},
};
use cxx_qt_lib::{QList, QString, QStringList};
use tokio::runtime::Handle;

use crate::{
    conditions_view::ConditionsView,
    icon_cache::IconCache,
    icon_delivery::PendingIcons,
    qt_values::interval_ms,
    services::{NotificationService, TimerIntervals},
};

pub(crate) struct WeatherControllerConfig {
    pub(crate) feed: Arc<dyn FeedApi>,
    pub(crate) icons: IconCache,
    pub(crate) location: Arc<LocationSettings>,
    pub(crate) notifications: NotificationService,
    pub(crate) runtime: Handle,
    pub(crate) intervals: WeatherIntervals,
}

/// The two cadences QML drives this controller with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WeatherIntervals {
    pub(crate) refresh: Duration,
    pub(crate) location_poll: Duration,
}

pub struct WeatherControllerRust {
    pub(crate) location_name: QString,
    pub(crate) location_source: QString,
    pub(crate) station_name: QString,
    pub(crate) temperature: QString,
    pub(crate) feels_like: QString,
    pub(crate) wind: QString,
    pub(crate) humidity: QString,
    pub(crate) condition: QString,
    pub(crate) condition_icon_url: QString,
    /// The addressable hero icon URL, re-published once the provider can serve it.
    pub(crate) hero_icon_url: String,
    /// Whether the hero's empty slot is still coming, rather than never coming.
    pub(crate) hero_icon_loading: bool,
    pub(crate) observed_label: QString,
    pub(crate) rain_summary: QString,
    pub(crate) rain_wet: bool,
    pub(crate) rain_levels: QList<f64>,
    pub(crate) rain_axis_labels: QStringList,
    pub(crate) rain_cursor_fraction: f64,
    pub(crate) rain_cursor_visible: bool,
    pub(crate) status: QString,
    pub(crate) refresh_interval_ms: i32,
    pub(crate) location_poll_interval_ms: i32,
    pub(crate) forecast: Vec<ForecastDay>,
    /// Retained so the radar cursor can be replaced without refetching raintext.
    pub(crate) rain_points: Vec<RainPoint>,
    /// Icon keys still downloading, which decide each slot's loading state.
    pub(crate) pending_icons: PendingIcons,
    /// Which refresh the window is showing, so a slower earlier fetch whose
    /// answer arrives after a later one's cannot put its older payload back.
    pub(crate) refresh_generation: u64,
    pub(crate) radar_frame_label: String,
    pub(crate) feed: Option<Arc<dyn FeedApi>>,
    pub(crate) icons: IconCache,
    /// The location currently on screen, compared against on every poll.
    pub(crate) location: Option<LocationConfig>,
    pub(crate) location_settings: Option<Arc<LocationSettings>>,
    pub(crate) runtime: Option<Handle>,
    pub(crate) notifications: Option<NotificationService>,
}

impl Default for WeatherControllerRust {
    fn default() -> Self {
        // The cold-failure panel is `ConditionsView`'s own default: `apply_result`
        // returns early on error and never builds a view, so these are what a
        // first failed fetch leaves on screen.
        let conditions = ConditionsView::default();
        let defaults = TimerIntervals::default();
        Self {
            location_name: QString::default(),
            location_source: QString::default(),
            station_name: (&conditions.station_name).into(),
            temperature: (&conditions.temperature).into(),
            feels_like: (&conditions.feels_like).into(),
            wind: (&conditions.wind).into(),
            humidity: (&conditions.humidity).into(),
            condition: (&conditions.condition).into(),
            condition_icon_url: (&conditions.icon_url).into(),
            hero_icon_url: String::new(),
            // Nothing has answered at construction, so the hero slot is pending
            // until the first refresh says otherwise, success or failure.
            hero_icon_loading: true,
            observed_label: (&conditions.observed_label).into(),
            rain_summary: RAIN_UNAVAILABLE.into(),
            rain_wet: false,
            rain_levels: QList::<f64>::default(),
            rain_axis_labels: QStringList::default(),
            rain_cursor_fraction: 0.0,
            rain_cursor_visible: false,
            // Empty, not a failure: the refresh construction starts is still
            // under way, and the pulsing hero above already says so.
            status: QString::default(),
            refresh_interval_ms: interval_ms(defaults.weather_refresh),
            location_poll_interval_ms: interval_ms(defaults.location_poll),
            forecast: Vec::new(),
            rain_points: Vec::new(),
            pending_icons: PendingIcons::default(),
            refresh_generation: 0,
            radar_frame_label: String::new(),
            feed: None,
            icons: IconCache::disabled(),
            location: None,
            location_settings: None,
            runtime: None,
            notifications: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WeatherControllerRust;

    #[test]
    fn a_new_bridge_claims_no_failure_before_its_first_refresh_answers() {
        // Given / When: the state the bridge holds before the refresh its
        // construction starts has answered.
        let state = WeatherControllerRust::default();

        // Then: no status line, and the hero says it is loading instead.
        assert!(state.status.is_empty(), "{}", state.status);
        assert!(state.hero_icon_loading);
    }
}
