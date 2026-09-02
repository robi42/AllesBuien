//! The test doubles and waiting helper the GUI harnesses under `app/tests/`
//! share.
//!
//! Every file under `app/tests/` is pinned by `AGENTS.md` to exactly one GUI
//! test and one `QGuiApplication`, so seven harnesses each carried their own
//! copy of the same stub feed, stub radar, stub notifier, and fixture location.
//! Two of them outgrew the module-size budget on the strength of that
//! duplication alone.
//!
//! This is a library crate rather than an `app/tests/common/mod.rs` for two
//! reasons. A `mod common` is private to each test binary, so any double a
//! given harness does not use is dead code there and the only way to compile
//! seven harnesses against one module is `#[allow(dead_code)]` — an attribute
//! this repository has none of. And a helper only earns trust with tests of its
//! own: `#[test]` functions inside `app/tests/common/` would compile into every
//! harness binary and break the one-GUI-test-per-executable rule, whereas this
//! crate's tests run in a process that never constructs a `QGuiApplication`.
//!
//! Only doubles that more than one harness needs live here. A scenario's own
//! fake — the generation-scripted radar in `radar_controller`, the two-frame
//! animation in `radar_view_grab` — stays beside the single test that reads it,
//! where its expectations are visible next to the assertions they serve.

pub mod doubles;
pub mod pump;

pub use doubles::{StubFeedApi, StubNotifier, StubRadarApi, amsterdam_west};
pub use pump::{SETTLE_BUDGET, pump_until};
