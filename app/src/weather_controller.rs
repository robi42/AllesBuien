use std::{pin::Pin, sync::Arc, time::Duration};

use allesbuien_core::{
    config::LocationConfig,
    weather::{FeedApi, ForecastDay, WeatherData, WeatherError},
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use tokio::runtime::Handle;

use crate::services::NotificationService;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++Qt" {
        include!(<QtCore/QAbstractListModel>);
        #[qobject]
        type QAbstractListModel;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;
    }

    #[qenum(WeatherController)]
    enum ForecastRole {
        Day,
        MinTemperature,
        MaxTemperature,
        RainChance,
        Condition,
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QAbstractListModel]
        #[qproperty(QString, station_name)]
        #[qproperty(QString, temperature)]
        #[qproperty(QString, wind)]
        #[qproperty(QString, condition)]
        #[qproperty(QString, rain_summary)]
        #[qproperty(QString, status)]
        #[qproperty(i32, refresh_interval_ms)]
        type WeatherController = super::WeatherControllerRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut WeatherController>);
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &WeatherController, parent: &QModelIndex) -> i32;
        #[cxx_override]
        fn data(self: &WeatherController, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &WeatherController) -> QHash_i32_QByteArray;
        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut WeatherController>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut WeatherController>);
    }

    impl cxx_qt::Threading for WeatherController {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_weather_controller"]
        fn make_unique() -> UniquePtr<WeatherController>;
    }

    unsafe extern "C++" {
        include!("weather_controller.h");
        #[rust_name = "set_weather_controller"]
        fn setWeatherController(
            engine: Pin<&mut QQmlApplicationEngine>,
            controller: Pin<&mut WeatherController>,
        );
    }
}

pub(crate) struct WeatherControllerConfig {
    pub(crate) feed: Arc<dyn FeedApi>,
    pub(crate) location: LocationConfig,
    pub(crate) notifications: NotificationService,
    pub(crate) runtime: Handle,
    pub(crate) refresh_interval: Duration,
}

pub struct WeatherControllerRust {
    station_name: QString,
    temperature: QString,
    wind: QString,
    condition: QString,
    rain_summary: QString,
    status: QString,
    refresh_interval_ms: i32,
    forecast: Vec<ForecastDay>,
    feed: Option<Arc<dyn FeedApi>>,
    location: Option<LocationConfig>,
    runtime: Option<Handle>,
    notifications: Option<NotificationService>,
}

impl Default for WeatherControllerRust {
    fn default() -> Self {
        Self {
            station_name: "Unavailable".into(),
            temperature: "Unavailable".into(),
            wind: "Unavailable".into(),
            condition: "Unavailable".into(),
            rain_summary: "Rain forecast unavailable".into(),
            status: "Weather unavailable".into(),
            refresh_interval_ms: 600_000,
            forecast: Vec::new(),
            feed: None,
            location: None,
            runtime: None,
            notifications: None,
        }
    }
}

pub(crate) fn create(config: WeatherControllerConfig) -> cxx::UniquePtr<ffi::WeatherController> {
    let mut controller = ffi::new_weather_controller();
    if let Some(mut pinned) = controller.as_mut() {
        let interval = i32::try_from(config.refresh_interval.as_millis()).unwrap_or(i32::MAX);
        let mut rust = pinned.as_mut().rust_mut();
        rust.feed = Some(config.feed);
        rust.location = Some(config.location);
        rust.notifications = Some(config.notifications);
        rust.runtime = Some(config.runtime);
        pinned.as_mut().set_refresh_interval_ms(interval);
        pinned.refresh();
    }
    controller
}

