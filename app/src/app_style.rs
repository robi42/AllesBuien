//! Picks the Qt Quick Controls style for the process.
//!
//! `QQuickStyle::setStyle` is process-global and only takes effect before the
//! engine instantiates its first control, which is why this is the first thing
//! `boot` does rather than something QML configures.

use cxx_qt_lib::QString;

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!(<QtQuickControls2/QQuickStyle>);
        type QQuickStyle;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        #[Self = "QQuickStyle"]
        #[rust_name = "set_style"]
        fn setStyle(style: &QString);
    }
}

pub fn set_material_style() {
    ffi::QQuickStyle::set_style(&QString::from("Material"));
}
