//! The Qt-bound scaffolding the GUI harnesses under `app/tests/` share: booting
//! the real window over injected ports, reading its rendered object tree, and
//! typing into it.
//!
//! [`allesbuien_testkit`] holds everything that does not need Qt. What is left
//! here was copied across the harnesses verbatim — five copies of one block of
//! mangled Qt symbols, twelve copies of the same boot preamble — and it cannot
//! join the testkit without that crate linking Qt.
//!
//! It is a crate for the same reasons the testkit is. An `app/tests/common`
//! module compiles into each harness binary privately, so every probe a harness
//! does not call is dead code under clippy's `-D warnings`, and this repository
//! carries no dead-code waivers. And it is not a module of the app library,
//! because test probes must not ship in the production binary.

pub mod boot;
pub mod keys;
pub mod probe;

pub use boot::{Harness, HarnessBuilder};
