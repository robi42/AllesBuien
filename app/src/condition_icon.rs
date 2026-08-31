//! Buienradar CDN icon URLs for condition codes.
//!
//! The feed publishes a 30x30 icon URL per forecast day and nothing at all for
//! station measurements, so the app builds its own URLs from the condition code
//! that [`allesbuien_core::condition`] already extracted. The 96x96 set renders
//! crisply on a scaled display and is published under UPPERCASE stems; the
//! lowercase stem answers 404 there, while the 30x30 set is lowercase only.

/// Root of Buienradar's published weather icon set.
const ICON_BASE: &str = "https://cdn.buienradar.nl/resources/images/icons/weather";

/// Builds the 96x96 CDN icon URL for a Buienradar condition code.
///
/// `published` is the URL the feed itself carried and is used whenever the code
/// is missing or not a plain ASCII letter stem. When neither is usable the
/// result is empty, which QML treats as "no icon" rather than a broken image.
#[must_use]
pub fn icon_url(code: &str, published: &str) -> String {
    let code = code.trim();
    if code.is_empty() || !code.chars().all(|letter| letter.is_ascii_alphabetic()) {
        return published.trim().to_owned();
    }
    format!("{ICON_BASE}/96x96/{}.png", code.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::icon_url;

    #[test]
    fn day_code_becomes_an_uppercase_96_pixel_url() {
        // Given / When
        let url = icon_url("q", "https://cdn.buienradar.nl/x/30x30/q.png");

        // Then
        assert_eq!(
            url,
            "https://cdn.buienradar.nl/resources/images/icons/weather/96x96/Q.png"
        );
    }

    #[test]
    fn night_variant_keeps_its_doubled_letter() {
        // Given / When
        let url = icon_url("qq", "");

        // Then
        assert!(url.ends_with("/96x96/QQ.png"), "unexpected url {url}");
    }

    #[test]
    fn missing_or_malformed_code_falls_back_to_the_published_url() {
        // Given
        let published = "https://cdn.buienradar.nl/resources/images/icons/weather/30x30/f.png";

        // When / Then
        assert_eq!(icon_url("", published), published);
        assert_eq!(icon_url("  ", published), published);
        assert_eq!(icon_url("4", published), published);
        assert_eq!(icon_url("a.png", published), published);
    }

    #[test]
    fn absent_code_and_absent_published_url_yield_an_empty_string() {
        // Given / When / Then
        assert!(icon_url("", "  ").is_empty());
    }
}
