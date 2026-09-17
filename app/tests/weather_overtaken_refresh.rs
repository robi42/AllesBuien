//! A refresh whose answer arrives after a later one's never reaches the window.
//!
//! Refreshes overlap in normal use: the location poll refreshes while the
//! recurring timer's fetch is still out. The slow one here answers last, with a
//! different station, temperature, and condition code, so every way it could
//! show through — the conditions text, the rows, the hero icon — would be
//! visible if its updates were still applied.

mod icon_sources;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use allesbuien::icon_cache::IconCache;
use allesbuien_core::{
    datetime::LocalTimestamp,
    weather::{CurrentConditions, FeedApi, FeedFuture, ForecastDay, WeatherData},
};
use allesbuien_harness::{Harness, HarnessBuilder};
use allesbuien_testkit::{SETTLE_BUDGET, fixtures::five_day_weather, pump_until};
use futures::{
    FutureExt,
    channel::oneshot::{self, Sender},
};
use icon_sources::ScriptedIconSource;

/// Answers the first fetch last, with a payload nothing else could produce.
struct OvertakenFeedApi {
    calls: AtomicUsize,
    slow: futures::future::Shared<oneshot::Receiver<()>>,
}

impl OvertakenFeedApi {
    fn new() -> (Arc<Self>, Sender<()>) {
        let (release, slow) = oneshot::channel();
        let feed = Self {
            calls: AtomicUsize::new(0),
            slow: slow.shared(),
        };
        (Arc::new(feed), release)
    }
}

impl FeedApi for OvertakenFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        let slow = first.then(|| self.slow.clone());
        Box::pin(async move {
            match slow {
                Some(gate) => {
                    gate.await.ok();
                    Ok(overtaken_weather())
                }
                None => Ok(five_day_weather()),
            }
        })
    }
}

/// The older answer: another station, another reading, and the code `z`, which
/// no other payload in this harness carries.
fn overtaken_weather() -> WeatherData {
    WeatherData {
        current: Some(CurrentConditions {
            station_name: String::from("Overtaken"),
            temperature: Some(-3.5),
            condition_code: String::from("z"),
            observed_at: LocalTimestamp::from_feed_in("2026-08-31T16:50:00", "Europe/Amsterdam"),
            ..CurrentConditions::default()
        }),
        forecast: vec![ForecastDay {
            day: String::from("2026-09-01T00:00:00"),
            condition_code: String::from("z"),
            day_at: LocalTimestamp::from_feed_in("2026-09-01T12:00:00", "Europe/Amsterdam"),
            ..ForecastDay::default()
        }],
        ..WeatherData::default()
    }
}

fn station(booted: &Harness) -> String {
    booted
        .weather_controller()
        .expect("controller should exist")
        .station_name()
        .to_string()
}

#[test]
fn an_overtaken_refresh_never_puts_its_older_payload_back() {
    // Given: a first fetch that will answer last, and an icon source that serves
    // every code, so the overtaken payload's `z` is asked for once it answers.
    let directory = tempfile::tempdir().expect("a temporary icon cache");
    let (source, _hold) = ScriptedIconSource::new(&[], &[]);
    let (feed, answer_first) = OvertakenFeedApi::new();
    let mut booted = HarnessBuilder::with_feed(feed.clone())
        .icons(IconCache::new(directory.path().to_owned(), source.clone()))
        .boot();

    // When: a second refresh overtakes the boot refresh and lands first.
    booted.refresh_weather();
    pump_until("the later refresh is on screen", SETTLE_BUDGET, || {
        booted.process_events();
        station(&booted) == "Schiphol"
    });
    assert_eq!(feed.calls.load(Ordering::SeqCst), 2);

    // When: the overtaken fetch finally answers and runs to its own last step,
    // which is asking for the icon only its payload references.
    answer_first.send(()).expect("the first fetch is waiting");
    // Asking for `Z` happens only after that refresh has already queued its
    // payload onto the Qt thread, so one more drain delivers whatever it queued.
    pump_until(
        "the overtaken refresh reaches its icons",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            source.calls("Z") == 1
        },
    );
    booted.process_events();

    // Then: the window still shows the later refresh, text and icons alike.
    let controller = booted
        .weather_controller()
        .expect("controller should exist");
    assert_eq!(controller.station_name().to_string(), "Schiphol");
    assert_eq!(controller.temperature().to_string(), "19.5 °C");
    assert_eq!(controller.observed_label().to_string(), "17:50");
    assert_eq!(
        controller.condition_icon_url().to_string(),
        "image://condition/QQ"
    );
    assert_eq!(controller.row_count(&cxx_qt_lib::QModelIndex::default()), 5);
    assert!(!*controller.hero_icon_loading());
}
