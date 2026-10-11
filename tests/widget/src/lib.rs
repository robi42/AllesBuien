//! The bar widget's shell proof, run by `cargo test`.
//!
//! The Omarchy bar widget keeps every decision and install step in
//! `plugin/widget.sh`, and `tests/widget/run.sh` proves that script against
//! the fakes in `tests/widget/fakes`. This crate holds no code. It exists so
//! that `cargo test --workspace`, which `bin/test` and CI already run, also
//! runs that proof and shellchecks the scripts behind it, and a red widget
//! suite fails the workspace tests like any other test. Its two tests live in
//! `tests/scripts.rs`.
