//! Omarchy theme properties for QML.
//!
//! `themeAvailable` is the fallback boundary: QML must use `SystemPalette` while it is false.
//! Rust intentionally retains no invented fallback palette.

use std::{path::PathBuf, pin::Pin, time::SystemTime};

use allesbuien_core::theme::{Theme, colors_mtime, read_theme};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::{
    qt_values::{interval_ms, or_empty},
    services::TimerIntervals,
};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, theme_background)]
        #[qproperty(QString, theme_foreground)]
        #[qproperty(QString, theme_accent)]
        #[qproperty(QString, theme_selection)]
        #[qproperty(QString, theme_lighter_background)]
        #[qproperty(QString, theme_muted)]
        #[qproperty(bool, theme_available)]
        #[qproperty(i32, poll_interval_ms)]
        type ThemeController = super::ThemeControllerRust;

        #[qinvokable]
        fn poll(self: Pin<&mut ThemeController>);
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_theme_controller"]
        #[must_use]
        fn make_unique() -> UniquePtr<ThemeController>;
    }

    unsafe extern "C++" {
        include!("theme_controller.h");
        #[rust_name = "set_theme_controller"]
        fn setThemeController(
            engine: Pin<&mut QQmlApplicationEngine>,
            controller: Pin<&mut ThemeController>,
        );
    }
}

pub(crate) struct ThemeControllerConfig {
    pub(crate) theme_base_path: PathBuf,
    pub(crate) poll_interval_ms: i32,
}

pub struct ThemeControllerRust {
    theme_background: QString,
    theme_foreground: QString,
    theme_accent: QString,
    theme_selection: QString,
    theme_lighter_background: QString,
    theme_muted: QString,
    theme_available: bool,
    poll_interval_ms: i32,
    theme_base_path: PathBuf,
    last_mtime: Option<SystemTime>,
    initialized: bool,
}

impl Default for ThemeControllerRust {
    fn default() -> Self {
        Self {
            theme_background: QString::default(),
            theme_foreground: QString::default(),
            theme_accent: QString::default(),
            theme_selection: QString::default(),
            theme_lighter_background: QString::default(),
            theme_muted: QString::default(),
            theme_available: false,
            poll_interval_ms: interval_ms(TimerIntervals::default().theme_poll),
            theme_base_path: PathBuf::new(),
            last_mtime: None,
            initialized: false,
        }
    }
}

pub(crate) fn create(config: ThemeControllerConfig) -> cxx::UniquePtr<ffi::ThemeController> {
    let mut controller = ffi::new_theme_controller();
    if let Some(mut pinned) = controller.as_mut() {
        let mut rust = pinned.as_mut().rust_mut();
        rust.theme_base_path = config.theme_base_path;
        pinned
            .as_mut()
            .set_poll_interval_ms(config.poll_interval_ms);
        pinned.poll();
    }
    controller
}

impl ffi::ThemeController {
    /// Re-reads the theme when its colours changed on disk since the last read.
    pub fn poll(mut self: Pin<&mut Self>) {
        let mtime = colors_mtime(&self.theme_base_path);
        if self.initialized && self.last_mtime == mtime {
            return;
        }
        let theme = read_theme(&self.theme_base_path);
        let mut rust = self.as_mut().rust_mut();
        rust.last_mtime = mtime;
        rust.initialized = true;
        self.apply_theme(theme);
    }

    /// Publishes a freshly read theme, or flips QML onto its `SystemPalette` fallback.
    fn apply_theme(mut self: Pin<&mut Self>, theme: Theme) {
        match theme {
            Theme::Available {
                background,
                foreground,
                accent,
                selection,
                lighter_background,
                muted,
            } => {
                self.as_mut().set_theme_background(or_empty(background));
                self.as_mut().set_theme_foreground(or_empty(foreground));
                self.as_mut().set_theme_accent(or_empty(accent));
                self.as_mut().set_theme_selection(or_empty(selection));
                self.as_mut()
                    .set_theme_lighter_background(or_empty(lighter_background));
                self.as_mut().set_theme_muted(or_empty(muted));
                self.set_theme_available(true);
            }
            Theme::Unavailable => self.set_theme_available(false),
        }
    }
}
