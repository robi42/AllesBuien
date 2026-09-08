//! Weather bridge: the conditions properties and the forecast list model.
//!
//! One refresh fills two shapes at once — scalar properties the conditions
//! panel binds directly, and the `QAbstractListModel` rows the forecast view
//! iterates — because they come from a single feed response and must never
//! disagree about which observation they are showing.
//!
//! `#[auto_cxx_name]` on the `extern "RustQt"` block is load-bearing: without
//! it, each multi-word `#[qproperty]` is exported under its `snake_case` Rust
//! name and the camelCase binding in QML silently resolves to `undefined`.

use std::{pin::Pin, sync::Arc};

use allesbuien_core::{
    error_chain,
    location::LocationConfig,
    location_notice, status,
    weather::{WeatherData, WeatherError},
};
use cxx_qt::{CxxQtType, Threading};

use crate::{
    conditions_view::ConditionsView,
    icon_cache,
    weather_state::{WeatherControllerConfig, WeatherControllerRust},
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
        #[qproperty(QString, location_name)]
        #[qproperty(QString, location_source)]
        #[qproperty(QString, station_name)]
        #[qproperty(QString, temperature)]
        #[qproperty(QString, feels_like)]
        #[qproperty(QString, wind)]
        #[qproperty(QString, humidity)]
        #[qproperty(QString, condition)]
        #[qproperty(QString, condition_icon_url)]
        #[qproperty(QString, observed_label)]
        #[qproperty(QString, rain_summary)]
        #[qproperty(bool, rain_wet)]
        #[qproperty(QList_f64, rain_levels)]
        #[qproperty(QStringList, rain_axis_labels)]
        #[qproperty(f64, rain_cursor_fraction)]
        #[qproperty(bool, rain_cursor_visible)]
        #[qproperty(QString, status)]
        #[qproperty(i32, refresh_interval_ms)]
        #[qproperty(i32, location_poll_interval_ms)]
        type WeatherController = super::WeatherControllerRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut WeatherController>);
        #[qinvokable]
        fn poll_location(self: Pin<&mut WeatherController>);
        #[qinvokable]
        fn track_radar_frame(self: Pin<&mut WeatherController>, label: &QString);
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
        #[must_use]
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

pub(crate) fn create(config: WeatherControllerConfig) -> cxx::UniquePtr<ffi::WeatherController> {
    let mut controller = ffi::new_weather_controller();
    if let Some(mut pinned) = controller.as_mut() {
        let refresh_ms = i32::try_from(config.intervals.refresh.as_millis()).unwrap_or(i32::MAX);
        let poll_ms = i32::try_from(config.intervals.location_poll.as_millis()).unwrap_or(i32::MAX);
        let location = config.location.resolve();
        let mut rust = pinned.as_mut().rust_mut();
        rust.feed = Some(config.feed);
        rust.icons = config.icons;
        rust.location_settings = Some(config.location);
        rust.notifications = Some(config.notifications);
        rust.runtime = Some(config.runtime);
        pinned.as_mut().adopt_location(&location);
        pinned.as_mut().set_refresh_interval_ms(refresh_ms);
        pinned.as_mut().set_location_poll_interval_ms(poll_ms);
        pinned.refresh();
    }
    controller
}

impl ffi::WeatherController {
    /// Re-asks the desktop where the viewer is, and re-fetches when it moved.
    ///
    /// Polled rather than watched for the same reason the theme is: the file is
    /// removed and rewritten rather than edited in place, so an inode-bound
    /// watcher stops hearing about it after the first `--clear`.
    pub fn poll_location(mut self: Pin<&mut Self>) {
        let Some(settings) = self.location_settings.as_ref() else {
            return;
        };
        let resolved = settings.resolve();
        if self.location.as_ref() == Some(&resolved) {
            return;
        }
        self.as_mut().adopt_location(&resolved);
        self.refresh();
    }

    fn adopt_location(mut self: Pin<&mut Self>, location: &LocationConfig) {
        self.as_mut()
            .set_location_name(location.display_label().into());
        self.as_mut()
            .set_location_source(location_notice::provenance(location.source).into());
        self.as_mut().rust_mut().location = Some(location.clone());
    }

    pub fn refresh(self: Pin<&mut Self>) {
        let (Some(feed), Some(location), Some(runtime), Some(notifications)) = (
            &self.feed,
            &self.location,
            &self.runtime,
            &self.notifications,
        ) else {
            return;
        };
        let (feed, icons, location, runtime, notifications) = (
            Arc::clone(feed),
            self.icons.clone(),
            location.clone(),
            runtime.clone(),
            notifications.clone(),
        );
        let qt_thread = self.qt_thread();
        drop(runtime.spawn(async move {
            let result = feed.fetch(location.latitude, location.longitude).await;
            if let Ok(data) = &result {
                // Warms the provider before the URLs reach QML, so the icons are
                // already decodable when the bindings below fire.
                icons.warm(&icon_cache::condition_codes(data)).await;
                let notified = notifications
                    .notify_rain(data.rain_soon.as_ref(), location.display_label())
                    .await;
                if let Err(error) = notified {
                    eprintln!("allesbuien: notification failed: {error}");
                }
            }
            if let Err(error) = qt_thread.queue(move |controller| controller.apply_result(result)) {
                eprintln!("allesbuien: weather update never reached the window: {error}");
            }
        }));
    }

    fn apply_result(mut self: Pin<&mut Self>, result: Result<WeatherData, WeatherError>) {
        let data = match result {
            Ok(data) => data,
            Err(error) => {
                let stale = !self.forecast.is_empty();
                eprintln!(
                    "allesbuien: weather refresh failed: {}",
                    error_chain::describe(&error)
                );
                self.set_status(status::weather_message(&error, stale).into());
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
        self.as_mut().set_feels_like((&view.feels_like).into());
        self.as_mut().set_wind((&view.wind).into());
        self.as_mut().set_humidity((&view.humidity).into());
        self.as_mut().set_condition((&view.condition).into());
        self.as_mut()
            .set_condition_icon_url((&view.icon_url).into());
        self.as_mut()
            .set_observed_label((&view.observed_label).into());
        self.set_status((&view.status).into());
    }
}
