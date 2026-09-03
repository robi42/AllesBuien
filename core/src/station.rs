//! The name of the station a reading came from, in this window's language.
//!
//! Buienradar labels every row of `actual.stationmeasurements` in Dutch: the
//! feed publishes `"stationname": "Meetstation Schiphol"`, and `meetstation` is
//! the Dutch measuring station. That spelling is a fact about their payload and
//! is quoted verbatim here, but it is not our copy, and every other visible
//! string in this app is English — `crate::radar_legend` settled the same
//! question for the rainfall unit when it chose `mm/h` over the vendor's
//! `mm/u`. Translating here, at the parse boundary, is what keeps the Dutch
//! form from reaching the rest of the crate at all.

/// The vendor's own spelling, including the separating space.
///
/// Held as one literal so the prefix that is matched and the prefix that is
/// documented cannot drift apart.
const VENDOR_PREFIX: &str = "Meetstation ";

/// Our replacement, carrying the space [`VENDOR_PREFIX`] consumed.
const OUR_PREFIX: &str = "Station ";

/// Rewrites a published station name into the English form the window shows.
///
/// `"Meetstation Schiphol"` becomes `"Station Schiphol"`. The word alone would
/// be a worse answer than either: the row exists to say where the numbers were
/// physically measured, and it sits beneath a header already naming the
/// viewer's location, so a bare `"Schiphol"` would read as a second, competing
/// place rather than as the source of the readings.
///
/// Only a leading [`VENDOR_PREFIX`] is rewritten, never an occurrence further
/// in. Nothing in the feed promises the prefix on every row — app fixtures
/// carry bare names such as `"Amsterdam-West"` — so an unprefixed name is
/// returned unchanged rather than treated as an error.
#[must_use]
pub(crate) fn english_station_name(name: &str) -> String {
    name.strip_prefix(VENDOR_PREFIX)
        .map_or_else(|| name.to_owned(), |place| format!("{OUR_PREFIX}{place}"))
}

#[cfg(test)]
mod tests {
    use super::english_station_name;

    #[test]
    fn the_vendor_dutch_prefix_becomes_the_english_one() {
        // Given / When / Then: the form every real feed row arrives in.
        assert_eq!(
            english_station_name("Meetstation Schiphol"),
            "Station Schiphol"
        );
        assert_eq!(
            english_station_name("Meetstation Hoek van Holland"),
            "Station Hoek van Holland"
        );
    }

    #[test]
    fn a_name_without_the_vendor_prefix_passes_through_untouched() {
        // Given / When / Then: the feed does not promise the prefix on every row.
        assert_eq!(english_station_name("Amsterdam-West"), "Amsterdam-West");
        assert_eq!(english_station_name(""), "");
    }

    #[test]
    fn the_vendor_word_is_only_recognised_at_the_front() {
        // Given / When / Then: a substring replace would mangle both of these.
        assert_eq!(
            english_station_name("Oud Meetstation Ell"),
            "Oud Meetstation Ell"
        );
        assert_eq!(english_station_name("Meetstation"), "Meetstation");
    }
}
