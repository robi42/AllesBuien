//! Buienradar condition codes and their English descriptions.
//!
//! The feed carries no English wording: `weatherdescription` is Dutch only.
//! Every station measurement and forecast day does however reference an icon
//! such as `.../icons/weather/30x30/qq.png`, whose file stem is Buienradar's
//! condition code. A single letter is the day variant and a doubled letter is
//! the night variant of the same condition.
//!
//! Buienradar publishes no official code table. The mapping below is
//! transcribed from `mjj4791/python-buienradar`'s `__BRCONDITIONS`
//! (`buienradar/constants.py`), the de-facto reference that Home Assistant's
//! official Buienradar integration depends on; Home Assistant's own
//! `STATE_CONDITION_CODES` confirms that exactly these twenty-two letters are
//! ever emitted and that `e`, `x`, `y`, and `z` are unused.
//!
//! Lookups are deterministic and total: an unrecognised code falls back to the
//! Dutch feed text and finally to [`UNKNOWN_CONDITION`], never to a panic and
//! never to an empty string.

/// Neutral label used when neither an English mapping nor Dutch text exists.
pub const UNKNOWN_CONDITION: &str = "Unknown";

/// Every condition code Buienradar is documented to emit, with English wording.
const ENGLISH_BY_CODE: &[(char, &str)] = &[
    ('a', "Clear"),
    ('b', "Partly cloudy"),
    ('c', "Heavily clouded"),
    ('d', "Partly cloudy with fog"),
    ('f', "Partly cloudy with light rain"),
    ('g', "Partly cloudy with thunderstorms"),
    ('h', "Partly cloudy with rain"),
    ('i', "Partly cloudy with snow"),
    ('j', "Partly cloudy"),
    ('k', "Partly cloudy with light rain"),
    ('l', "Rain"),
    ('m', "Heavily clouded with light rain"),
    ('n', "Clear with local mist or fog"),
    ('o', "Partly cloudy"),
    ('p', "Cloudy"),
    ('q', "Heavily clouded with rain"),
    ('r', "Partly cloudy"),
    ('s', "Cloudy with thunderstorms"),
    ('t', "Heavy snowfall"),
    ('u', "Partly cloudy with light snow"),
    ('v', "Heavily clouded with light snow"),
    ('w', "Heavily clouded with rain and sleet"),
];

/// Extracts the lowercase condition code from a Buienradar icon URL.
///
/// Accepts both icon sizes and either letter case, tolerates query strings, and
/// returns [`None`] for anything that is not a one or two letter ASCII stem.
#[must_use]
pub fn code_from_icon_url(url: &str) -> Option<String> {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let stem = path
        .rsplit('/')
        .next()?
        .rsplit_once('.')
        .map_or(path.rsplit('/').next()?, |(stem, _)| stem);
    let code = stem.to_ascii_lowercase();
    let mut letters = code.chars();
    let first = letters.next()?;
    if !first.is_ascii_lowercase() {
        return None;
    }
    match letters.next() {
        None => Some(code),
        Some(second) if second == first && letters.next().is_none() => Some(code),
        Some(_) => None,
    }
}

/// Returns the base condition letter for a code, discarding the night doubling.
#[must_use]
pub fn base_letter(code: &str) -> Option<char> {
    let first = code.chars().next()?.to_ascii_lowercase();
    first.is_ascii_lowercase().then_some(first)
}

/// Reports whether a code is the doubled night variant, e.g. `qq`.
#[must_use]
pub fn is_night(code: &str) -> bool {
    let mut letters = code.chars();
    match (letters.next(), letters.next(), letters.next()) {
        (Some(first), Some(second), None) => {
            first.eq_ignore_ascii_case(&second) && first.is_ascii_alphabetic()
        }
        _ => false,
    }
}

/// Looks up the documented English description for a condition code.
#[must_use]
pub fn english_text(code: &str) -> Option<&'static str> {
    let letter = base_letter(code)?;
    ENGLISH_BY_CODE
        .iter()
        .find(|(candidate, _)| *candidate == letter)
        .map(|(_, text)| *text)
}

