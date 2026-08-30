use std::{sync::Arc, time::Duration};

use allesbuien_core::{
    config::LocationConfig, notify::Notifier, radar::RadarApi, weather::FeedApi,
};
use tokio::runtime::{Builder, Handle, Runtime};

pub type NotifierService = dyn Notifier + Send + Sync;

pub struct CoreServiceConfig {
    pub feed: Arc<dyn FeedApi>,
    pub radar: Arc<dyn RadarApi>,
    pub notifier: Arc<NotifierService>,
    pub location: LocationConfig,
}

pub struct CoreServices {
    feed: Arc<dyn FeedApi>,
    radar: Arc<dyn RadarApi>,
    notifier: Arc<NotifierService>,
    location: LocationConfig,
}

impl CoreServices {
    #[must_use]
    pub fn new(config: CoreServiceConfig) -> Self {
        Self {
            feed: config.feed,
            radar: config.radar,
            notifier: config.notifier,
            location: config.location,
        }
    }

    #[must_use]
    pub fn feed(&self) -> &Arc<dyn FeedApi> {
        &self.feed
    }

    #[must_use]
    pub fn radar(&self) -> &Arc<dyn RadarApi> {
        &self.radar
    }

    #[must_use]
    pub fn notifier(&self) -> &Arc<NotifierService> {
        &self.notifier
    }

    #[must_use]
    pub const fn location(&self) -> &LocationConfig {
        &self.location
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TimerIntervals {
    pub weather_refresh: Duration,
    pub radar_refresh: Duration,
    pub radar_playback: Duration,
    pub theme_poll: Duration,
}

impl Default for TimerIntervals {
    fn default() -> Self {
        Self {
            weather_refresh: Duration::from_secs(10 * 60),
            radar_refresh: Duration::from_secs(5 * 60),
            radar_playback: Duration::from_millis(400),
            theme_poll: Duration::from_secs(1),
        }
    }
}

pub(crate) struct AppRuntime(Runtime);

impl AppRuntime {
    pub(crate) fn new() -> std::io::Result<Self> {
        Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("allesbuien-core")
            .enable_all()
            .build()
            .map(Self)
    }

    pub(crate) fn handle(&self) -> Handle {
        self.0.handle().clone()
    }
}
