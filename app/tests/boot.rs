use std::sync::Arc;

use allesbuien_harness::HarnessBuilder;
use allesbuien_testkit::StubFeedApi;

#[test]
fn creates_engine_and_window_when_booting_with_stub_services() {
    // Given: fake core boundaries and inert timer configuration.
    let builder = HarnessBuilder::with_feed(Arc::new(StubFeedApi));

    // When: the application is booted once in this integration-test process.
    let booted = builder.boot();

    // Then: both the QML engine and its root window were created.
    assert!(booted.engine_created() && booted.window_created());
}
