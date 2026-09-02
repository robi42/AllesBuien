use std::{path::PathBuf, sync::Arc};

use allesbuien::{AppBuilder, CoreServiceConfig, CoreServices, TimerIntervals};
use allesbuien_testkit::{StubFeedApi, StubNotifier, StubRadarApi, amsterdam_west};

#[test]
fn creates_engine_and_window_when_booting_with_stub_services() {
    // Given: fake core boundaries and inert timer configuration.
    let services = CoreServices::new(CoreServiceConfig {
        feed: Arc::new(StubFeedApi),
        radar: Arc::new(StubRadarApi),
        notifier: Arc::new(StubNotifier),
        location: amsterdam_west(),
        radar_state_dir: PathBuf::from("/nonexistent/test-radar"),
    });
    let builder = AppBuilder::new(
        services,
        PathBuf::from("/nonexistent/test-theme"),
        TimerIntervals::default(),
    );

    // When: the application is booted once in this integration-test process.
    let booted = builder.boot().expect("stub-wired GUI should boot");

    // Then: both the QML engine and its root window were created.
    assert!(booted.engine_created() && booted.window_created());
}