/// Best available human description for a condition, preferring English.
///
/// Falls back to the Dutch feed text when the code is unmapped, and to
/// [`UNKNOWN_CONDITION`] when neither is usable.
#[must_use]
pub fn describe(code: &str, dutch: &str) -> String {
    if let Some(english) = english_text(code) {
        return english.to_owned();
    }
    let dutch = dutch.trim();
    if dutch.is_empty() {
        UNKNOWN_CONDITION.to_owned()
    } else {
        dutch.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ENGLISH_BY_CODE, UNKNOWN_CONDITION, base_letter, code_from_icon_url, describe,
        english_text, is_night,
    };

    #[test]
    fn day_and_night_icon_urls_yield_their_stems() {
        // Given / When / Then
        assert_eq!(
            code_from_icon_url(
                "https://cdn.buienradar.nl/resources/images/icons/weather/30x30/c.png"
            )
            .as_deref(),
            Some("c")
        );
        assert_eq!(
            code_from_icon_url(
                "https://cdn.buienradar.nl/resources/images/icons/weather/96x96/QQ.png"
            )
            .as_deref(),
            Some("qq")
        );
        assert_eq!(
            code_from_icon_url("https://cdn.buienradar.nl/x/f.png?v=2").as_deref(),
            Some("f")
        );
    }

    #[test]
    fn non_condition_icon_urls_are_rejected_rather_than_guessed() {
        // Given / When / Then
        assert_eq!(code_from_icon_url(""), None);
        assert_eq!(
            code_from_icon_url("https://cdn.example/icons/sunny.png"),
            None
        );
        assert_eq!(code_from_icon_url("https://cdn.example/icons/ab.png"), None);
        assert_eq!(code_from_icon_url("https://cdn.example/icons/12.png"), None);
    }

    #[test]
    fn night_variants_are_detected_and_share_the_day_letter() {
        // Given / When / Then
        assert!(is_night("qq"));
        assert!(!is_night("q"));
        assert!(!is_night("qr"));
        assert!(!is_night("qqq"));
        assert_eq!(base_letter("QQ"), Some('q'));
        assert_eq!(base_letter("q"), Some('q'));
        assert_eq!(base_letter(""), None);
        assert_eq!(base_letter("7"), None);
    }

    #[test]
    fn documented_codes_resolve_to_english_text() {
        // Given / When / Then
        assert_eq!(english_text("a"), Some("Clear"));
        assert_eq!(english_text("c"), Some("Heavily clouded"));
        assert_eq!(english_text("f"), Some("Partly cloudy with light rain"));
        assert_eq!(english_text("qq"), Some("Heavily clouded with rain"));
        assert_eq!(
            english_text("W"),
            Some("Heavily clouded with rain and sleet")
        );
    }

    #[test]
    fn undocumented_codes_have_no_english_mapping() {
        // Given / When / Then
        for code in ["e", "x", "y", "z"] {
            assert_eq!(english_text(code), None, "code {code}");
        }
    }

    #[test]
    fn describe_falls_back_to_dutch_then_to_a_neutral_label() {
        // Given / When / Then
        assert_eq!(
            describe("qq", "Zwaar bewolkt en regen"),
            "Heavily clouded with rain"
        );
        assert_eq!(describe("z", "Zwaar bewolkt"), "Zwaar bewolkt");
        assert_eq!(describe("", "  "), UNKNOWN_CONDITION);
        assert_eq!(describe("", ""), UNKNOWN_CONDITION);
    }

    #[test]
    fn every_table_entry_is_a_unique_lowercase_letter_with_non_empty_text() {
        // Given / When
        let mut letters: Vec<char> = ENGLISH_BY_CODE.iter().map(|(code, _)| *code).collect();
        let count = letters.len();
        letters.sort_unstable();
        letters.dedup();

        // Then
        assert_eq!(letters.len(), count);
        assert_eq!(count, 22);
        for (code, text) in ENGLISH_BY_CODE {
            assert!(code.is_ascii_lowercase(), "code {code}");
            assert!(!text.is_empty(), "code {code}");
        }
    }
}
