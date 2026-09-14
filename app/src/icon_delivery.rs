//! How one weather refresh brings its condition icons on screen.
//!
//! The feed's answer is never held back for an icon. What the disk cache already
//! holds is published first, so a cached launch paints text and icons in one
//! pass; the text follows at once, with every icon the provider cannot serve yet
//! withheld as an empty URL; each download then re-publishes the icons as soon as
//! it lands, so no icon waits for another. Withholding is what makes the
//! re-publish a change QML reacts to: a URL republished as the same string
//! emits nothing, and a hero icon that failed once would stay blank.

use std::pin::Pin;

use allesbuien_core::weather::{WeatherData, WeatherError};
use cxx_qt::CxxQtThread;

use crate::{
    condition_icon,
    icon_cache::{self, IconCache},
    image_provider,
    weather_controller::ffi,
};

/// Publishes one refresh's `result` onto the window, then its missing icons.
pub(crate) async fn publish(
    result: Result<WeatherData, WeatherError>,
    icons: &IconCache,
    qt_thread: &CxxQtThread<ffi::WeatherController>,
) {
    let codes = result
        .as_ref()
        .map(icon_cache::condition_codes)
        .unwrap_or_default();
    icons.restore(&codes);
    let pending = awaits_download(&codes);
    queue_update(qt_thread, move |mut controller| {
        controller.as_mut().apply_result(result);
        controller.set_icons_loading(pending);
    });
    if !pending {
        return;
    }
    icons
        .warm(&codes, &|| {
            queue_update(qt_thread, ffi::WeatherController::apply_icons);
        })
        .await;
    // A refresh that started mid-download may still be downloading when this
    // one clears the flag, so its skeletons can go a little early. Its icons
    // still arrive through `apply_icons`, the overlap is bounded by the
    // per-request timeout, and refreshes are minutes apart, so the flag is not
    // worth a counter.
    queue_update(qt_thread, |controller| controller.set_icons_loading(false));
}

impl ffi::WeatherController {
    /// Re-publishes every icon URL from retained state, now that the provider
    /// may serve one it could not when the text was published.
    ///
    /// Deriving from what is retained rather than from a captured payload is
    /// what lets refreshes interleave: a late call can never restore older data.
    fn apply_icons(mut self: Pin<&mut Self>) {
        let hero = image_provider::published_url(&self.hero_icon_url);
        self.as_mut().set_condition_icon_url(hero.into());
        self.forecast_icons_changed();
    }
}

/// Whether any of `codes` names an icon the provider cannot serve yet.
fn awaits_download(codes: &[String]) -> bool {
    codes.iter().any(|code| {
        condition_icon::cache_key(code).is_some_and(|key| !image_provider::has_icon(&key))
    })
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
    use super::awaits_download;
    use crate::image_provider::{FrameBytes, store_icon};

    #[test]
    fn only_a_usable_code_the_provider_lacks_awaits_a_download() {
        // Given: a key no other test in this process stores.
        let codes = [String::from("rf"), String::from("../etc/passwd")];
        let before = awaits_download(&codes);

        // When
        store_icon("RF", FrameBytes::from(b"icon".to_vec()));

        // Then
        assert!(before);
        assert!(!awaits_download(&codes));
        assert!(!awaits_download(&[String::new()]));
    }
}
