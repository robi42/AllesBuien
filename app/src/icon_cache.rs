//! Warms payload-referenced Buienradar condition icons into the image provider.
//!
//! QML used to bind straight to `https://cdn.buienradar.nl/...`, so every start
//! paid a fresh round trip and the icons appeared about a second after the text.
//! A payload references at most six of the roughly 22 codes seen over time. Up to
//! four misses are warmed together, matching the radar cache's courteous policy
//! toward the free public CDN. Thereafter the `image://condition/<CODE>` provider
//! serves them locally — the second launch never touches the network.
//!
//! Every failure degrades to "no icon" rather than to an error: a miss, an
//! unreachable CDN, an oversized body, or a damaged file all leave the code
//! unwarmed, and the reserved QML slot stays empty instead of moving the text.

use std::{collections::BTreeSet, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use allesbuien_core::weather::WeatherData;
use futures::{StreamExt, stream};

use crate::{condition_icon, icon_store, image_provider};

// Matches the radar fan-out policy; courtesy to the free CDN matters more than throughput.
const MAX_CONCURRENT_ICONS: usize = 4;

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
        let keys: Vec<String> = codes
            .iter()
            .filter_map(|code| condition_icon::cache_key(code))
            .collect();
        stream::iter(keys)
            .map(|key| async move {
                if !image_provider::has_icon(&key)
                    && let Some(bytes) = self.entry(&key).await
                {
                    image_provider::store_icon(&key, bytes.into());
                }
            })
            .buffer_unordered(MAX_CONCURRENT_ICONS)
            .collect::<()>()
            .await;
    }

    /// Reads one icon off disk, downloading and persisting it on a miss.
    async fn entry(&self, key: &str) -> Option<Vec<u8>> {
        let (directory, source) = (self.directory.as_ref()?, self.source.as_ref()?);
        let path = icon_store::entry_path(directory, key);
        if let Some(cached) = icon_store::read(&path) {
            return Some(cached);
        }
        let downloaded = source.get(&condition_icon::cdn_url(key)).await?;
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
mod tests;
