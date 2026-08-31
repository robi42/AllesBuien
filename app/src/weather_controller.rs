use std::{pin::Pin, sync::Arc, time::Duration};

use allesbuien_core::{
    condition::describe,
    config::LocationConfig,
    rain::RainPoint,
    weather::{FeedApi, ForecastDay, WeatherData, WeatherError},
};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QStringList, QVariant,
};
use tokio::runtime::Handle;

use crate::{
    condition_icon, conditions_view::ConditionsView, rain_graph::RainGraph,
    services::NotificationService,
};

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
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;
    }

    #[qenum(WeatherController)]
    enum ForecastRole {
        Day,
        Weekday,
        MinTemperature,
        MaxTemperature,
        RainChance,
        Condition,
        IconUrl,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QAbstractListModel]
        #[qproperty(QString, station_name)]
        #[qproperty(QString, temperature)]
        #[qproperty(QString, wind)]
        #[qproperty(QString, condition)]
        #[qproperty(QString, condition_icon_url)]
        #[qproperty(QString, observed_label)]
        #[qproperty(QString, rain_summary)]
        #[qproperty(bool, rain_wet)]
        #[qproperty(QList_f64, rain_levels)]
        #[qproperty(QStringList, rain_axis_labels)]
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
        fn index(
            self: &WeatherController,
            row: i32,
            column: i32,
            parent: &QModelIndex,
        ) -> QModelIndex;
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
    condition_icon_url: QString,
    observed_label: QString,
    rain_summary: QString,
    rain_wet: bool,
    rain_levels: QList<f64>,
    rain_axis_labels: QStringList,
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
            condition_icon_url: QString::default(),
            observed_label: QString::default(),
            rain_summary: "Rain forecast unavailable".into(),
            rain_wet: false,
            rain_levels: QList::<f64>::default(),
            rain_axis_labels: QStringList::default(),
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
        let data = match result {
            Ok(data) => data,
            Err(error) => {
                self.set_status(format!("Weather unavailable: {error}").into());
                return;
            }
        };
        self.as_mut().replace_forecast(data.forecast);
        self.as_mut().apply_rain(&data.rain_line, &data.rain_series);
        self.apply_conditions(&ConditionsView::from_current(data.current.as_ref()));
    }

    fn apply_conditions(mut self: Pin<&mut Self>, view: &ConditionsView) {
        self.as_mut().set_station_name((&view.station_name).into());
        self.as_mut().set_temperature((&view.temperature).into());
        self.as_mut().set_wind((&view.wind).into());
        self.as_mut().set_condition((&view.condition).into());
        self.as_mut()
            .set_condition_icon_url((&view.icon_url).into());
        self.as_mut()
            .set_observed_label((&view.observed_label).into());
        self.set_status((&view.status).into());
    }

    fn apply_rain(mut self: Pin<&mut Self>, line: &str, points: &[RainPoint]) {
        let graph = RainGraph::build(points, line);
        self.as_mut().set_rain_summary((&graph.summary).into());
        self.as_mut().set_rain_wet(graph.wet);
        self.as_mut()
            .set_rain_levels(graph.levels.iter().collect::<QList<f64>>());
        self.set_rain_axis_labels(
            graph
                .axis_labels
                .iter()
                .map(QString::from)
                .collect::<QStringList>(),
        );
    }

    fn replace_forecast(mut self: Pin<&mut Self>, forecast: Vec<ForecastDay>) {
        // SAFETY: Category 13 — no operation between this paired reset can fail or panic,
        // and CxxQtThread guarantees execution on the model's owning Qt thread.
        unsafe { self.as_mut().begin_reset_model() };
        self.as_mut().rust_mut().forecast = forecast;
        // SAFETY: Category 13 — this closes the immediately preceding model reset.
        unsafe { self.end_reset_model() };
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
            ffi::ForecastRole::Weekday => QVariant::from(&QString::from(&day.day_at.weekday_label)),
            ffi::ForecastRole::MinTemperature => QVariant::from(&day.min_temperature_min),
            ffi::ForecastRole::MaxTemperature => QVariant::from(&day.max_temperature_max),
            ffi::ForecastRole::RainChance => QVariant::from(&i32::from(day.rain_chance)),
            ffi::ForecastRole::Condition => QVariant::from(&QString::from(&describe(
                &day.condition_code,
                &day.condition,
            ))),
            ffi::ForecastRole::IconUrl => QVariant::from(&QString::from(
                &condition_icon::icon_url(&day.condition_code, &day.icon_url),
            )),
            _ => QVariant::default(),
        }
    }

    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in [
            (ffi::ForecastRole::Day.repr, "day"),
            (ffi::ForecastRole::Weekday.repr, "weekday"),
            (ffi::ForecastRole::MinTemperature.repr, "minTemperature"),
            (ffi::ForecastRole::MaxTemperature.repr, "maxTemperature"),
            (ffi::ForecastRole::RainChance.repr, "rainChance"),
            (ffi::ForecastRole::Condition.repr, "condition"),
            (ffi::ForecastRole::IconUrl.repr, "iconUrl"),
        ] {
            roles.insert(role, QByteArray::from(name));
        }
        roles
    }
}
