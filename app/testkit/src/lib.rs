//! The Qt-free test doubles, fixtures, and waiting helpers the GUI harnesses
//! under `app/tests/` share.
//!
//! Every file under `app/tests/` is pinned by `AGENTS.md` to exactly one GUI
//! test and one `QGuiApplication`, so the harnesses each carried their own
//! copy of the same stub feed, stub radar, stub notifier, and fixture location.
//! Two of them outgrew the module-size budget on the strength of that
//! duplication alone.
//!
//! This is a library crate rather than an `app/tests/common/mod.rs` for two
//! reasons. A `mod common` is private to each test binary, so any double a
//! given harness does not use is dead code there and the only way to compile
//! every harness against one module is a dead-code waiver — an attribute this
//! repository has none of. And a helper only earns trust with tests of its
//! own: `#[test]` functions inside `app/tests/common/` would compile into every
//! harness binary and break the one-GUI-test-per-executable rule, whereas this
//! crate's tests run in a process that never constructs a `QGuiApplication`.
//!
//! Only doubles and fixtures that more than one harness needs live here. A scenario's own
//! fake — the generation-scripted radar in `radar_controller`, the two-frame
//! animation in `radar_view_grab` — stays beside the single test that reads it,
//! where its expectations are visible next to the assertions they serve.
//!
//! Nothing here links Qt. The Qt-bound half — booting the window and reading
//! its rendered object tree — is `allesbuien-harness`.

pub mod doubles;
pub mod fixtures;
pub mod pump;
pub mod weather_setting;

pub use doubles::{StubFeedApi, StubNotifier, StubRadarApi, amsterdam_west};
pub use pump::{SETTLE_BUDGET, block_on, pump_until};
pub use weather_setting::{clear_weather, write_weather};
