//! The radar's animation ends on the rain graph's last bar, whichever feed
//! answers first, and every frame past it stays downloaded.
//!
//! The radar fetches half an hour more forecast than the rain graph spans, so
//! that a run trailing the present still reaches the graph's last bar; a fresh
//! run therefore carries frames past that bar, on which the graph's cursor has
//! nowhere to stand. The window plays, steps, and jumps to End only as far as
//! the frame on that bar. A later graph that reaches further brings the hidden
//! frames back from memory, without a fetch or a new generation, and a run that
//! lands after the graph arrives already cut.
//!
//! It is also the one end-to-end proof of the older handoff the other way, the
//! frame on screen placing the graph's cursor, since no other harness boots
//! both feeds with moments that overlap.
//!
//! The harness reads nothing the cut added, so it builds on a tree without the
//! cut, and fails there on its first wait for one.

mod rain_end_ports;

use std::{sync::Arc, time::Duration};

use allesbuien::{
    TimerIntervals, radar_controller::ffi::RadarController,
    weather_controller::ffi::WeatherController,
};
use allesbuien_harness::{
    Harness, HarnessBuilder,
    keys::{Key, key_click},
    probe::{child, focused_window, number_of, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, pump_until};
use cxx_qt_lib::{QModelIndex, QUrl};
use rain_end_ports::{Answer, ScriptedFeed, TenFrameRadar, caption};

/// The booted window and the two ports the test scripts it through.
struct Scene {
    booted: Harness,
    feed: Arc<ScriptedFeed>,
    vendor: Arc<TenFrameRadar>,
}

impl Scene {
    /// Boots over a feed without a rain series and the ten-frame radar, every
    /// timer slowed to an hour, so that only the test moves anything. Both
    /// bridges adopt their location, and the theme is read, at construction.
    fn boot() -> Self {
        let feed = Arc::new(ScriptedFeed::new(Answer::Empty));
        let vendor = Arc::new(TenFrameRadar::default());
        let idle = Duration::from_hours(1);
        let booted = HarnessBuilder::with_feed(feed.clone())
            .radar(vendor.clone())
            .intervals(TimerIntervals {
                weather_refresh: idle,
                radar_refresh: idle,
                radar_playback: idle,
                theme_poll: idle,
                location_poll: idle,
            })
            .boot();
        Self {
            booted,
            feed,
            vendor,
        }
    }

    fn radar(&self) -> &RadarController {
        self.booted
            .radar_controller()
            .expect("the radar bridge should exist")
    }

    fn weather(&self) -> &WeatherController {
        self.booted
            .weather_controller()
            .expect("the weather bridge should exist")
    }

    /// Whether the radar is playing, and the frame it is on.
    fn playback(&self) -> (bool, i32) {
        (*self.radar().playing(), *self.radar().current_frame_index())
    }

    /// The captions of the frames the radar shows.
    fn captions(&self) -> Vec<String> {
        self.radar()
            .timestamps()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Where the rain graph's cursor stands, or `None` while it is hidden.
    fn cursor(&self) -> Option<f64> {
        let weather = self.weather();
        (*weather.rain_cursor_visible()).then(|| *weather.rain_cursor_fraction())
    }

    /// Runs the event loop until `condition` holds.
    fn wait(&self, what: &str, condition: impl Fn(&Self) -> bool) {
        pump_until(what, SETTLE_BUDGET, || {
            self.booted.process_events();
            condition(self)
        });
    }

    /// Refreshes the weather to `answer`, and waits for its graph of `bars`.
    ///
    /// The wait is on the graph, the cause, so that a radar that never follows
    /// it fails on a wait naming the radar rather than this one.
    fn answer(&mut self, answer: Answer, bars: isize) {
        self.feed.answer(answer);
        self.booted.refresh_weather();
        self.wait(&format!("the rain graph holds {bars} bars"), |scene| {
            scene.weather().rain_levels().len() == bars
        });
    }

    /// Whether radar refresh number `generation` has put its run on screen.
    fn adopted(&self, generation: u64) -> bool {
        *self.radar().generation() == generation && *self.radar().frame_count() > 0
    }

    /// Presses `key` as a keyboard does, then reports the playback it left.
    fn press(&self, key: Key) -> (bool, i32) {
        // SAFETY: on the GUI thread of the live booted app, whose window the
        // probe returns.
        unsafe { key_click(focused_window().expect("the window is focused"), key) };
        self.booted.process_events();
        self.playback()
    }

    /// Whether the radar overlay is bound to the frame the radar is on, and has
    /// decoded it: a frame the image store does not hold reports an error.
    fn overlay_ready(&self) -> bool {
        let url = self.radar().current_frame_url().to_string();
        // SAFETY: as in `press`.
        unsafe {
            let window = focused_window().expect("the window is focused");
            let overlay = child(window, "radarOverlay").expect("the overlay should exist");
            let bound = property(overlay, "source")
                .value::<QUrl>()
                .is_some_and(|source| source.to_string() == url);
            bound && property(overlay, "status").value::<i32>() == Some(1)
        }
    }
}

/// The last position of the radar's slider, the frame End lands on.
fn slider_end() -> f64 {
    // SAFETY: on the GUI thread of the live booted app, whose window the probe
    // returns.
    unsafe {
        let window = focused_window().expect("the window is focused");
        number_of(
            child(window, "frameSlider").expect("the slider should exist"),
            "to",
        )
    }
}

/// Whether `actual` is `expected`, to within rounding.
fn near(actual: Option<f64>, expected: f64) -> bool {
    actual.is_some_and(|actual| (actual - expected).abs() < 1e-9)
}

/// The captions of frames `first` through `last`.
fn captions_of(first: usize, last: usize) -> Vec<String> {
    (first..=last).map(caption).collect()
}

#[test]
fn the_radar_animation_ends_on_the_rain_graphs_last_bar() {
    let mut scene = Scene::boot();
    without_a_graph_the_whole_run_plays(&mut scene);
    a_graph_ends_the_run_on_its_last_bar(&mut scene);
    the_transport_stops_at_that_bar(&mut scene);
    a_later_graph_brings_frames_back_from_memory(&mut scene);
    the_next_run_arrives_already_cut(&mut scene);
    a_run_still_loading_is_cut_when_it_lands(&mut scene);
    a_failed_weather_refresh_keeps_the_cut(&mut scene);
    a_refresh_without_a_series_cuts_nothing(&mut scene);
}

/// With no rain graph to end at, the whole run plays, from its first frame.
fn without_a_graph_the_whole_run_plays(scene: &mut Scene) {
    // Given: the boot refreshes, a forecast without a rain series and a run.
    scene.wait("a focused window adopts all ten frames", |scene| {
        // SAFETY: on the GUI thread of the live booted app.
        let focused = unsafe { focused_window() }.is_some();
        let forecast = scene.weather().row_count(&QModelIndex::default()) == 1;
        focused && forecast && *scene.radar().frame_count() == 10
    });

    // Then
    assert_eq!(scene.captions(), captions_of(0, 9));
    assert_eq!(scene.playback(), (true, 0), "a run plays from its start");
    let fraction = *scene.radar().observed_fraction();
    assert!(
        near(Some(fraction), 1.5 / 9.0),
        "two observations: {fraction}"
    );
    assert_eq!(scene.vendor.frame_calls(), 10);

    // When: the viewer parks on the last frame.
    scene.booted.seek_radar(9);

    // Then
    assert_eq!(scene.playback(), (false, 9));
}

/// A graph that ends inside the run's predictions ends the run on its last bar.
fn a_graph_ends_the_run_on_its_last_bar(scene: &mut Scene) {
    // When: a graph whose bars stand for frames 2 to 6.
    scene.answer(Answer::Window(2, 6), 5);
    scene.wait(
        "the run ends on frame 6, the rain graph's last bar",
        |scene| *scene.radar().frame_count() == 7,
    );

    // Then: the viewer, parked on a frame now hidden, is moved to the last one
    // shown and left paused, and the graph's cursor stands on its last bar.
    let radar = scene.radar();
    assert_eq!(scene.captions(), captions_of(0, 6));
    let axis: Vec<String> = scene
        .weather()
        .rain_axis_labels()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(axis.last(), Some(&caption(6)), "the last bar is frame 6's");
    assert_eq!(scene.playback(), (false, 6));
    assert_eq!(radar.current_frame_url().to_string(), "image://radar/1/6");
    assert!(near(scene.cursor(), 4.5 / 5.0), "{:?}", scene.cursor());
    // The divider is measured against the frames shown, not the whole run.
    let fraction = *radar.observed_fraction();
    assert!(near(Some(fraction), 1.5 / 6.0), "fraction {fraction}");
    assert!(*radar.forecast_boundary_visible());
    let end = slider_end();
    assert!(near(Some(end), 6.0), "the slider ends at {end}");
    // A cut fetches nothing and starts no generation.
    assert_eq!((scene.vendor.frame_calls(), *radar.generation()), (10, 1));
}

/// Home, End, Right, Space, and the playback loop all stop at that bar.
fn the_transport_stops_at_that_bar(scene: &mut Scene) {
    // When / Then
    assert_eq!(scene.press(Key::Home), (false, 0));
    assert_eq!(scene.cursor(), None, "frame 0 is before the graph");
    assert_eq!(scene.press(Key::End), (false, 6), "End is the last bar's");
    assert_eq!(scene.press(Key::Right), (false, 6), "nothing past it");
    assert_eq!(scene.press(Key::Space), (true, 6));
    scene.booted.tick_radar();
    assert_eq!(scene.playback(), (true, 0), "the loop wraps after the bar");
}

/// A later graph that reaches further brings frames back, from memory.
fn a_later_graph_brings_frames_back_from_memory(scene: &mut Scene) {
    // When: a graph whose bars stand for frames 3 to 8.
    scene.answer(Answer::Window(3, 8), 6);
    scene.wait("the run reaches frame 8, the new last bar", |scene| {
        *scene.radar().frame_count() == 9
    });

    // Then: the viewer keeps their frame and their playback, and nothing was
    // fetched again.
    assert_eq!(scene.playback(), (true, 0));
    let fetched = (scene.vendor.metadata_calls(), scene.vendor.frame_calls());
    assert_eq!(fetched, (1, 10));
    assert_eq!(*scene.radar().generation(), 1);

    // When: the viewer goes to frame 8, which was never on screen.
    scene.booted.seek_radar(8);

    // Then: the store still holds it.
    let url = scene.radar().current_frame_url().to_string();
    assert_eq!(url, "image://radar/1/8");
    scene.wait("frame 8 decodes from the store", Scene::overlay_ready);
    assert!(near(scene.cursor(), 5.5 / 6.0), "{:?}", scene.cursor());
}

/// The next run arrives already cut, at the end the graph still has.
fn the_next_run_arrives_already_cut(scene: &mut Scene) {
    // When
    scene.booted.refresh_radar();
    scene.wait("the second run is adopted", |scene| scene.adopted(2));

    // Then: every frame of it downloaded, and frame 9 is hidden.
    assert_eq!(*scene.radar().frame_count(), 9);
    assert_eq!(scene.playback(), (true, 0));
    assert_eq!(scene.vendor.frame_calls(), 20);
    assert_eq!(scene.cursor(), None, "frame 0 is before the graph");
}

/// An end that moves while a run is loading waits for that run.
fn a_run_still_loading_is_cut_when_it_lands(scene: &mut Scene) {
    // Given: a refresh whose run is held back.
    let release = scene.vendor.hold_next();
    scene.booted.refresh_radar();

    // When: the graph moves while the run is still loading, and then it lands.
    scene.answer(Answer::Window(2, 5), 4);
    release.send(()).expect("the third run's metadata is held");
    scene.wait("the third run is adopted", |scene| scene.adopted(3));

    // Then: it lands cut at the new end. A cut published while the run was
    // still loading would have bound frame 0 before the store held it, and the
    // same URL published again on landing would never have reloaded it.
    assert_eq!(*scene.radar().frame_count(), 6);
    assert_eq!(scene.playback(), (true, 0));
    scene.wait("the third run's first frame decodes", Scene::overlay_ready);
}

/// A failed weather refresh keeps its graph on screen, and so keeps the cut.
fn a_failed_weather_refresh_keeps_the_cut(scene: &mut Scene) {
    // Given
    scene.feed.answer(Answer::Failing);
    scene.booted.refresh_weather();
    scene.wait("the weather refresh fails", |scene| {
        scene
            .weather()
            .status()
            .to_string()
            .starts_with("Weather may be out of date")
    });

    // When
    scene.booted.refresh_radar();
    scene.wait("the fourth run is adopted", |scene| scene.adopted(4));

    // Then: cut at the graph still on screen.
    assert_eq!(*scene.radar().frame_count(), 6);
    assert_eq!(scene.weather().rain_levels().len(), 4);
}

/// A refresh without a rain series leaves nothing to end at.
fn a_refresh_without_a_series_cuts_nothing(scene: &mut Scene) {
    // When
    scene.answer(Answer::Empty, 0);
    scene.wait("the whole run plays again", |scene| {
        *scene.radar().frame_count() == 10
    });

    // Then: even a frame hidden since its run landed comes back from memory.
    assert_eq!(scene.captions(), captions_of(0, 9));
    assert_eq!(scene.playback(), (true, 0));
    scene.booted.seek_radar(9);
    scene.wait("frame 9, hidden on landing, decodes", Scene::overlay_ready);
}
