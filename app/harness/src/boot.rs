//! One booted window over injected ports, with inert defaults for the rest.
//!
//! Every harness boots the same way: services from a feed, a radar, a notifier,
//! and a location; an [`AppBuilder`] over a theme path and timer intervals.
//! Most harnesses vary one of those and leave the others inert, so the builder
//! starts from the stubs and each harness names only what its scenario is about.

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use allesbuien::{
    AppBuilder, BootedApp, CoreServiceConfig, CoreServices, NotifierService, TimerIntervals,
};
use allesbuien_core::{
    location_settings::LocationSettings, notify::NotifyConfig, radar::RadarApi, weather::FeedApi,
};
use allesbuien_testkit::{StubNotifier, StubRadarApi, amsterdam_west};
use tempfile::TempDir;

/// A theme directory that never exists, so the window uses its fallback palette.
const ABSENT_THEME: &str = "/nonexistent/test-theme";

/// A booted window and the radar cache directory it writes into.
///
/// Fields drop in declaration order, so the application — and the runtime
/// still writing frames — is gone before its cache directory is removed.
pub struct Harness {
    app: BootedApp,
    _radar_state: TempDir,
}

impl Deref for Harness {
    type Target = BootedApp;

    fn deref(&self) -> &BootedApp {
        &self.app
    }
}

impl DerefMut for Harness {
    fn deref_mut(&mut self) -> &mut BootedApp {
        &mut self.app
    }
}

/// The ports and settings one harness boots with.
pub struct HarnessBuilder {
    feed: Arc<dyn FeedApi>,
    radar: Arc<dyn RadarApi>,
    notifier: Arc<NotifierService>,
    notify_config: Option<NotifyConfig>,
    location: LocationSettings,
    theme_path: PathBuf,
    intervals: TimerIntervals,
}

impl HarnessBuilder {
    /// Starts from `feed`, a failing stub radar, a silent notifier with
    /// notifications off, the pinned Amsterdam-West location, an absent theme,
    /// and the production timer intervals.
    #[must_use]
    pub fn with_feed(feed: Arc<dyn FeedApi>) -> Self {
        Self {
            feed,
            radar: Arc::new(StubRadarApi),
            notifier: Arc::new(StubNotifier),
            notify_config: None,
            location: amsterdam_west(),
            theme_path: PathBuf::from(ABSENT_THEME),
            intervals: TimerIntervals::default(),
        }
    }

    /// Serves the radar from `radar` instead of the failing stub.
    #[must_use]
    pub fn radar(mut self, radar: Arc<dyn RadarApi>) -> Self {
        self.radar = radar;
        self
    }

    /// Dispatches through `notifier` under `config`, instead of never notifying.
    #[must_use]
    pub fn notifications(mut self, notifier: Arc<NotifierService>, config: NotifyConfig) -> Self {
        self.notifier = notifier;
        self.notify_config = Some(config);
        self
    }

    /// Resolves the location from `location`, e.g. a watched temporary home.
    #[must_use]
    pub fn location(mut self, location: LocationSettings) -> Self {
        self.location = location;
        self
    }

    /// Reads the Omarchy theme from `theme_path` instead of an absent directory.
    #[must_use]
    pub fn theme_path(mut self, theme_path: PathBuf) -> Self {
        self.theme_path = theme_path;
        self
    }

    /// Drives the controllers' timers at `intervals`.
    #[must_use]
    pub fn intervals(mut self, intervals: TimerIntervals) -> Self {
        self.intervals = intervals;
        self
    }

    /// Creates this process's one `QGuiApplication` and loads the root window.
    ///
    /// # Panics
    ///
    /// When the radar cache directory cannot be created or the window does not
    /// boot; either way the harness has nothing to assert against.
    #[must_use]
    pub fn boot(self) -> Harness {
        let radar_state = tempfile::tempdir().expect("a temporary radar cache directory");
        let config = CoreServiceConfig {
            feed: self.feed,
            radar: self.radar,
            notifier: self.notifier,
            location: self.location,
            radar_state_dir: radar_state.path().to_owned(),
        };
        let services = match self.notify_config {
            Some(notify_config) => CoreServices::new_with_notify(config, notify_config),
            None => CoreServices::new(config),
        };
        let app = AppBuilder::new(services, self.theme_path, self.intervals)
            .boot()
            .expect("the fake-wired GUI should boot");
        Harness {
            app,
            _radar_state: radar_state,
        }
    }
}
