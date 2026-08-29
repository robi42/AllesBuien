use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use allesbuien_core::{
    config::Config,
    notify::{Notifier, NotifySend},
    radar::{RadarApi, RadarError, ReqwestRadarApi},
    weather::{FeedApi, ReqwestFeedApi, WeatherError},
};
use cxx_qt::casting::Upcast;
use cxx_qt_lib::{
    QGuiApplication, QQmlApplicationEngine, QQmlEngine, QQmlImageProviderBaseImageType, QString,
    QUrl,
};

mod app_style;
pub mod image_provider;

pub const BASEMAP_PNG: &[u8] = include_bytes!("../assets/basemap-nl.png");
const MAIN_QML_URL: &str = "qrc:/qt/qml/nl/allesbuien/qml/Main.qml";

pub type NotifierService = dyn Notifier + Send + Sync;

pub struct CoreServices {
    feed: Arc<dyn FeedApi>,
    radar: Arc<dyn RadarApi>,
    notifier: Arc<NotifierService>,
}

impl CoreServices {
    #[must_use]
    pub fn new(
        feed: Arc<dyn FeedApi>,
        radar: Arc<dyn RadarApi>,
        notifier: Arc<NotifierService>,
    ) -> Self {
        Self {
            feed,
            radar,
            notifier,
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

pub struct AppBuilder {
    services: CoreServices,
    theme_base_path: PathBuf,
    intervals: TimerIntervals,
}

impl AppBuilder {
    #[must_use]
    pub fn new(
        services: CoreServices,
        theme_base_path: PathBuf,
        intervals: TimerIntervals,
    ) -> Self {
        Self {
            services,
            theme_base_path,
            intervals,
        }
    }

    /// Creates exactly one Qt application and loads the root QML window.
    ///
    /// # Errors
    /// Returns [`BootstrapError`] if Qt cannot create the application, engine, or root object.
    pub fn boot(self) -> Result<BootedApp, BootstrapError> {
        let _controller_seams = (&self.services, &self.theme_base_path, self.intervals);
        app_style::set_material_style();
        QGuiApplication::set_desktop_file_name(&QString::from("allesbuien"));

        let mut application = QGuiApplication::new();
        let Some(mut application_pin) = application.as_mut() else {
            return Err(BootstrapError::ApplicationCreation);
        };
        application_pin
            .as_mut()
            .set_application_name(&QString::from("allesbuien"));
        application_pin
            .as_mut()
            .set_application_display_name(&QString::from("AllesBuien"));

        let mut engine = QQmlApplicationEngine::new();
        let Some(mut engine_pin) = engine.as_mut() else {
            return Err(BootstrapError::EngineCreation);
        };
        let window_created = Arc::new(AtomicBool::new(false));
        let created_signal = Arc::clone(&window_created);
        let _object_created_guard = engine_pin.as_mut().on_object_created(move |_, object, _| {
            created_signal.store(!object.is_null(), Ordering::Release);
        });

        let provider =
            image_provider::ffi::new_image_provider(QQmlImageProviderBaseImageType::Image);
        let qml_engine: core::pin::Pin<&mut QQmlEngine> = engine_pin.as_mut().upcast_pin();
        // SAFETY: Category 8 (FFI ownership): the C++ trampoline returns a fresh,
        // non-null allocation and QQmlEngine takes its sole ownership by contract.
        unsafe { qml_engine.add_image_provider(&QString::from("radar"), provider) };
        engine_pin.as_mut().load(&QUrl::from(MAIN_QML_URL));

        if !window_created.load(Ordering::Acquire) {
            return Err(BootstrapError::RootWindowCreation);
        }

        Ok(BootedApp {
            application,
            engine,
            window_created: true,
        })
    }
}

pub struct BootedApp {
    application: cxx::UniquePtr<QGuiApplication>,
    engine: cxx::UniquePtr<QQmlApplicationEngine>,
    window_created: bool,
}

impl BootedApp {
    #[must_use]
    pub fn engine_created(&self) -> bool {
        !self.engine.is_null()
    }

    #[must_use]
    pub const fn window_created(&self) -> bool {
        self.window_created
    }

    pub fn exec(mut self) -> i32 {
        self.application.as_mut().map_or(1, QGuiApplication::exec)
    }
}

#[derive(Debug)]
pub enum BootstrapError {
    ApplicationCreation,
    EngineCreation,
    RootWindowCreation,
}

impl Display for BootstrapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApplicationCreation => formatter.write_str("failed to create QGuiApplication"),
            Self::EngineCreation => formatter.write_str("failed to create QQmlApplicationEngine"),
            Self::RootWindowCreation => formatter.write_str("failed to create root QML window"),
        }
    }
}

impl Error for BootstrapError {}

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
    let services = CoreServices::new(Arc::new(feed), Arc::new(radar), Arc::new(NotifySend));
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
