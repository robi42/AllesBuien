//! Omarchy's theme and text size, as properties for QML.
//!
//! `themeAvailable` is the fallback boundary: QML must use `SystemPalette` while it is false.
//! Rust intentionally retains no invented fallback palette.
//!
//! The same 2-second poll follows `omarchy-display-text-size`. Every poll
//! re-reads `[font] base-size` from `~/.config/omarchy/shell.toml`, whether or
//! not the colours changed, and the bridge publishes the window's fit from
//! [`allesbuien_core::window_fit`]: the zoom `Main.qml` scales its content by,
//! in `zoomParts` of [`ZOOM_UNIT`](allesbuien_core::window_fit::ZOOM_UNIT), the
//! window's minimum size, and the size it
//! opens at. QML hands back the two things only the window knows: the screen it
//! is on, through `trackScreen`, and its first frame, through `trackFrame`,
//! after which the second poll ends the launch and fixes the opening size.
//! `launching` tells QML when it can stop reporting frames.

use std::{
    path::{Path, PathBuf},
    pin::Pin,
    time::SystemTime,
};

use allesbuien_core::{
    text_size::{DEFAULT_BASE_SIZE, read_base_size},
    theme::{Theme, colors_mtime, read_theme},
    window_fit::{Screen, Size, WindowDesign, WindowFitter},
};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::{
    qt_values::{interval_ms, or_empty, qt_int},
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
        #[qproperty(i32, zoom_parts)]
        #[qproperty(i32, minimum_width)]
        #[qproperty(i32, minimum_height)]
        #[qproperty(i32, opening_width)]
        #[qproperty(i32, opening_height)]
        #[qproperty(bool, launching)]
        type ThemeController = super::ThemeControllerRust;

        #[qinvokable]
        fn poll(self: Pin<&mut ThemeController>);

        // One word per parameter: CXX-Qt hands each parameter's name to QML's
        // type registry as written, where bin/check-qml-contract refuses
        // snake_case. `workwidth` and `workheight` are QML's
        // `Screen.desktopAvailableWidth` and `Screen.desktopAvailableHeight`.
        #[qinvokable]
        fn track_screen(
            self: Pin<&mut ThemeController>,
            width: i32,
            height: i32,
            workwidth: i32,
            workheight: i32,
        );

        #[qinvokable]
        fn track_frame(self: Pin<&mut ThemeController>);
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

/// The window's sizes as designed, at 12 px: what the zoom multiplies.
///
/// The minimum width is the radar's own floor plus the column beside it, the
/// outer margins and the gap between them, all set in `Main.qml`:
/// 400 + 380 + 24 * 2 + 24. Below that the two panels start taking width from
/// each other, and the first thing to go is the map, which is the one element
/// there that gets worse the smaller it is.
///
/// The minimum height is measured, not derived: the forecast list clips rather
/// than squashing its rows, so the floor is whatever makes all five of them
/// fit. 640 renders four and slices the fifth, 660 still slices it, 664 lands
/// the last row's icon on the clip edge, 668 is clean. 672 adds one 4 px step as
/// the margin for a themed font with taller metrics, since the window follows
/// whichever theme Omarchy has on.
pub const WINDOW: WindowDesign = WindowDesign {
    minimum: Size {
        width: 852,
        height: 672,
    },
    opening: Size {
        width: 980,
        height: 760,
    },
};

pub(crate) struct ThemeControllerConfig {
    pub(crate) theme_base_path: PathBuf,
    /// Omarchy's shell configuration; `None` reads nothing and means 12 px.
    pub(crate) shell_config: Option<PathBuf>,
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
    zoom_parts: i32,
    minimum_width: i32,
    minimum_height: i32,
    opening_width: i32,
    opening_height: i32,
    launching: bool,
    theme_base_path: PathBuf,
    shell_config: Option<PathBuf>,
    fitter: WindowFitter,
    last_mtime: Option<SystemTime>,
    initialized: bool,
}

impl Default for ThemeControllerRust {
    /// The window as designed, launching, before any setting has been read.
    fn default() -> Self {
        let fitter = WindowFitter::new(WINDOW, DEFAULT_BASE_SIZE);
        let fit = fitter.fit();
        Self {
            theme_background: QString::default(),
            theme_foreground: QString::default(),
            theme_accent: QString::default(),
            theme_selection: QString::default(),
            theme_lighter_background: QString::default(),
            theme_muted: QString::default(),
            theme_available: false,
            poll_interval_ms: interval_ms(TimerIntervals::default().theme_poll),
            zoom_parts: qt_int(fit.zoom),
            minimum_width: qt_int(fit.minimum.width),
            minimum_height: qt_int(fit.minimum.height),
            opening_width: qt_int(fit.opening.width),
            opening_height: qt_int(fit.opening.height),
            launching: fitter.launching(),
            theme_base_path: PathBuf::new(),
            shell_config: None,
            fitter,
            last_mtime: None,
            initialized: false,
        }
    }
}

pub(crate) fn create(config: ThemeControllerConfig) -> cxx::UniquePtr<ffi::ThemeController> {
    let mut controller = ffi::new_theme_controller();
    if let Some(mut pinned) = controller.as_mut() {
        let base_size = base_size_at(config.shell_config.as_deref());
        let mut rust = pinned.as_mut().rust_mut();
        rust.theme_base_path = config.theme_base_path;
        rust.shell_config = config.shell_config;
        rust.fitter = WindowFitter::new(WINDOW, base_size);
        pinned
            .as_mut()
            .set_poll_interval_ms(config.poll_interval_ms);
        pinned.as_mut().publish_fit();
        pinned.check_colours();
    }
    controller
}

/// The text size `shell_config` sets; 12 px when there is none to read.
fn base_size_at(shell_config: Option<&Path>) -> u32 {
    shell_config.map_or(DEFAULT_BASE_SIZE, read_base_size)
}

impl ffi::ThemeController {
    /// Re-reads the theme when its colours changed on disk since the last read,
    /// counts the poll towards the end of the launch, and re-reads the text size,
    /// every time.
    pub fn poll(mut self: Pin<&mut Self>) {
        self.as_mut().check_colours();
        let base_size = base_size_at(self.shell_config.as_deref());
        let mut rust = self.as_mut().rust_mut();
        let launch_ended = rust.fitter.poll();
        let resized = rust.fitter.set_base_size(base_size);
        if launch_ended || resized {
            self.publish_fit();
        }
    }

    /// Takes the screen the window is on, as QML reports it. A report with any
    /// number that is not positive is no screen and changes nothing.
    pub fn track_screen(
        mut self: Pin<&mut Self>,
        width: i32,
        height: i32,
        workwidth: i32,
        workheight: i32,
    ) {
        let Some(screen) = Screen::from_report(width, height, workwidth, workheight) else {
            return;
        };
        if self.as_mut().rust_mut().fitter.track_screen(screen) {
            self.publish_fit();
        }
    }

    /// Notes that the window has shown a frame; the second poll after the
    /// first one ends the launch.
    pub fn track_frame(self: Pin<&mut Self>) {
        self.rust_mut().fitter.track_frame();
    }

    /// Re-reads the theme when its colours changed on disk since the last read.
    fn check_colours(mut self: Pin<&mut Self>) {
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

    /// Publishes the window's fit. The minimum goes first, so the layout is
    /// never handed less than its floor, and a smaller opening size can then
    /// shrink the window; the zoom goes after both.
    fn publish_fit(mut self: Pin<&mut Self>) {
        let fit = self.fitter.fit();
        let launching = self.fitter.launching();
        self.as_mut().set_minimum_width(qt_int(fit.minimum.width));
        self.as_mut().set_minimum_height(qt_int(fit.minimum.height));
        self.as_mut().set_opening_width(qt_int(fit.opening.width));
        self.as_mut().set_opening_height(qt_int(fit.opening.height));
        self.as_mut().set_zoom_parts(qt_int(fit.zoom));
        self.set_launching(launching);
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

#[cfg(test)]
mod tests {
    use allesbuien_core::window_fit::ZOOM_UNIT;

    use super::ThemeControllerRust;

    #[test]
    fn the_default_state_is_the_window_as_designed_while_launching() {
        // Given / When
        let state = ThemeControllerRust::default();

        // Then: never 0, which would collapse the window before the first read.
        assert_eq!(state.zoom_parts, 12_000);
        assert_eq!(u32::try_from(state.zoom_parts), Ok(ZOOM_UNIT));
        assert_eq!((state.minimum_width, state.minimum_height), (852, 672));
        assert_eq!((state.opening_width, state.opening_height), (980, 760));
        assert!(state.launching);
    }
}
