use std::sync::{Arc, Mutex};

use allesbuien_core::{
    location_settings::LocationSettings,
    weather::{FeedApi, FeedFuture, WeatherData},
};
use allesbuien_harness::HarnessBuilder;
use allesbuien_testkit::{SETTLE_BUDGET, clear_weather, pump_until, write_weather};

/// A feed that answers with nothing and remembers where it was asked about.
struct RecordingFeedApi {
    asked: Mutex<Vec<(f64, f64)>>,
}

impl RecordingFeedApi {
    fn asked(&self) -> Vec<(f64, f64)> {
        self.asked.lock().expect("asked lock").clone()
    }
}

impl FeedApi for RecordingFeedApi {
    fn fetch(&self, latitude: f64, longitude: f64) -> FeedFuture<'_> {
        self.asked
            .lock()
            .expect("asked lock")
            .push((latitude, longitude));
        Box::pin(async {
            Ok(WeatherData {
                current: None,
                forecast: Vec::new(),
                rain_line: String::new(),
                rain_soon: None,
                rain_series: Vec::new(),
            })
        })
    }
}

/// The five bound values this harness reads back out of the two bridges.
#[derive(Debug, PartialEq)]
struct Shown {
    name: String,
    source: String,
    coverage_notice: String,
    marker: Option<(f64, f64)>,
}

fn shown(booted: &allesbuien::BootedApp) -> Shown {
    let weather = booted.weather_controller().expect("weather controller");
    let radar = booted.radar_controller().expect("radar controller");
    Shown {
        name: weather.location_name().to_string(),
        source: weather.location_source().to_string(),
        coverage_notice: radar.coverage_notice().to_string(),
        marker: radar
            .marker_visible()
            .then(|| (*radar.marker_x_fraction(), *radar.marker_y_fraction())),
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this one-process GUI regression keeps set, unchanged, clear, and out-of-coverage location transitions in one ordered narrative"
)]
fn follows_the_desktop_location_setting_while_the_window_stays_open() {
    // Given: one GUI application over a temporary home that already names Utrecht.
    let home = tempfile::tempdir().expect("temporary home");
    write_weather(
        home.path(),
        r#"{"name":"Utrecht","latitude":52.09,"longitude":5.12}"#,
    );
    let feed = Arc::new(RecordingFeedApi {
        asked: Mutex::new(Vec::new()),
    });
    let mut booted = HarnessBuilder::with_feed(feed.clone())
        .location(LocationSettings::watching(home.path().to_owned()))
        .boot();

    let opened = shown(&booted);
    let (utrecht_x, utrecht_y) = opened.marker.expect("Utrecht is on the Dutch radar");
    assert_eq!(opened.name, "Utrecht");
    assert_eq!(opened.source, "Via Omarchy");
    assert_eq!(opened.coverage_notice, "");
    pump_until(
        "the opening refresh asks about Utrecht",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            feed.asked() == [(52.09, 5.12)]
        },
    );

    // When: `omarchy-weather-location --set Groningen 53.22,6.57` rewrites it.
    write_weather(
        home.path(),
        r#"{"name":"Groningen","latitude":53.22,"longitude":6.57}"#,
    );
    booted.poll_location();
    let moved = shown(&booted);
    let (groningen_x, groningen_y) = moved.marker.expect("Groningen is on the Dutch radar");

    // Then: the window followed without a restart, and refetched the new place.
    assert_eq!(moved.name, "Groningen");
    assert_eq!(moved.source, "Via Omarchy");
    assert!(
        groningen_x > utrecht_x && groningen_y < utrecht_y,
        "Groningen is north-east of Utrecht, so the marker moves right and up: \
         {moved:?} against ({utrecht_x}, {utrecht_y})"
    );
    pump_until("the move refetches Groningen", SETTLE_BUDGET, || {
        booted.process_events();
        feed.asked() == [(52.09, 5.12), (53.22, 6.57)]
    });

    // When: a poll observes nothing at all changed.
    booted.poll_location();
    booted.process_events();

    // Then: nothing moved and nothing was refetched, so a 2 s tick is not a 2 s refresh.
    assert_eq!(shown(&booted), moved);
    assert_eq!(feed.asked(), [(52.09, 5.12), (53.22, 6.57)]);

    // When: `omarchy-weather-location --clear` removes the file outright.
    clear_weather(home.path());
    booted.poll_location();
    let cleared = shown(&booted);

    // Then: the fallback is shown as a fallback rather than as a chosen place.
    assert_eq!(cleared.name, "Amsterdam-West");
    assert_eq!(cleared.source, "No setting");
    assert_eq!(cleared.coverage_notice, "");
    assert!(cleared.marker.is_some(), "Amsterdam-West is on the radar");

    // When: the desktop is set to a place this app's radar cannot cover.
    write_weather(
        home.path(),
        r#"{"name":"Berlin","latitude":52.52,"longitude":13.4}"#,
    );
    booted.poll_location();
    let uncovered = shown(&booted);

    // Then: the missing marker is explained rather than left looking like a fault.
    assert_eq!(uncovered.name, "Berlin");
    assert_eq!(uncovered.source, "Via Omarchy");
    assert_eq!(
        uncovered.coverage_notice,
        "Berlin is outside the Dutch radar — no map marker"
    );
    assert_eq!(uncovered.marker, None, "no marker outside the raster");
}
