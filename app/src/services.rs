//! The injected side effects: HTTP, the Tokio runtime, and notification dispatch.
//!
//! Nothing here is global, and that is the point: [`CoreServiceConfig`] is the
//! single seam every test substitutes at, so a fake feed or a fake notifier is
//! a constructor argument rather than a patched module.
//!
//! [`AppRuntime`] owns the Tokio runtime the controllers spawn onto, which is
//! what keeps HTTP, file reads, and `notify-send` off the Qt thread.

use std::{path::PathBuf, sync::Arc, time::Duration};

use allesbuien_core::{
    location_settings::LocationSettings,
    notify::{
        NotificationInput, Notifier, NotifyConfig, NotifyError, NotifyState,
        RainSoon as NotifyRainSoon, maybe_send_rain_notification,
    },
    radar::RadarApi,
    weather::{FeedApi, RainSoon},
};
use tokio::runtime::{Builder, Handle, Runtime};

pub type NotifierService = dyn Notifier + Send + Sync;

pub struct CoreServiceConfig {
    pub feed: Arc<dyn FeedApi>,
    pub radar: Arc<dyn RadarApi>,
    pub notifier: Arc<NotifierService>,
    pub location: LocationSettings,
    pub radar_state_dir: PathBuf,
}

pub struct CoreServices {
    feed: Arc<dyn FeedApi>,
    radar: Arc<dyn RadarApi>,
    notifier: Arc<NotifierService>,
    location: Arc<LocationSettings>,
    radar_state_dir: PathBuf,
    notify_config: NotifyConfig,
}

impl CoreServices {
    #[must_use]
    pub fn new(config: CoreServiceConfig) -> Self {
        Self::new_with_notify(
            config,
            NotifyConfig {
                enabled: false,
                threshold_minutes: 0,
                cooldown: Duration::ZERO,
            },
        )
    }

    #[must_use]
    pub fn new_with_notify(config: CoreServiceConfig, notify_config: NotifyConfig) -> Self {
        Self {
            feed: config.feed,
            radar: config.radar,
            notifier: config.notifier,
            location: Arc::new(config.location),
            radar_state_dir: config.radar_state_dir,
            notify_config,
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
    pub const fn notify_config(&self) -> NotifyConfig {
        self.notify_config
    }

    #[must_use]
    pub const fn location(&self) -> &Arc<LocationSettings> {
        &self.location
    }

    #[must_use]
    pub fn radar_state_dir(&self) -> &PathBuf {
        &self.radar_state_dir
    }
}

#[derive(Clone)]
pub(crate) struct NotificationService {
    notifier: Arc<NotifierService>,
    state: Arc<tokio::sync::Mutex<NotifyState>>,
}

impl NotificationService {
    pub(crate) fn new(notifier: Arc<NotifierService>, config: NotifyConfig) -> Self {
        Self {
            notifier,
            state: Arc::new(tokio::sync::Mutex::new(NotifyState {
                config,
                last_sent_at: None,
                last_event_key: None,
            })),
        }
    }

    /// The place is an argument rather than stored state because the desktop can
    /// move it while the app runs, and a title naming where the viewer *was* is
    /// worse than no title at all.
    pub(crate) async fn notify_rain(
        &self,
        rain: Option<&RainSoon>,
        location_label: &str,
    ) -> Result<(), NotifyError> {
        let rain = rain.map(|rain| NotifyRainSoon {
            minutes_until: rain.minutes_until,
            time_label: &rain.time_label,
            max_mmh: rain.max_mmh,
        });
        let mut state = self.state.lock().await;
        maybe_send_rain_notification(
            &NotifierAdapter(self.notifier.as_ref()),
            NotificationInput {
                rain: rain.as_ref(),
                location_label,
            },
            &mut state,
        )
    }
}

struct NotifierAdapter<'a>(&'a NotifierService);

impl Notifier for NotifierAdapter<'_> {
    fn send(&self, title: &str, body: &str) -> Result<(), NotifyError> {
        self.0.send(title, body)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TimerIntervals {
    pub weather_refresh: Duration,
    pub radar_refresh: Duration,
    pub radar_playback: Duration,
    pub theme_poll: Duration,
    pub location_poll: Duration,
}

impl Default for TimerIntervals {
    fn default() -> Self {
        Self {
            weather_refresh: Duration::from_secs(10 * 60),
            radar_refresh: Duration::from_secs(5 * 60),
            radar_playback: Duration::from_millis(400),
            theme_poll: Duration::from_secs(2),
            location_poll: Duration::from_secs(2),
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
