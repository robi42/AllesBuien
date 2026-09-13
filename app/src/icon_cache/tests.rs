//! Regression coverage for persistent, fail-soft, bounded icon warming.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
};

use allesbuien_core::weather::{CurrentConditions, ForecastDay, WeatherData};
use allesbuien_testkit::block_on;
use tempfile::{TempDir, tempdir};

use super::{IconCache, IconFuture, IconSource, condition_codes};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nbody";

struct CountingSource {
    calls: AtomicUsize,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    body: Vec<u8>,
}

impl CountingSource {
    fn serving(body: &[u8]) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            body: body.to_vec(),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }
}

impl IconSource for CountingSource {
    fn get(&self, _url: &str) -> IconFuture<'_> {
        let mut started = false;
        Box::pin(std::future::poll_fn(move |context| {
            if started {
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                return Poll::Ready(Some(self.body.clone()));
            }
            started = true;
            self.calls.fetch_add(1, Ordering::SeqCst);
            let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_in_flight.fetch_max(in_flight, Ordering::SeqCst);
            context.waker().wake_by_ref();
            Poll::Pending
        }))
    }
}

fn cache(directory: &TempDir, source: Arc<CountingSource>) -> IconCache {
    IconCache::new(directory.path().to_path_buf(), source)
}

#[test]
fn a_second_launch_reads_the_icon_off_disk_instead_of_the_network() {
    // Given: one warmed directory and a brand-new cache handle over it.
    let directory = tempdir().expect("temp cache");
    let first_source = CountingSource::serving(PNG);
    let first = block_on(cache(&directory, Arc::clone(&first_source)).entry("QQ"));
    let second_source = CountingSource::serving(PNG);

    // When
    let second = block_on(cache(&directory, Arc::clone(&second_source)).entry("QQ"));

    // Then
    assert_eq!(first_source.calls(), 1);
    assert_eq!(second_source.calls(), 0);
    assert_eq!(first.as_deref(), Some(PNG));
    assert_eq!(second.as_deref(), Some(PNG));
    assert!(directory.path().join("QQ.png").exists());
}

#[test]
fn a_damaged_cache_entry_is_refetched_rather_than_served() {
    // Given: a file whose contents can never decode as a PNG.
    let directory = tempdir().expect("temp cache");
    std::fs::write(directory.path().join("Q.png"), b"\x89PN").expect("partial entry");
    let source = CountingSource::serving(PNG);

    // When
    let entry = block_on(cache(&directory, Arc::clone(&source)).entry("Q"));

    // Then
    assert_eq!(source.calls(), 1);
    assert_eq!(entry.as_deref(), Some(PNG));
    assert_eq!(
        std::fs::read(directory.path().join("Q.png")).expect("rewritten entry"),
        PNG
    );
}

#[test]
fn an_undecodable_download_does_not_stop_another_icon_from_warming() {
    // Given: one damaged response beside one valid disk entry.
    let directory = tempdir().expect("temp cache");
    let source = CountingSource::serving(b"<html>404 not found</html>");
    std::fs::write(directory.path().join("GOOD.png"), PNG).expect("cached icon");

    // When
    block_on(
        cache(&directory, Arc::clone(&source)).warm(&[String::from("BAD"), String::from("GOOD")]),
    );

    // Then
    assert_eq!(source.calls(), 1);
    assert!(!directory.path().join("BAD.png").exists());
    assert!(crate::image_provider::has_icon("GOOD"));
}

#[test]
fn a_disabled_cache_never_reaches_for_the_network() {
    // Given / When
    let entry = block_on(IconCache::disabled().entry("QQ"));

    // Then
    assert!(entry.is_none());
}

#[test]
fn warming_an_unusable_code_publishes_nothing() {
    // Given
    let directory = tempdir().expect("temp cache");
    let source = CountingSource::serving(PNG);

    // When
    block_on(
        cache(&directory, Arc::clone(&source))
            .warm(&[String::new(), String::from("../etc/passwd")]),
    );

    // Then
    assert_eq!(source.calls(), 0);
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("temp cache")
            .count(),
        0
    );
}

#[test]
fn warming_fetches_every_payload_code_with_bounded_concurrency() {
    // Given: the largest possible payload-referenced working set.
    let data = WeatherData {
        current: Some(CurrentConditions {
            condition_code: String::from("BA"),
            ..CurrentConditions::default()
        }),
        forecast: ["BB", "BC", "BD", "BE", "BF"]
            .into_iter()
            .map(|code| ForecastDay {
                condition_code: code.to_owned(),
                ..ForecastDay::default()
            })
            .collect(),
        ..WeatherData::default()
    };
    let codes = condition_codes(&data);
    let expected_fetches = codes.len();
    let directory = tempdir().expect("temp cache");
    let source = CountingSource::serving(PNG);

    // When
    block_on(cache(&directory, Arc::clone(&source)).warm(&codes));

    // Then
    assert_eq!(source.calls(), expected_fetches);
    assert_eq!(source.max_in_flight(), 4);
    assert!(
        codes
            .iter()
            .all(|code| crate::image_provider::has_icon(code))
    );
}

#[test]
fn every_referenced_code_is_collected_once_and_blanks_are_dropped() {
    // Given
    let data = WeatherData {
        current: Some(CurrentConditions {
            condition_code: String::from("qq"),
            ..CurrentConditions::default()
        }),
        forecast: ["f", "qq", "", "a"]
            .into_iter()
            .map(|code| ForecastDay {
                condition_code: code.to_owned(),
                ..ForecastDay::default()
            })
            .collect(),
        ..WeatherData::default()
    };

    // When
    let codes = condition_codes(&data);

    // Then
    assert_eq!(codes, ["a", "f", "qq"]);
}
