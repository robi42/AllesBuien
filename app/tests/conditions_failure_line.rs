//! One failed fetch says so once, and the reading it could not fill does not
//! pretend to be a number that was cut short.
//!
//! Neither defect was reachable from the controller. The duplicate status was
//! two labels binding one `status` property, so every property-level assertion
//! passed while the window printed the sentence twice about 190 px apart; the
//! truncation was `Text.ElideRight` deciding at paint time, which no binding
//! reports. Both are outcomes of the rendered tree, so this reads them there:
//! the forecast list must contain no second copy of the sentence, and the hero
//! must ask for no more width than it was given.

use std::sync::Arc;

use allesbuien_core::weather::{FeedApi, FeedFuture, WeatherError};
use allesbuien_harness::{
    HarnessBuilder,
    probe::{child, focused_window, number_of, property, text_of},
};
use allesbuien_testkit::{SETTLE_BUDGET, pump_until};

/// A feed whose every request fails at once, as it does when the window opens
/// offline.
struct UnreachableFeedApi;

impl FeedApi for UnreachableFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            let error = reqwest::Client::new()
                .get("not a URL")
                .send()
                .await
                .expect_err("the fake request URL must be invalid");
            Err(WeatherError::FeedRequest(error))
        })
    }
}

#[test]
fn a_failed_fetch_states_itself_once_and_leaves_the_reading_whole() {
    // Given: a booted window whose only feed request fails.
    let booted = HarnessBuilder::with_feed(Arc::new(UnreachableFeedApi)).boot();

    // When: that failure has reached the controller and the layout has settled.
    pump_until(
        "the cold failure reaches the panel and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let failure_is_ready = booted
                .weather_controller()
                .is_some_and(|controller| controller.status().to_string().contains(" — "));
            // SAFETY: on the GUI thread of the live booted app.
            failure_is_ready && unsafe { focused_window() }.is_some()
        },
    );
    let status = booted
        .weather_controller()
        .expect("controller should exist")
        .status()
        .to_string();
    // SAFETY: the window and every item come from the probes on `booted`, which
    // outlives this block, and every call stays on the GUI thread.
    unsafe {
        let window = focused_window().expect("root QQuickWindow should be focused");

        // Then: the conditions panel states the failure, and it is the only
        // thing on screen that does. The forecast list is itself reachable from
        // this same search, which is what makes the missing second label an
        // absence rather than a search that never got there.
        let conditions_error =
            child(window, "conditionsError").expect("conditionsError should exist");
        assert_eq!(
            property(conditions_error, "visible").value::<bool>(),
            Some(true)
        );
        assert_eq!(text_of(conditions_error), status);
        child(window, "forecastView").expect("forecastView should exist");
        assert!(
            child(window, "forecastError").is_none(),
            "the forecast list printed a second copy of the same sentence"
        );

        // Then: the hero carries the mark rather than a word it has to cut, and
        // asks for no more width than the column gave it, so nothing elides.
        let hero = child(window, "heroReading").expect("heroReading should exist");
        assert_eq!(text_of(hero), "—");
        let width = number_of(hero, "width");
        let implicit = number_of(hero, "implicitWidth");
        assert!(width > 0.0, "the hero should have been laid out");
        assert!(
            implicit <= width,
            "the hero wanted {implicit} px inside {width} px, so it elided"
        );
    }
}
