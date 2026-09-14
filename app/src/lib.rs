//! The application, assembled: configuration in, exit status out.
//!
//! [`run`] is the whole program — read the config, build the real HTTP, cache,
//! and notification services, hand them to [`AppBuilder`], and enter Qt's event
//! loop. It lives in a library rather than in `main.rs` because the GUI tests
//! link this crate and drive the same construction path with fakes substituted
//! at the [`CoreServiceConfig`] seam; a `main.rs` that wired things up itself
//! would be a second, untested assembly of the same app.

use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::Arc,
};

use crate::icon_cache::IconCache;
use allesbuien_core::{
    config::Config,
    notify::NotifySend,
    radar::RadarError,
    radar_transport::ReqwestRadarApi,
    weather::{ReqwestFeedApi, WeatherError},
};
mod app_style;
mod bootstrap;
pub mod condition_icon;
pub mod conditions_view;
pub mod forecast_model;
pub mod icon_cache;
mod icon_delivery;
pub mod icon_store;
#[expect(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::unnecessary_box_returns,
    reason = "the image-provider module contains CXX-Qt constructor plumbing whose generated signatures and boxed QObject allocation are fixed by its C++ ABI"
)]
pub mod image_provider;
mod location_follow;
mod qt_values;
pub mod radar_clock;
#[expect(
    clippy::float_cmp,
    reason = "the radar bridge module contains CXX-Qt generated exact equality for f64 Q_PROPERTY change detection"
)]
pub mod radar_controller;
mod radar_frames;
mod radar_playback;
mod radar_state;
pub mod rain_graph;
mod refresh_job;
mod services;
pub mod theme_controller;
#[expect(
    clippy::float_cmp,
    reason = "the weather bridge module contains CXX-Qt generated exact equality for f64 Q_PROPERTY change detection"
)]
pub mod weather_controller;
mod weather_state;

pub const BASEMAP_PNG: &[u8] = include_bytes!("../assets/basemap-nl.png");
pub use bootstrap::{AppBuilder, BootedApp, BootstrapError};
pub use services::{CoreServiceConfig, CoreServices, NotifierService, TimerIntervals};

#[derive(Debug)]
pub enum RunError {
    Weather(WeatherError),
    Radar(RadarError),
    Bootstrap(BootstrapError),
    EventLoop(i32),
}

impl Display for RunError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Weather(error) => Display::fmt(error, formatter),
            Self::Radar(error) => Display::fmt(error, formatter),
            Self::Bootstrap(error) => Display::fmt(error, formatter),
            Self::EventLoop(code) => write!(formatter, "Qt event loop exited with status {code}"),
        }
    }
}

impl Error for RunError {}

/// Builds the production services, starts Qt, and runs its event loop.
///
/// # Errors
///
/// Returns [`RunError`] when a weather or radar client cannot be constructed,
/// Qt application bootstrap fails, or the Qt event loop exits unsuccessfully.
pub fn run() -> Result<(), RunError> {
    let config = Config::load();
    let feed = ReqwestFeedApi::new(config.endpoints.feed_url, config.endpoints.raintext_url)
        .map_err(RunError::Weather)?;
    let radar = ReqwestRadarApi::new(&config.endpoints.radar_url).map_err(RunError::Radar)?;
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let icons = icon_cache::ReqwestIconSource::new().map_or_else(IconCache::disabled, |source| {
        IconCache::new(
            icon_store::cache_directory(
                std::env::var_os("XDG_CACHE_HOME")
                    .map(PathBuf::from)
                    .as_deref(),
                &home,
            ),
            Arc::new(source),
        )
    });
    let services = CoreServices::new_with_notify(
        CoreServiceConfig {
            feed: Arc::new(feed),
            radar: Arc::new(radar),
            notifier: Arc::new(NotifySend),
            location: config.location,
            radar_state_dir: config.state_dir,
        },
        config.notify,
    );
    let exit_code = AppBuilder::new(
        services,
        home.join(".local/state/omarchy/current/theme"),
        TimerIntervals::default(),
    )
    .with_icons(icons)
    .boot()
    .map_err(RunError::Bootstrap)?
    .exec();

    match exit_code {
        0 => Ok(()),
        code => Err(RunError::EventLoop(code)),
    }
}