impl ffi::WeatherController {
    pub fn refresh(self: Pin<&mut Self>) {
        let (Some(feed), Some(location), Some(runtime), Some(notifications)) = (
            &self.feed,
            &self.location,
            &self.runtime,
            &self.notifications,
        ) else {
            return;
        };
        let (feed, location, runtime, notifications) = (
            Arc::clone(feed),
            location.clone(),
            runtime.clone(),
            notifications.clone(),
        );
        let qt_thread = self.qt_thread();
        drop(runtime.spawn(async move {
            let result = feed.fetch(location.latitude, location.longitude).await;
            if let Ok(data) = &result
                && let Err(error) = notifications.notify_rain(data.rain_soon.as_ref()).await
            {
                eprintln!("allesbuien: notification failed: {error}");
            }
            let _queue_result = qt_thread.queue(move |controller| controller.apply_result(result));
        }));
    }

    fn apply_result(mut self: Pin<&mut Self>, result: Result<WeatherData, WeatherError>) {
        match result {
            Ok(data) => {
                self.as_mut().replace_forecast(data.forecast);
                self.as_mut()
                    .set_rain_summary(QString::from(if data.rain_line.is_empty() {
                        "Rain forecast unavailable"
                    } else {
                        &data.rain_line
                    }));
                match data.current {
                    Some(current) => {
                        self.as_mut().set_station_name(current.station_name.into());
                        self.as_mut().set_temperature(
                            current.temperature.map_or_else(
                                || "Unavailable".into(),
                                |v| format!("{v:.1} °C").into(),
                            ),
                        );
                        self.as_mut().set_wind(current.wind_speed.map_or_else(
                            || "Unavailable".into(),
                            |v| format!("{v:.1} m/s").into(),
                        ));
                        self.as_mut().set_condition(
                            current
                                .condition
                                .unwrap_or_else(|| "Unavailable".into())
                                .into(),
                        );
                        self.set_status(QString::default());
                    }
                    None => self.as_mut().set_unavailable(),
                }
            }
            Err(error) => self.set_status(format!("Weather unavailable: {error}").into()),
        }
    }

    fn replace_forecast(mut self: Pin<&mut Self>, forecast: Vec<ForecastDay>) {
        // SAFETY: Category 13 — no operation between this paired reset can fail or panic,
        // and CxxQtThread guarantees execution on the model's owning Qt thread.
        unsafe { self.as_mut().begin_reset_model() };
        self.as_mut().rust_mut().forecast = forecast;
        // SAFETY: Category 13 — this closes the immediately preceding model reset.
        unsafe { self.end_reset_model() };
    }

    fn set_unavailable(mut self: Pin<&mut Self>) {
        self.as_mut().set_station_name("Unavailable".into());
        self.as_mut().set_temperature("Unavailable".into());
        self.as_mut().set_wind("Unavailable".into());
        self.as_mut().set_condition("Unavailable".into());
        self.set_status("Current conditions unavailable".into());
    }

    pub fn row_count(&self, _parent: &QModelIndex) -> i32 {
        i32::try_from(self.forecast.len()).unwrap_or(i32::MAX)
    }

    pub fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(day) = usize::try_from(index.row())
            .ok()
            .and_then(|row| self.forecast.get(row))
        else {
            return QVariant::default();
        };
        match (ffi::ForecastRole { repr: role }) {
            ffi::ForecastRole::Day => QVariant::from(&QString::from(&day.day)),
            ffi::ForecastRole::MinTemperature => QVariant::from(&day.min_temperature_min),
            ffi::ForecastRole::MaxTemperature => QVariant::from(&day.max_temperature_max),
            ffi::ForecastRole::RainChance => QVariant::from(&i32::from(day.rain_chance)),
            ffi::ForecastRole::Condition => QVariant::from(&QString::from(&day.condition)),
            _ => QVariant::default(),
        }
    }

    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in [
            (ffi::ForecastRole::Day.repr, "day"),
            (ffi::ForecastRole::MinTemperature.repr, "minTemperature"),
            (ffi::ForecastRole::MaxTemperature.repr, "maxTemperature"),
            (ffi::ForecastRole::RainChance.repr, "rainChance"),
            (ffi::ForecastRole::Condition.repr, "condition"),
        ] {
            roles.insert(role, QByteArray::from(name));
        }
        roles
    }
}
