//! The bar widget's proofs, and the app window's one QML proof that needs a
//! process of its own, run by `cargo test`.
//!
//! The Omarchy bar widget keeps every decision and install step in
//! `plugin/widget.sh`, and `tests/widget/run.sh` proves that script against
//! the fakes in `tests/widget/fakes`. `plugin/BarWidget.qml` decides when to
//! probe, and `tests/widget/qml/tst_schedule.qml` proves that schedule under
//! the fake modules in `tests/widget/qml/fakes`, which stand in for the ones
//! only an Omarchy desktop has.
//!
//! The app window reports the screen it is on through
//! `app/qml/ScreenReport.qml`, and `tests/window/qml/tst_screen_report.qml`
//! moves it between two monitors. That takes a desktop of two screens, which
//! the offscreen platform gives only a process started with its configuration,
//! and this crate already starts `qmltestrunner` as a child.
//!
//! This crate holds no code. It exists so that `cargo test --workspace`, which
//! `bin/test` and CI already run, also runs these proofs and shellchecks the
//! scripts behind the first, and a red suite fails the workspace tests like
//! any other test. Its four tests live in `tests/scripts.rs`.
