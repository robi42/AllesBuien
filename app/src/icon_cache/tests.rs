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

use super::{IconCache, IconFuture, IconSource, condition_codes, icon_body, missing_keys};
use crate::icon_store::MAX_ICON_BYTES;

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
    let first = block_on(cache(&directory, Arc::clone(&first_source)).read_or_download("QQ"));
    let second_source = CountingSource::serving(PNG);

    // When
    let second = block_on(cache(&directory, Arc::clone(&second_source)).read_or_download("QQ"));

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
    let entry = block_on(cache(&directory, Arc::clone(&source)).read_or_download("Q"));

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
        cache(&directory, Arc::clone(&source))
            .warm(&[String::from("BAD"), String::from("GOOD")], &|_| {}),
    );

    // Then
    assert_eq!(source.calls(), 1);
    assert!(!directory.path().join("BAD.png").exists());
    assert!(crate::image_provider::has_icon("GOOD"));
}

#[test]
fn restoring_publishes_disk_entries_without_asking_the_source() {
    // Given: one stored icon, one stored damaged file, and one code never stored.
    let directory = tempdir().expect("temp cache");
    std::fs::write(directory.path().join("RA.png"), PNG).expect("cached icon");
    std::fs::write(directory.path().join("RB.png"), b"\x89PN").expect("partial entry");
    let source = CountingSource::serving(PNG);

    // When
    cache(&directory, Arc::clone(&source)).restore(&[
        String::from("ra"),
        String::from("RB"),
        String::from("RE"),
    ]);

    // Then
    assert_eq!(source.calls(), 0);
    assert!(crate::image_provider::has_icon("RA"));
    assert!(!crate::image_provider::has_icon("RB"));
    assert!(!crate::image_provider::has_icon("RE"));
}

#[test]
fn warming_settles_every_key_once_whether_or_not_it_was_published() {
    // Given: one valid download, and one response that is no PNG at all.
    let directory = tempdir().expect("temp cache");
    let valid = CountingSource::serving(PNG);
    let damaged = CountingSource::serving(b"<html>404 not found</html>");
    let settled = std::sync::Mutex::new(Vec::new());
    let on_settled = |key: &str| {
        settled.lock().expect("settled keys").push(key.to_owned());
    };

    // When
    block_on(cache(&directory, Arc::clone(&valid)).warm(&[String::from("rc")], &on_settled));
    block_on(cache(&directory, Arc::clone(&damaged)).warm(&[String::from("RD")], &on_settled));

    // Then
    assert_eq!((valid.calls(), damaged.calls()), (1, 1));
    assert_eq!(*settled.lock().expect("settled keys"), ["RC", "RD"]);
    assert!(crate::image_provider::has_icon("RC"));
    assert!(!crate::image_provider::has_icon("RD"));
}

#[test]
fn only_distinct_usable_codes_the_provider_lacks_are_missing() {
    // Given: a key no other test in this process stores.
    let codes = [
        String::from("rg"),
        String::from("RG"),
        String::from("../etc/passwd"),
        String::new(),
    ];
    let before = missing_keys(&codes);

    // When
    crate::image_provider::store_icon("RG", b"icon".to_vec().into());

    // Then
    assert_eq!(before, ["RG"]);
    assert_eq!(missing_keys(&codes), [] as [String; 0]);
}

#[test]
fn a_disabled_cache_never_reaches_for_the_network() {
    // Given / When
    let entry = block_on(IconCache::disabled().read_or_download("QQ"));

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
            .warm(&[String::new(), String::from("../etc/passwd")], &|_| {}),
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
    block_on(cache(&directory, Arc::clone(&source)).warm(&codes, &|_| {}));

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

/// A successful answer carrying `body` and declaring no length, so the reader
/// can only refuse it as it streams.
fn answered(body: Vec<u8>) -> reqwest::Response {
    http::Response::new(body).into()
}

#[test]
fn an_icon_body_past_its_limit_is_a_miss_and_a_small_one_is_served() {
    // Given: a PNG one byte past the limit, and an ordinary one.
    let mut oversized = PNG.to_vec();
    oversized.resize(MAX_ICON_BYTES + 1, 0);

    // When
    let refused = block_on(icon_body(answered(oversized)));
    let served = block_on(icon_body(answered(PNG.to_vec())));

    // Then
    assert_eq!(refused, None);
    assert_eq!(served.as_deref(), Some(PNG));
}
