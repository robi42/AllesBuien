use std::sync::Arc;

use allesbuien_core::weather::{FeedApi, FeedFuture};
use allesbuien_harness::{
    HarnessBuilder,
    probe::{child, focused_window, grab_window, property, text_of},
};
use allesbuien_testkit::{SETTLE_BUDGET, fixtures::five_day_weather, pump_until};
use cxx_qt_lib::QString;

/// A feed that always answers with the shared five-day fixture.
struct FiveDayFeedApi;

impl FeedApi for FiveDayFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async { Ok(five_day_weather()) })
    }
}

#[test]
fn shows_five_forecast_rows_and_visible_attribution_when_feed_is_available() {
    // Given: the real QML window backed by a five-day fake feed.
    // When: boot and Qt event processing populate and render the forecast view.
    let booted = HarnessBuilder::with_feed(Arc::new(FiveDayFeedApi)).boot();
    pump_until(
        "the model carries five forecast rows and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let model_is_ready = booted
                .weather_controller()
                .expect("controller should exist")
                .row_count(&cxx_qt_lib::QModelIndex::default())
                == 5;
            // SAFETY: on the GUI thread of the live booted app.
            model_is_ready && unsafe { focused_window() }.is_some()
        },
    );

    // SAFETY: the window and every item come from the probes on `booted`, which
    // outlives this block, and every call stays on the GUI thread.
    unsafe {
        let window = focused_window().expect("root QQuickWindow should be focused");
        let forecast = child(window, "forecastView").expect("forecastView should exist");
        let attribution = child(window, "attribution").expect("attribution should exist");

        // Then: the rendered object tree exposes five rows, and the attribution
        // carries what Buienradar's free-data terms actually require — the
        // visible domain, hyperlinked to the root domain. Reading both off the
        // rendered Label is what makes this a licence check rather than a pin
        // on whatever sentence the UI happens to word it with today.
        assert_eq!(property(forecast, "count").value::<i32>(), Some(5));
        assert_eq!(property(attribution, "visible").value::<bool>(), Some(true));
        assert_eq!(text_of(attribution), "Buienradar.nl");
        assert_eq!(
            property(attribution, "link")
                .value::<QString>()
                .map(|link| link.to_string())
                .as_deref(),
            Some("https://www.buienradar.nl")
        );

        let image = grab_window(window);
        assert!(
            !image.is_null(),
            "offscreen window grab should contain pixels"
        );
    }
}
