//! The feed and the radar `radar_ends_with_rain` scripts, captioned by one clock.
//!
//! The radar's captions are what the bridge makes of each stamp under the
//! `"timeOffset":0` this radar sends, `radar_clock::frame_label(stamp, Some(0))`,
//! and the rain rows carry those same captions, parsed with no shift, so a bar
//! and the frame stamped for it read the same `HH:MM` in every zone. The shift
//! production parses raintext with, `clock_shift_minutes()`, is taken at the
//! present instant rather than at the stamps' dates, so in winter, or in a zone
//! whose clocks change on other dates, the two would sit an hour apart.
//!
//! Frames 0 and 1 are stamped in the past and 2 to 9 in 2999, so the run splits
//! at 2 on any day: two observations, then predictions a graph can end. A run
//! stamped wholly in the past, as the other radar harnesses' are, is wholly
//! observed, and no end cuts it. Both Augusts keep Amsterdam's offset and that
//! of every zone `bin/test` runs in, so the ten captions stay five minutes
//! apart; none of them crosses midnight, which the core tests cover.

use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use allesbuien::radar_clock::frame_label;
use allesbuien_core::{
    radar::{RadarApi, RadarFuture},
    rain::parse_series,
    weather::{FeedApi, FeedFuture, ForecastDay, WeatherData, WeatherError},
};
use futures::channel::oneshot;

/// The run's frame stamps, naive UTC as the vendor sends them.
const STAMPS: [&str; 10] = [
    "2026-08-30T09:00:00",
    "2026-08-30T09:05:00",
    "2999-08-30T09:10:00",
    "2999-08-30T09:15:00",
    "2999-08-30T09:20:00",
    "2999-08-30T09:25:00",
    "2999-08-30T09:30:00",
    "2999-08-30T09:35:00",
    "2999-08-30T09:40:00",
    "2999-08-30T09:45:00",
];

/// The caption the radar bridge shows for frame `index` of every run.
pub(crate) fn caption(index: usize) -> String {
    frame_label(STAMPS[index], Some(0))
}

/// What the feed answers its next fetch with.
#[derive(Clone, Copy)]
pub(crate) enum Answer {
    /// A forecast without a rain series, so there is no graph to end at.
    Empty,
    /// A rain graph whose bars stand for frames `first` through `last`.
    Window(usize, usize),
    /// A fetch that fails, which leaves the graph on screen as it was.
    Failing,
}

/// A feed that answers each fetch with the [`Answer`] it was given last.
pub(crate) struct ScriptedFeed {
    answer: Mutex<Answer>,
}

impl ScriptedFeed {
    pub(crate) fn new(answer: Answer) -> Self {
        Self {
            answer: Mutex::new(answer),
        }
    }

    /// Answers every fetch from now on with `answer`.
    pub(crate) fn answer(&self, answer: Answer) {
        *self
            .answer
            .lock()
            .expect("no fetch panics holding the answer") = answer;
    }
}

impl FeedApi for ScriptedFeed {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        let answer = *self
            .answer
            .lock()
            .expect("no fetch panics holding the answer");
        Box::pin(async move {
            let rain_series = match answer {
                Answer::Empty => Vec::new(),
                Answer::Window(first, last) => {
                    let rows: Vec<String> = (first..=last)
                        .map(|index| format!("000|{}", caption(index)))
                        .collect();
                    parse_series(&rows.join("\n"), 0).points
                }
                // A client that cannot be built, which the retry ladder gives
                // up on at once, so the failed refresh lands without waiting
                // out its backoff.
                Answer::Failing => {
                    let error = reqwest::Client::builder()
                        .user_agent("\n")
                        .build()
                        .expect_err("the invalid user agent must prevent client construction");
                    return Err(WeatherError::ClientBuild(error));
                }
            };
            // One forecast day, so a failure after it reads as out of date
            // rather than unavailable.
            Ok(WeatherData {
                forecast: vec![ForecastDay::default()],
                rain_line: String::from("next 2h: dry"),
                rain_series,
                ..WeatherData::default()
            })
        })
    }
}

/// A radar that answers every metadata request with a new ten-frame run.
///
/// Each run carries its own run id, so the cache never already holds it and
/// every refresh downloads all ten frames: the frame count says exactly how
/// many runs were fetched.
#[derive(Default)]
pub(crate) struct TenFrameRadar {
    metadata_calls: AtomicUsize,
    frame_calls: AtomicUsize,
    held: Mutex<Option<oneshot::Receiver<()>>>,
}

impl TenFrameRadar {
    pub(crate) fn metadata_calls(&self) -> usize {
        self.metadata_calls.load(Ordering::SeqCst)
    }

    pub(crate) fn frame_calls(&self) -> usize {
        self.frame_calls.load(Ordering::SeqCst)
    }

    /// Holds the next run's metadata until the returned sender fires.
    pub(crate) fn hold_next(&self) -> oneshot::Sender<()> {
        let (release, held) = oneshot::channel();
        *self
            .held
            .lock()
            .expect("no request panics holding the hold") = Some(held);
        release
    }
}

impl RadarApi for TenFrameRadar {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        let call = self.metadata_calls.fetch_add(1, Ordering::SeqCst) + 1;
        let held = self
            .held
            .lock()
            .expect("no request panics holding the hold")
            .take();
        let times = STAMPS
            .iter()
            .enumerate()
            .map(|(index, stamp)| {
                format!(r#"{{"timestamp":"{stamp}","url":"fake://{call}/{index}"}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let body =
            format!(r#"{{"timestamp":"2999083009{call:02}","timeOffset":0,"times":[{times}]}}"#);
        Box::pin(async move {
            if let Some(held) = held {
                held.await.ok();
            }
            Ok(body.into_bytes())
        })
    }

    fn frame<'a>(&'a self, _url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        self.frame_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(allesbuien::BASEMAP_PNG.to_vec()) })
    }
}
