//! The corner timestamp says it is a measurement time, not a clock.
//!
//! `SCHIPHOL` over a bare `12:30` names an instrument and a number and leaves
//! the relation between them to be inferred; `@ 12:30` states it. The `@` is
//! added in `ConditionsView.qml` rather than in `conditions_view.rs`, because
//! it is a typographic relation glyph on a caption row this file's sibling
//! already reads off the rendered tree - the controller keeps carrying the
//! plain `hh:mm` a notification or a test can assert without a window.
//!
//! This is a second file rather than an extra assertion inside
//! `conditions_header_corner.rs` because the two scenarios contradict each
//! other in one process. That harness proves the corner COLLAPSES when the feed
//! names neither station nor time, which needs both captions empty; this one
//! needs both populated. `AGENTS.md` pins every file under `app/tests/` to one
//! GUI test and one `QGuiApplication`, so the populated case has to boot its own
//! window rather than share that one.

use std::sync::Arc;

use allesbuien_core::{
    datetime::LocalTimestamp,
    weather::{CurrentConditions, FeedApi, FeedFuture, WeatherData},
};
use allesbuien_harness::{
    HarnessBuilder,
    probe::{child, focused_window, text_of},
};
use allesbuien_testkit::{SETTLE_BUDGET, pump_until};

/// A station that names itself and the minute it read.
///
/// `17:50` is the same fixture `conditions_view.rs` formats against, so the
/// controller's plain string and the corner's decorated one can be compared to
/// one another rather than each to a number of its own.
struct TimestampedStationFeedApi;

impl FeedApi for TimestampedStationFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    station_name: String::from("Schiphol"),
                    temperature: Some(11.5),
                    observed_at: LocalTimestamp::from_feed_in(
                        "2026-08-31T17:50:00",
                        "Europe/Amsterdam",
                    ),
                    ..CurrentConditions::default()
                }),
                forecast: Vec::new(),
                rain_line: String::new(),
                rain_soon: None,
                rain_series: Vec::new(),
            })
        })
    }
}

#[test]
fn the_corner_time_is_rendered_as_a_measurement_rather_than_a_clock() {
    // Given: a booted window whose feed names its station and the minute it read.
    let booted = HarnessBuilder::with_feed(Arc::new(TimestampedStationFeedApi)).boot();

    // When: that answer has reached the controller and the layout has settled.
    pump_until(
        "the timestamped station reaches the header and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let header_is_ready = booted
                .weather_controller()
                .is_some_and(|controller| controller.observed_label().to_string() == "17:50");
            // SAFETY: on the GUI thread of the live booted app.
            header_is_ready && unsafe { focused_window() }.is_some()
        },
    );

    // SAFETY: the window and every item come from the probes on `booted`, which
    // outlives this block, and every call stays on the GUI thread.
    unsafe {
        let window = focused_window().expect("root QQuickWindow should be focused");

        // Then: the time carries the relation, and the station beside it does
        // not. Reading both is what proves the corner was reached and that the
        // glyph belongs to the timestamp rather than to every caption in it.
        let station = child(window, "stationLabel").expect("stationLabel should exist");
        let observed = child(window, "observedLabel").expect("observedLabel should exist");
        assert_eq!(text_of(station), "SCHIPHOL");
        assert_eq!(text_of(observed), "@ 17:50");
    }
}
