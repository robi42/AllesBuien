//! Warms the ~22 Buienradar condition icons into the image provider.
//!
//! QML used to bind straight to `https://cdn.buienradar.nl/...`, so every start
//! paid a fresh round trip and the icons appeared about a second after the text.
//! Each run's codes are warmed from [`crate::icon_store`] before their URLs
//! reach QML, and thereafter served through the `image://condition/<CODE>`
//! provider — the second launch never touches the network.
//!
//! Every failure degrades to "no icon" rather than to an error: a miss, an
//! unreachable CDN, an oversized body, or a damaged file all leave the code
//! unwarmed, and the QML slot collapses exactly as it does for an unknown code.

use std::{collections::BTreeSet, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use allesbuien_core::weather::WeatherData;

use crate::{condition_icon, icon_store, image_provider};

pub type IconFuture<'a> = Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>>;

/// Fetches one icon body, or [`None`] for any unusable response.
pub trait IconSource: Send + Sync {
    fn get(&self, url: &str) -> IconFuture<'_>;
}

/// A warmable icon cache: a directory to persist in and a way to fill it.
#[derive(Clone, Default)]
pub struct IconCache {
    directory: Option<PathBuf>,
    source: Option<Arc<dyn IconSource>>,
}

impl IconCache {
    /// A cache that neither reads nor writes, for callers that supply no source.
    #[must_use]
    pub fn disabled() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn new(directory: PathBuf, source: Arc<dyn IconSource>) -> Self {
        Self {
            directory: Some(directory),
            source: Some(source),
        }
    }

    /// Publishes every requested code to the provider, downloading the misses.
    pub async fn warm(&self, codes: &[String]) {
        for key in codes
            .iter()
            .filter_map(|code| condition_icon::cache_key(code))
        {
            if image_provider::has_icon(&key) {
                continue;
            }
            if let Some(bytes) = self.entry(&key).await {
                image_provider::store_icon(&key, bytes.into());
            }
        }
    }

    /// Reads one icon off disk, downloading and persisting it on a miss.
    async fn entry(&self, key: &str) -> Option<Vec<u8>> {
        let (directory, source) = (self.directory.as_ref()?, self.source.as_ref()?);
        let path = icon_store::entry_path(directory, key);
        if let Some(cached) = icon_store::read(&path) {
            return Some(cached);
        }
        let downloaded = source.get(&condition_icon::icon_url(key, "")).await?;
        if !icon_store::is_complete_png(&downloaded) {
            return None;
        }
        icon_store::write(directory, &path, &downloaded);
        Some(downloaded)
    }
}

/// Downloads icons over HTTP, treating any non-success response as a miss.
pub struct ReqwestIconSource(reqwest::Client);

impl ReqwestIconSource {
    /// Builds a transport, or [`None`] when the HTTP client cannot be created.
    #[must_use]
    pub fn new() -> Option<Self> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .ok()
            .map(Self)
    }
}

impl IconSource for ReqwestIconSource {
    fn get(&self, url: &str) -> IconFuture<'_> {
        let request = self.0.get(url).send();
        Box::pin(async move {
            let response = request.await.ok()?.error_for_status().ok()?;
            response.bytes().await.ok().map(|body| body.to_vec())
        })
    }
}

/// Every condition code one weather payload references, deduplicated.
#[must_use]
pub fn condition_codes(data: &WeatherData) -> Vec<String> {
    data.current
        .iter()
        .map(|current| current.condition_code.clone())
        .chain(data.forecast.iter().map(|day| day.condition_code.clone()))
        .filter(|code| !code.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll, Waker},
    };

    use allesbuien_core::weather::{CurrentConditions, ForecastDay, WeatherData};
    use tempfile::{TempDir, tempdir};

    use super::{IconCache, IconFuture, IconSource, condition_codes};

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nbody";

    struct CountingSource {
        calls: AtomicUsize,
        body: Vec<u8>,
    }

    impl CountingSource {
        fn serving(body: &[u8]) -> Arc<Self> {
            Arc::new(Self {
                calls: AtomicUsize::new(0),
                body: body.to_vec(),
            })
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl IconSource for CountingSource {
        fn get(&self, _url: &str) -> IconFuture<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Some(self.body.clone()) })
        }
    }

    fn cache(directory: &TempDir, source: Arc<CountingSource>) -> IconCache {
        IconCache::new(directory.path().to_path_buf(), source)
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    #[test]
    fn a_second_launch_reads_the_icon_off_disk_instead_of_the_network() {
        // Given: one warmed directory and a brand-new cache handle over it.
        let directory = tempdir().expect("temp cache");
        let source = CountingSource::serving(PNG);
        let first = block_on(cache(&directory, Arc::clone(&source)).entry("QQ"));

        // When
        let second = block_on(cache(&directory, Arc::clone(&source)).entry("QQ"));

        // Then
        assert_eq!(source.calls(), 1);
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
    fn an_undecodable_download_is_never_cached() {
        // Given
        let directory = tempdir().expect("temp cache");
        let source = CountingSource::serving(b"<html>404 not found</html>");

        // When
        let entry = block_on(cache(&directory, Arc::clone(&source)).entry("A"));

        // Then
        assert!(entry.is_none());
        assert!(!directory.path().join("A.png").exists());
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
    fn warming_a_usable_code_publishes_it_to_the_provider() {
        // Given
        let directory = tempdir().expect("temp cache");
        let source = CountingSource::serving(PNG);

        // When
        block_on(cache(&directory, Arc::clone(&source)).warm(&[String::from("yy")]));

        // Then
        assert_eq!(source.calls(), 1);
        assert!(crate::image_provider::has_icon("YY"));
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
}
