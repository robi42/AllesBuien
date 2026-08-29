use std::{path::PathBuf, sync::Arc};

use allesbuien::{AppBuilder, CoreServices, TimerIntervals};
use allesbuien_core::{
    notify::{Notifier, NotifyError},
    radar::{RadarApi, RadarFuture},
    weather::{FeedApi, FeedFuture},
};

struct StubFeedApi;

impl FeedApi for StubFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async { unreachable!("bootstrap must not fetch weather") })
    }
}

struct StubRadarApi;

impl RadarApi for StubRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async { unreachable!("bootstrap must not fetch radar metadata") })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async { unreachable!("bootstrap must not fetch radar frames") })
    }
}

struct StubNotifier;

impl Notifier for StubNotifier {
    fn send(&self, _title: &str, _body: &str) -> Result<(), NotifyError> {
        Ok(())
    }
}

#[test]
fn creates_engine_and_window_when_booting_with_stub_services() {
    // Given: fake core boundaries and inert timer configuration.
    let services = CoreServices::new(
        Arc::new(StubFeedApi),
        Arc::new(StubRadarApi),
        Arc::new(StubNotifier),
    );
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
