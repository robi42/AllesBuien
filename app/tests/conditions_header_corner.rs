//! The header's right corner has to disappear, not merely go blank.
//!
//! Both corner captions bind `visible: text.length > 0`, and their parent
//! `ColumnLayout` binds its own visibility to theirs. Asserting those bindings
//! would prove nothing about the thing that actually goes wrong: a `RowLayout`
//! keeps its `spacing` in front of an item it still lays out, so a corner that
//! is blank rather than absent leaves the location name ending a gap early for
//! no stated reason. That is a layout outcome, so this reads it off the
//! rendered tree - the left column has to end exactly where the header row
//! does.
//!
//! It also guards the corner timestamp's `@`. That glyph is prefixed in QML,
//! and an unguarded `"@ " + observedLabel` would leave `text.length` at 2 with
//! no time behind it, so the caption would never go blank and this whole
//! collapse would stop happening - silently, because nothing else asserts the
//! empty case. The populated corner is `app/tests/conditions_observed_time.rs`;
//! the two scenarios contradict each other in one process, and `AGENTS.md`
//! allows each file one GUI test and one `QGuiApplication`.

use std::sync::Arc;

use allesbuien_core::weather::{CurrentConditions, FeedApi, FeedFuture, WeatherData};
use allesbuien_harness::{
    HarnessBuilder,
    probe::{child, focused_window, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, pump_until};

/// A station that reports readings but names neither itself nor a time.
///
/// The feed does not promise either field, and this is the shape that empties
/// both corner captions at once: `station_name` is what `stationLabel` reads,
/// and a default `observed_at` carries no time label for `observedLabel`.
struct NamelessStationFeedApi;

impl FeedApi for NamelessStationFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: Some(CurrentConditions {
                    temperature: Some(11.5),
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
fn a_feed_naming_no_station_leaves_no_gap_beside_the_location() {
    // Given: a booted window whose feed answers with readings but no station
    // name and no observation time.
    let booted = HarnessBuilder::with_feed(Arc::new(NamelessStationFeedApi)).boot();

    // When: that answer has reached the controller and the layout has settled.
    pump_until(
        "the nameless station reaches the header and the window is focused",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            let header_is_ready = booted.weather_controller().is_some_and(|controller| {
                controller.station_name().is_empty()
                    && controller.temperature().to_string() == "11.5 °C"
            });
            // SAFETY: on the GUI thread of the live booted app.
            header_is_ready && unsafe { focused_window() }.is_some()
        },
    );

    // SAFETY: the window and every item come from the probes on `booted`, which
    // outlives this block, and every call stays on the GUI thread.
    unsafe {
        let window = focused_window().expect("root QQuickWindow should be focused");
        let found =
            |name: &str| child(window, name).unwrap_or_else(|| panic!("{name} should exist"));
        let corner = found("headerCorner");
        let header_row = found("headerRow");
        let location_source = found("locationSource");

        // Then: the corner is gone rather than blank, and the left column runs
        // the full width of the header, so nothing was reserved for it.
        assert_eq!(
            property(corner, "visible").value::<bool>(),
            Some(false),
            "the corner should collapse when it has nothing to say"
        );
        let row_width = property(header_row, "width")
            .value::<f64>()
            .expect("the header row should have a width");
        let left_width = property(location_source, "width")
            .value::<f64>()
            .expect("the location column should have a width");
        assert!(row_width > 0.0, "the header row should have been laid out");
        assert!(
            (row_width - left_width).abs() < f64::EPSILON,
            "the collapsed corner still reserved {} px of the header",
            row_width - left_width
        );
    }
}
