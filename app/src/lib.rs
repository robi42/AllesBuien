use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::Arc,
};

use allesbuien_core::{
    config::Config,
    notify::NotifySend,
    radar::{RadarError, ReqwestRadarApi},
    weather::{ReqwestFeedApi, WeatherError},
};
mod app_style;
mod bootstrap;
pub mod image_provider;
mod services;
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
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(feed),
        radar: Arc::new(radar),
        notifier: Arc::new(NotifySend),
        location: config.location,
    });
    let exit_code = AppBuilder::new(
        services,
        home.join(".config/omarchy/current/theme"),
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
