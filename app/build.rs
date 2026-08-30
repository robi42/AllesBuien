use std::path::Path;

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    println!("cargo:rerun-if-env-changed=QMAKE");
    if std::env::var_os("QMAKE").is_none() && Path::new("/usr/bin/qmake6").is_file() {
        // SAFETY: Category 2 (data races) cannot occur: this build-script main is
        // single-threaded and sets QMAKE before CxxQtBuilder can spawn any work.
        // cxx-qt-build honors QMAKE; this avoids this host's Qt 5 /usr/bin/qmake.
        unsafe { std::env::set_var("QMAKE", "/usr/bin/qmake6") };
    }

    CxxQtBuilder::new_qml_module(QmlModule::new("nl.allesbuien").qml_file("qml/Main.qml"))
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .file("src/app_style.rs")
        .file("src/image_provider.rs")
        .file("src/radar_controller.rs")
        .file("src/weather_controller.rs")
        .cpp_file("src/image_provider.cpp")
        .cpp_file("src/radar_controller.cpp")
        .cpp_file("src/weather_controller.cpp")
        .build();
}
