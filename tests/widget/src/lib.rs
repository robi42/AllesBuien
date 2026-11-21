//! The bar widget's proofs, run by `cargo test`.
//!
//! The Omarchy bar widget keeps every decision and install step in
//! `plugin/widget.sh`, and `tests/widget/run.sh` proves that script against
//! the fakes in `tests/widget/fakes`. `plugin/BarWidget.qml` decides when to
//! probe, and `tests/widget/qml/tst_schedule.qml` proves that schedule under
//! the fake modules in `tests/widget/qml/fakes`, which stand in for the ones
//! only an Omarchy desktop has. This crate holds no code. It exists so
//! that `cargo test --workspace`, which `bin/test` and CI already run, also
//! runs both proofs and shellchecks the scripts behind the first, and a red
//! widget suite fails the workspace tests like any other test. Its three tests
//! live in `tests/scripts.rs`.
