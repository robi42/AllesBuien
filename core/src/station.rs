//! The place a reading was measured at, taken from the vendor's station name.
//!
//! Buienradar labels every row of `actual.stationmeasurements` in Dutch: the
//! feed publishes `"stationname": "Meetstation Schiphol"`, and `meetstation` is
//! the Dutch measuring station. That spelling is a fact about their payload and
//! is quoted verbatim here, but it is not our copy, and every other visible
//! string in this app is English — `crate::radar_legend` settled the same
//! question for the rainfall unit when it chose `mm/h` over the vendor's
//! `mm/u`. Answering the question here, at the parse boundary, is what keeps
//! the Dutch form from reaching the rest of the crate at all.
//!
//! The answer used to be `"Station Schiphol"`, and this module argued at length
//! that the bare place would be worse. That argument was sound for the layout
//! it was written against and is now void, because the layout changed. The
//! station then sat on a FULL-WIDTH row of its own directly beneath the hero
//! reading, level with nothing, where `SCHIPHOL` alone really would have read
//! as a second place competing with the one in the header. It now sits in the
//! header's right corner as a caption PAIR stacked over the time it read,
//! opposite the location it qualifies. Three things there say what the word was
//! carrying: the position, which is the corner captions occupy and headlines do
//! not; the pairing with a clock, which nothing but an instrument reading is;
//! and the caption treatment itself, 9 pt tracked caps at the emphasis floor,
//! which is how this window spells "field value" everywhere else. A word that
//! restates three signals is not disambiguating anything — it is spending 9 pt
//! of a corner that has to hold `HOEK VAN HOLLAND`, a real station name, inside
//! half a header. So the vendor's prefix is now dropped rather than replaced.
//!
//! Kept as its own module after that reversal shrank it to three lines of code.
//! Its only caller, `crate::weather_feed`, stands at 226 of the 250 pure lines
//! `bin/test` allows, so folding this in would clear the budget by luck rather
//! than by cohesion — and would file a copy decision, its reversal, and its
//! tests inside a payload parser. The seam is still one responsibility with a
//! name, which is the thing `AGENTS.md` asks a module to be.

/// The vendor's own spelling, including the separating space.
///
/// Held as one literal so the prefix that is matched and the prefix that is
/// documented cannot drift apart.
const VENDOR_PREFIX: &str = "Meetstation ";

/// Reduces a published station name to the place the instrument stands at.
///
/// `"Meetstation Schiphol"` becomes `"Schiphol"`. Only a leading
/// [`VENDOR_PREFIX`] is dropped, never an occurrence further in. Nothing in the
/// feed promises the prefix on every row — app fixtures carry bare names such
/// as `"Amsterdam-West"` — so an unprefixed name is returned unchanged rather
/// than treated as an error.
#[must_use]
pub(crate) fn station_place_name(name: &str) -> String {
    name.strip_prefix(VENDOR_PREFIX).unwrap_or(name).to_owned()
}

#[cfg(test)]
mod tests {
    use super::station_place_name;

    #[test]
    fn the_vendor_dutch_prefix_is_dropped() {
        // Given / When / Then: the form every real feed row arrives in.
        assert_eq!(station_place_name("Meetstation Schiphol"), "Schiphol");
        assert_eq!(
            station_place_name("Meetstation Hoek van Holland"),
            "Hoek van Holland"
        );
    }

    #[test]
    fn a_name_without_the_vendor_prefix_passes_through_untouched() {
        // Given / When / Then: the feed does not promise the prefix on every row.
        assert_eq!(station_place_name("Amsterdam-West"), "Amsterdam-West");
        assert_eq!(station_place_name(""), "");
    }

    #[test]
    fn the_vendor_word_is_only_recognised_at_the_front() {
        // Given / When / Then: a substring replace would mangle both of these.
        assert_eq!(
            station_place_name("Oud Meetstation Ell"),
            "Oud Meetstation Ell"
        );
        assert_eq!(station_place_name("Meetstation"), "Meetstation");
    }
}
