use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::Arc,
};

use allesbuien_core::{
    config::Config,
    notify::{NotifyConfig, NotifySend},
    radar::{RadarError, ReqwestRadarApi},
    weather::{ReqwestFeedApi, WeatherError},
};
mod app_style;
mod bootstrap;
pub mod condition_icon;
pub mod conditions_view;
pub mod image_provider;
pub mod radar_clock;
pub mod radar_controller;
pub mod rain_graph;
mod services;
pub mod theme_controller;
pub mod weather_controller;

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

pub fn run() -> Result<(), RunError> {
    let config = Config::load();
    let feed = ReqwestFeedApi::new(config.endpoints.feed_url, config.endpoints.raintext_url)
        .map_err(RunError::Weather)?;
    let radar = ReqwestRadarApi::new(&config.endpoints.radar_url).map_err(RunError::Radar)?;
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let services = CoreServices::new_with_notify(
        CoreServiceConfig {
            feed: Arc::new(feed),
            radar: Arc::new(radar),
            notifier: Arc::new(NotifySend),
            location: config.location,
            radar_state_dir: config.state_dir,
        },
        NotifyConfig {
            enabled: config.notify.enabled,
            threshold_minutes: config.notify.threshold_minutes,
            cooldown: config.notify.cooldown,
        },
    );
    let exit_code = AppBuilder::new(
        services,
        home.join(".local/state/omarchy/current/theme"),
        TimerIntervals::default(),
    )
    .boot()
    .map_err(RunError::Bootstrap)?
    .exec();

    match exit_code {
        0 => Ok(()),
        code => Err(RunError::EventLoop(code)),
    }
}
