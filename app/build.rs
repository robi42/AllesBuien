use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QResource, QResources, QmlModule};

const CLANGD_LINK_DIR: &str = "cxxqt-clangd";

/// Generated C++ header root, relative to `OUT_DIR`. Matches `cxx-qt-build`'s
/// `dir::header_root()`, which is `dir::target()/include`.
const HEADER_ROOT_IN_OUT_DIR: &str = "cxxqtbuild/include";

/// Generated C++ source root, relative to `OUT_DIR`. Matches `cxx-qt-build`'s
/// `dir::gen()`.
const GEN_ROOT_IN_OUT_DIR: &str = "cxxqtgen";

fn main() {
    println!("cargo:rerun-if-env-changed=QMAKE");
    if std::env::var_os("QMAKE").is_none() && Path::new("/usr/bin/qmake6").is_file() {
        // SAFETY: no data race can occur: this build-script main is
        // single-threaded and sets QMAKE before CxxQtBuilder can spawn any work.
        // cxx-qt-build honors QMAKE; this avoids the Qt 5 /usr/bin/qmake that
        // qt5-base installs.
        unsafe { std::env::set_var("QMAKE", "/usr/bin/qmake6") };
    }

    CxxQtBuilder::new_qml_module(QmlModule::new("nl.allesbuien").qml_files([
        "qml/Main.qml",
        "qml/Theme.qml",
        "qml/RadarPanel.qml",
        "qml/RadarTransport.qml",
        "qml/ConditionIcon.qml",
        "qml/MetricChip.qml",
        "qml/RadarLegend.qml",
        "qml/ScrimNotice.qml",
        "qml/ConditionsView.qml",
        "qml/ForecastView.qml",
        "qml/RainView.qml",
    ]))
    .qrc_resources(QResources::new().resource(QResource::new().files(["assets/basemap-nl.png"])))
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .file("src/app_style.rs")
    .file("src/image_provider.rs")
    .file("src/radar_controller.rs")
    .file("src/theme_controller.rs")
    .file("src/weather_controller.rs")
    .cpp_file("src/image_provider.cpp")
    .cpp_file("src/radar_controller.cpp")
    .cpp_file("src/theme_controller.cpp")
    .cpp_file("src/weather_controller.cpp")
    .build();

    link_generated_cpp_for_clangd();
}

/// Point `target/cxxqt-clangd/{include,gen}` at the C++ tree this build just
/// generated, so `.clangd` can reach it under a name that never changes.
///
/// `cxx-qt-build` writes that tree into the hashed `OUT_DIR` and, unlike
/// vanilla `cxx` with its `target/cxxbridge`, never mirrors it anywhere stable.
/// The hash rotates on toolchain, feature, and dependency-graph changes, and
/// `.clangd` accepts literal include paths only - it cannot glob - so an editor
/// has no way to name that directory. Relinking on every build is what keeps
/// the stable names pointing at the live tree across rotation.
///
/// This is editor tooling, not a build input: a failure warns and never breaks
/// the build.
fn link_generated_cpp_for_clangd() {
    let Some(out_dir) = std::env::var_os("OUT_DIR").map(PathBuf::from) else {
        return;
    };
    let Some(target_dir) = cargo_target_dir(&out_dir) else {
        println!("cargo:warning=clangd links skipped: no Cargo target directory above OUT_DIR");
        return;
    };

    let link_root = target_dir.join(CLANGD_LINK_DIR);
    if let Err(error) = fs::create_dir_all(&link_root) {
        println!("cargo:warning=clangd links skipped: {error}");
        return;
    }

    for (link_name, generated) in [
        ("include", HEADER_ROOT_IN_OUT_DIR),
        ("gen", GEN_ROOT_IN_OUT_DIR),
    ] {
        let generated = out_dir.join(generated);
        if !generated.is_dir() {
            println!(
                "cargo:warning=clangd link {link_name} skipped: cxx-qt-build wrote no {}",
                generated.display()
            );
            continue;
        }
        if let Err(error) = relink(&link_root.join(link_name), &generated) {
            println!("cargo:warning=clangd link {link_name} failed: {error}");
        }
    }
}

/// Walk up from `OUT_DIR` to the Cargo target directory, which is the nearest
/// ancestor Cargo tagged with `CACHEDIR.TAG`. `CARGO_TARGET_DIR` wins when set.
/// This mirrors how `cxx-qt-build` finds the same directory for `target/cxxqt`,
/// so both stable trees land side by side under any profile or target triple.
fn cargo_target_dir(out_dir: &Path) -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("CARGO_TARGET_DIR") {
        return Some(PathBuf::from(configured));
    }
    out_dir
        .ancestors()
        .find(|dir| {
            dir.join("CACHEDIR.TAG").is_file()
                || (dir.file_name() == Some(OsStr::new("target"))
                    && dir
                        .parent()
                        .is_some_and(|parent| parent.join("Cargo.toml").is_file()))
        })
        .map(Path::to_path_buf)
}

fn relink(link: &Path, generated: &Path) -> std::io::Result<()> {
    // A plain `symlink` refuses an occupied path, and the occupant is normally
    // a symlink into an OUT_DIR that has since rotated away. Clear it first,
    // tolerating a real directory left behind by an interrupted build.
    if link.is_symlink() {
        fs::remove_file(link)?;
    } else if link.exists() {
        fs::remove_dir_all(link)?;
    }
    std::os::unix::fs::symlink(generated, link)
}
