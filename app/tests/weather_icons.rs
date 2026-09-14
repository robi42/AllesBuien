//! Condition icons never hold the weather text back, and arrive one by one.
//!
//! Every assertion reads the rendered tree rather than the model's `data()`: a
//! delegate's bound `source` only changes when the controller actually notified
//! it, so the per-icon `dataChanged` and the hero property's re-publish are what
//! is under test, not the provider store they both consult.

mod icon_sources;

use std::{ptr::NonNull, sync::Arc};

use allesbuien::{icon_cache::IconCache, weather_controller::ffi::WeatherController};
use allesbuien_core::weather::{FeedApi, FeedFuture};
use allesbuien_harness::{
    Harness, HarnessBuilder,
    probe::{QObject, child, delegate, descendant, focused_window, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, fixtures::five_day_weather, pump_until};
use cxx_qt_lib::QUrl;
use icon_sources::ScriptedIconSource;

/// The shared five-day fixture with its last day's code changed to `a`, so the
/// hero (`qq`), the first four rows (`f`), and the last row are three keys.
struct ThreeCodeFeedApi;

impl FeedApi for ThreeCodeFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            let mut weather = five_day_weather();
            if let Some(last) = weather.forecast.last_mut() {
                last.condition_code = String::from("a");
            }
            Ok(weather)
        })
    }
}

/// One condition icon in the window: the hero, or a forecast row's.
#[derive(Clone, Copy)]
enum Icon {
    Hero,
    Row(i32),
}

/// The live `ConditionIcon` item for `icon`, looked up afresh on every call
/// because a model reset replaces the row delegates.
fn item(icon: Icon) -> Option<NonNull<QObject>> {
    // SAFETY: called only on the GUI thread while the booted app is alive, and
    // `forecastView` is a `ListView`.
    unsafe {
        let window = focused_window()?;
        match icon {
            Icon::Hero => child(window, "heroIcon"),
            Icon::Row(row) => delegate(child(window, "forecastView")?, row)
                .and_then(|delegate| descendant(delegate, "forecastIcon")),
        }
    }
}

/// The URL `icon` is currently bound to.
fn bound_source(icon: Icon) -> String {
    let item = item(icon).expect("the icon should be rendered");
    // SAFETY: `item` was just returned by the probes on the GUI thread.
    unsafe { property(item, "source") }
        .value::<QUrl>()
        .expect("an icon source should be a URL")
        .to_string()
}

/// Whether `icon`'s skeleton is currently shown.
fn skeleton_visible(icon: Icon) -> bool {
    let item = item(icon).expect("the icon should be rendered");
    // SAFETY: `item` was just returned by the probes on the GUI thread.
    unsafe {
        let skeleton = descendant(item, "skeleton").expect("every icon has a skeleton");
        property(skeleton, "visible")
            .value::<bool>()
            .expect("visible should be a bool")
    }
}

/// Pumps `booted`'s event loop until `check` holds.
fn settle(booted: &Harness, condition: &str, mut check: impl FnMut() -> bool) {
    pump_until(condition, SETTLE_BUDGET, || {
        booted.process_events();
        check()
    });
}

fn controller(booted: &Harness) -> &WeatherController {
    booted
        .weather_controller()
        .expect("controller should exist")
}

#[test]
fn icons_follow_the_text_one_by_one_and_recover_on_a_later_refresh() {
    // Given: `F` held back, `QQ` refused once, `A` served at once.
    let directory = tempfile::tempdir().expect("a temporary icon cache");
    let (source, release) = ScriptedIconSource::new(&["F"], &["QQ"]);
    let icons = IconCache::new(directory.path().to_owned(), source.clone());

    // When: boot publishes the first refresh.
    let mut booted = HarnessBuilder::with_feed(Arc::new(ThreeCodeFeedApi))
        .icons(icons)
        .boot();
    settle(&booted, "the text and every row are on screen", || {
        item(Icon::Row(4)).is_some()
            && controller(&booted).temperature().to_string() == "19.5 °C"
            && source.calls("QQ") == 1
    });

    // Then: the text is on screen while no icon is, and the pending slots say so.
    assert_eq!(bound_source(Icon::Hero), "");
    assert_eq!(bound_source(Icon::Row(0)), "");
    assert!(skeleton_visible(Icon::Hero));
    assert!(skeleton_visible(Icon::Row(0)));

    // Then: the served icon shows while another is still downloading.
    settle(
        &booted,
        "row 4 shows its icon and drops its skeleton",
        || bound_source(Icon::Row(4)) == "image://condition/A" && !skeleton_visible(Icon::Row(4)),
    );
    assert_eq!(bound_source(Icon::Row(0)), "");

    // When: a later refresh asks again for the icon that failed.
    booted.refresh_weather();

    // Then: the same condition code is re-published once it can be served.
    settle(&booted, "the refused hero icon recovers", || {
        bound_source(Icon::Hero) == "image://condition/QQ"
    });
    assert_eq!(source.calls("QQ"), 2);
    assert_eq!(bound_source(Icon::Row(0)), "");

    // When: the held-back download completes.
    release.send(()).expect("the gate should still be awaited");

    // Then: its rows show it, loading ends, and no skeleton is left behind.
    settle(&booted, "row 0 shows its icon and loading ends", || {
        bound_source(Icon::Row(0)) == "image://condition/F" && !*controller(&booted).icons_loading()
    });
    assert!(!skeleton_visible(Icon::Hero));
    assert!(!skeleton_visible(Icon::Row(0)));
}
