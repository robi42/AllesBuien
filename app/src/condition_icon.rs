//! Buienradar CDN icon URLs for condition codes.
//!
//! The feed publishes a 30x30 icon URL per forecast day and nothing at all for
//! station measurements, so the app builds its own URLs from the condition code
//! that [`allesbuien_core::condition`] already extracted. The 96x96 set renders
//! crisply on a scaled display and is published under UPPERCASE stems; the
//! lowercase stem answers 404 there, while the 30x30 set is lowercase only.
//!
//! QML never reads the CDN URL: [`cdn_url`] is what [`crate::icon_cache`]
//! downloads, and [`display_url`] is the `image://condition/<CODE>` address the
//! provider answers from disk, so a second launch paints without the network.

/// Root of Buienradar's published weather icon set.
const ICON_BASE: &str = "https://cdn.buienradar.nl/resources/images/icons/weather";

/// QML image-provider scheme the condition icons are served under.
const PROVIDER_SCHEME: &str = "image://condition";

/// Builds the 96x96 CDN icon URL for a [`cache_key`].
///
/// The key is already the uppercase stem the 96x96 set is published under, so
/// this only places it; a code the cache cannot key on never reaches here.
#[must_use]
pub(crate) fn cdn_url(key: &str) -> String {
    format!("{ICON_BASE}/96x96/{key}.png")
}

/// Builds the cached-provider URL QML binds to for a Buienradar condition code.
///
/// Falls back to `published` whenever [`cache_key`] refuses the code, so a code
/// the cache cannot key on still renders the URL the feed itself carried.
#[must_use]
pub fn display_url(code: &str, published: &str) -> String {
    cache_key(code).map_or_else(
        || published.trim().to_owned(),
        |key| format!("{PROVIDER_SCHEME}/{key}"),
    )
}

/// The cache key an `image://condition/<KEY>` URL addresses, or [`None`] for
/// any other URL, so the readiness rule never restates the provider scheme.
#[must_use]
pub(crate) fn provider_key(url: &str) -> Option<&str> {
    url.strip_prefix(PROVIDER_SCHEME)?.strip_prefix('/')
}

/// Normalises a condition code into the single filesystem-safe name the cache,
/// the provider URL, and the CDN URL all agree on.
#[must_use]
pub fn cache_key(code: &str) -> Option<String> {
    let code = code.trim();
    if code.is_empty() || !code.chars().all(|letter| letter.is_ascii_alphabetic()) {
        return None;
    }
    Some(code.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{cache_key, cdn_url, display_url, provider_key};

    #[test]
    fn day_code_becomes_an_uppercase_96_pixel_url() {
        // Given
        let key = cache_key("q").expect("a single letter is a usable code");

        // When
        let url = cdn_url(&key);

        // Then
        assert_eq!(
            url,
            "https://cdn.buienradar.nl/resources/images/icons/weather/96x96/Q.png"
        );
    }

    #[test]
    fn night_variant_keeps_its_doubled_letter() {
        // Given
        let key = cache_key("qq").expect("a doubled letter is a usable code");

        // When
        let url = cdn_url(&key);

        // Then
        assert!(url.ends_with("/96x96/QQ.png"), "unexpected url {url}");
    }

    #[test]
    fn a_usable_code_is_displayed_through_the_cached_provider() {
        // Given / When / Then
        assert_eq!(display_url("q", ""), "image://condition/Q");
        assert_eq!(display_url("qq", ""), "image://condition/QQ");
        assert_eq!(display_url(" F ", ""), "image://condition/F");
    }

    #[test]
    fn an_uncacheable_code_still_displays_the_published_url() {
        // Given
        let published = "https://cdn.buienradar.nl/resources/images/icons/weather/30x30/f.png";

        // When / Then
        assert_eq!(display_url("", published), published);
        assert_eq!(display_url("a.png", published), published);
        assert!(display_url("4", " ").is_empty());
    }

    #[test]
    fn the_cache_key_matches_the_cdn_stem_for_every_usable_code() {
        // Given / When / Then
        assert_eq!(cache_key("qq").as_deref(), Some("QQ"));
        assert_eq!(cache_key("Q").as_deref(), Some("Q"));
        assert_eq!(cache_key("../etc/passwd"), None);
        assert_eq!(cache_key(""), None);
    }

    #[test]
    fn only_a_provider_url_names_a_cache_key() {
        // Given / When / Then
        assert_eq!(provider_key(&display_url("qq", "")), Some("QQ"));
        assert_eq!(
            provider_key("https://cdn.buienradar.nl/x/30x30/f.png"),
            None
        );
        assert_eq!(provider_key(""), None);
    }
}
