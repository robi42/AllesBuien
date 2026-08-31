use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{
    QGuiApplication, QQmlApplicationEngine, QQmlEngine, QQmlImageProviderBaseImageType, QString,
    QUrl,
};

use crate::{
    CoreServices, TimerIntervals, app_style, image_provider,
    radar_controller::{self, RadarControllerConfig},
    services::{AppRuntime, NotificationService},
    theme_controller::{self, ThemeControllerConfig},
    weather_controller::{self, WeatherControllerConfig},
};

const MAIN_QML_URL: &str = "qrc:/qt/qml/nl/allesbuien/qml/Main.qml";

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
        let runtime = AppRuntime::new().map_err(BootstrapError::RuntimeCreation)?;
        let mut weather_controller = weather_controller::create(WeatherControllerConfig {
            feed: Arc::clone(self.services.feed()),
            location: self.services.location().clone(),
            notifications: NotificationService::new(
                Arc::clone(self.services.notifier()),
                self.services.location().label.clone(),
                self.services.notify_config(),
            ),
            runtime: runtime.handle(),
            refresh_interval: self.intervals.weather_refresh,
        });
        let Some(weather_controller_pin) = weather_controller.as_mut() else {
            return Err(BootstrapError::ControllerCreation);
        };
        // Controller modules reuse this seam: inject ports and the shared runtime, bind before
        // QML load, and marshal every async result through CxxQtThread::queue.
        weather_controller::ffi::set_weather_controller(
            engine_pin.as_mut(),
            weather_controller_pin,
        );
        let mut radar_controller = radar_controller::create(RadarControllerConfig {
            api: Arc::clone(self.services.radar()),
            state_dir: self.services.radar_state_dir().clone(),
            runtime: runtime.handle(),
            refresh_interval: self.intervals.radar_refresh,
            playback_interval: self.intervals.radar_playback,
        });
        let Some(radar_controller_pin) = radar_controller.as_mut() else {
            return Err(BootstrapError::ControllerCreation);
        };
        radar_controller::ffi::set_radar_controller(engine_pin.as_mut(), radar_controller_pin);
        let poll_interval_ms =
            i32::try_from(self.intervals.theme_poll.as_millis()).unwrap_or(i32::MAX);
        let mut theme_controller = theme_controller::create(ThemeControllerConfig {
            theme_base_path: self.theme_base_path,
            poll_interval_ms,
        });
        let Some(theme_controller_pin) = theme_controller.as_mut() else {
            return Err(BootstrapError::ControllerCreation);
        };
        theme_controller::ffi::set_theme_controller(engine_pin.as_mut(), theme_controller_pin);
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
            engine,
            weather_controller,
            radar_controller,
            theme_controller,
            _runtime: runtime,
            application,
            window_created: true,
        })
    }
}

pub struct BootedApp {
    engine: cxx::UniquePtr<QQmlApplicationEngine>,
    weather_controller: cxx::UniquePtr<weather_controller::ffi::WeatherController>,
    radar_controller: cxx::UniquePtr<radar_controller::ffi::RadarController>,
    theme_controller: cxx::UniquePtr<theme_controller::ffi::ThemeController>,
    _runtime: AppRuntime,
    application: cxx::UniquePtr<QGuiApplication>,
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

    #[must_use]
    pub fn weather_controller(&self) -> Option<&weather_controller::ffi::WeatherController> {
        self.weather_controller.as_ref()
    }

    #[must_use]
    pub fn radar_controller(&self) -> Option<&radar_controller::ffi::RadarController> {
        self.radar_controller.as_ref()
    }

    #[must_use]
    pub fn theme_controller(&self) -> Option<&theme_controller::ffi::ThemeController> {
        self.theme_controller.as_ref()
    }

    pub fn process_events(&self) {
        if let Some(application) = self.application.as_ref() {
            application.process_events();
        }
    }

    pub fn refresh_weather(&mut self) {
        if let Some(controller) = self.weather_controller.as_mut() {
            controller.refresh();
        }
    }

    pub fn refresh_radar(&mut self) {
        if let Some(controller) = self.radar_controller.as_mut() {
            controller.refresh();
        }
    }

    pub fn tick_radar(&mut self) {
        if let Some(controller) = self.radar_controller.as_mut() {
            controller.advance_frame();
        }
    }

    pub fn seek_radar(&mut self, index: i32) {
        if let Some(controller) = self.radar_controller.as_mut() {
            controller.seek(index);
        }
    }

    pub fn poll_theme(&mut self) {
        if let Some(controller) = self.theme_controller.as_mut() {
            controller.poll();
        }
    }

    pub fn exec(mut self) -> i32 {
        self.application.as_mut().map_or(1, QGuiApplication::exec)
    }
}

#[derive(Debug)]
pub enum BootstrapError {
    ApplicationCreation,
    EngineCreation,
    RuntimeCreation(std::io::Error),
    ControllerCreation,
    RootWindowCreation,
}

impl Display for BootstrapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApplicationCreation => formatter.write_str("failed to create QGuiApplication"),
            Self::EngineCreation => formatter.write_str("failed to create QQmlApplicationEngine"),
            Self::RuntimeCreation(error) => write!(formatter, "failed to create runtime: {error}"),
            Self::ControllerCreation => {
                formatter.write_str("failed to create application controller")
            }
            Self::RootWindowCreation => formatter.write_str("failed to create root QML window"),
        }
    }
}

impl Error for BootstrapError {}
