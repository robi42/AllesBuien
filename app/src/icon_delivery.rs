//! How one weather refresh brings its condition icons on screen.
//!
//! The feed's answer is never held back for an icon. What the disk cache already
//! holds is published first, so a cached launch paints text and icons in one
//! pass; the text follows at once, with every icon the provider cannot serve yet
//! withheld as an empty URL; each download then re-publishes the icons as soon as
//! it lands, so no icon waits for another. Withholding is what makes the
//! re-publish a change QML reacts to: a URL republished as the same string
//! emits nothing, and a hero icon that failed once would stay blank.
//!
//! Refreshes overlap — a poll that finds a new location refreshes while the
//! recurring one is still in flight — so every update this module queues is
//! tagged with the refresh it belongs to and dropped unless the window is still
//! showing that refresh. Without the tag, a slow fetch whose answer arrives
//! after a later one's would put its own older payload back on screen, and its
//! icon updates would then count against it.
//!
//! Whether a slot is still loading is answered per icon key, not per refresh.
//! A skeleton belongs to the one download it stands for: a slot whose own
//! download has failed goes empty even while another icon is still on its way,
//! and a refresh that finishes early cannot clear a slot a later refresh is
//! still filling. [`PendingIcons`] counts each key once per refresh that awaits
//! it, which is what makes both hold.

use std::{collections::BTreeMap, pin::Pin};

use allesbuien_core::weather::{WeatherData, WeatherError};
use cxx_qt::{CxxQtThread, CxxQtType};

use crate::{
    condition_icon,
    icon_cache::{self, IconCache},
    image_provider,
    weather_controller::ffi,
};

/// Icon keys whose download has started and not yet settled, with how many
/// refreshes are waiting on each.
#[derive(Debug, Default)]
pub(crate) struct PendingIcons(BTreeMap<String, usize>);

impl PendingIcons {
    /// Counts one more refresh waiting on each of `keys`.
    fn add(&mut self, keys: &[String]) {
        for key in keys {
            *self.0.entry(key.clone()).or_default() += 1;
        }
    }

    /// Counts one refresh's attempt at `key` as over.
    fn settle(&mut self, key: &str) {
        if let Some(waiting) = self.0.get_mut(key) {
            *waiting -= 1;
            if *waiting == 0 {
                self.0.remove(key);
            }
        }
    }

    /// Whether the icon at `url` is still downloading for some refresh.
    #[must_use]
    pub(crate) fn awaits(&self, url: &str) -> bool {
        condition_icon::provider_key(url).is_some_and(|key| self.0.contains_key(key))
    }
}

/// Publishes one refresh's `result` onto the window, then its missing icons.
///
/// Every update is dropped unless `generation` is still the refresh the window
/// is showing, so an overtaken fetch neither restores its payload nor marks any
/// slot as loading for icons nobody is waiting for.
pub(crate) async fn publish(
    result: Result<WeatherData, WeatherError>,
    icons: &IconCache,
    qt_thread: &CxxQtThread<ffi::WeatherController>,
    generation: u64,
) {
    let codes = result
        .as_ref()
        .map(icon_cache::condition_codes)
        .unwrap_or_default();
    icons.restore(&codes);
    let keys = icon_cache::missing_keys(&codes);
    let awaited = keys.clone();
    queue_update(qt_thread, move |mut controller| {
        if controller.refresh_generation != generation {
            return;
        }
        // Counted before the rows are replaced, so they are first read loading.
        controller.as_mut().rust_mut().pending_icons.add(&awaited);
        controller.as_mut().apply_result(result);
        controller.apply_icons();
    });
    if keys.is_empty() {
        return;
    }
    icons
        .warm(&keys, &|key| {
            let key = key.to_owned();
            queue_update(qt_thread, move |mut controller| {
                if controller.refresh_generation != generation {
                    return;
                }
                controller.as_mut().rust_mut().pending_icons.settle(&key);
                controller.apply_icons();
            });
        })
        .await;
}

impl ffi::WeatherController {
    /// Re-publishes every icon URL and loading state from retained state, now
    /// that the provider may serve one it could not when the text was published.
    ///
    /// Deriving from what is retained rather than from a captured payload is
    /// what lets refreshes interleave: a late call can never restore older data.
    fn apply_icons(mut self: Pin<&mut Self>) {
        let hero = image_provider::published_url(&self.hero_icon_url);
        let hero_loading = self.pending_icons.awaits(&self.hero_icon_url);
        self.as_mut().set_condition_icon_url(hero.into());
        self.as_mut().set_hero_icon_loading(hero_loading);
        self.forecast_icons_changed();
    }
}

/// Queues `update` onto the Qt thread, logging a window that is already gone.
fn queue_update(
    qt_thread: &CxxQtThread<ffi::WeatherController>,
    update: impl FnOnce(Pin<&mut ffi::WeatherController>) + Send + 'static,
) {
    if let Err(error) = qt_thread.queue(update) {
        eprintln!("allesbuien: weather update never reached the window: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::PendingIcons;

    #[test]
    fn a_key_stays_pending_until_every_refresh_waiting_on_it_settles() {
        // Given: two refreshes both waiting on `F`, one of them also on `QQ`.
        let mut pending = PendingIcons::default();
        pending.add(&[String::from("F"), String::from("QQ")]);
        pending.add(&[String::from("F")]);

        // When: the first refresh's attempts are over.
        pending.settle("F");
        pending.settle("QQ");

        // Then: only the key the second refresh still waits on is loading.
        assert!(pending.awaits("image://condition/F"));
        assert!(!pending.awaits("image://condition/QQ"));
        pending.settle("F");
        assert!(!pending.awaits("image://condition/F"));
    }

    #[test]
    fn only_a_provider_url_can_be_pending_and_a_stray_settle_is_harmless() {
        // Given
        let mut pending = PendingIcons::default();
        pending.add(&[String::from("F")]);

        // When
        pending.settle("A");

        // Then
        assert!(pending.awaits("image://condition/F"));
        assert!(!pending.awaits("https://cdn.buienradar.nl/x/30x30/f.png"));
        assert!(!pending.awaits(""));
    }
}
