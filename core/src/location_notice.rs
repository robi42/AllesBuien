//! What the window says about the place it is showing, and about its limits.
//!
//! Two facts the viewer cannot otherwise recover. The first is provenance: a
//! resolved location renders identically whether it was chosen or fallen back
//! to, so `Amsterdam-West` read as a decision the viewer had made rather than as
//! the absence of one, and the fix nobody could find was one command away.
//!
//! The second is coverage. This app draws Dutch radar and only Dutch radar, so a
//! location outside [`crate::radar_projection`]'s bounds has no marker and no
//! rain to show — the map paints its base and nothing else, which is exactly
//! what a broken image request looks like. Saying so turns a suspected bug into
//! a stated limit.
//!
//! The wording lives beside the rule that decides it, for the same reason
//! [`crate::status`] holds the failure wording: a caption the viewer reads is a
//! product decision, and one spread across two bridges drifts into two
//! spellings of the same sentence.

use crate::{
    location::{LocationConfig, LocationSource},
    radar_projection::marker_position,
};

/// Captions naming which source answered, shown under the place name.
///
/// The two that answer are prepositional because of where they are read: the
/// caption sits directly beneath the name, so it is read as a predicate of it
/// and has to complete the sentence the name starts. `Amsterdam / Omarchy
/// setting` is two nouns and leaves the relation between them to be guessed;
/// `Amsterdam / Via Omarchy` says the place came from the desktop's setting.
///
/// The fallback deliberately breaks that grammar. Nothing answered, so there is
/// no source to name and no `via` to write; saying `No setting` names the
/// absence, which is both the true state and the one that hints there is
/// something to set. `Default` named the value instead and read like a choice.
pub const ENVIRONMENT_PROVENANCE: &str = "Via env override";
pub const DESKTOP_PROVENANCE: &str = "Via Omarchy";
pub const DEFAULT_PROVENANCE: &str = "No setting";

/// The caption for where this location came from.
#[must_use]
pub const fn provenance(source: LocationSource) -> &'static str {
    match source {
        LocationSource::Environment => ENVIRONMENT_PROVENANCE,
        LocationSource::DesktopSetting => DESKTOP_PROVENANCE,
        LocationSource::BuiltInDefault => DEFAULT_PROVENANCE,
    }
}

/// Warns when the Dutch radar cannot cover this location, and is silent when it can.
#[must_use]
pub fn coverage_notice(location: &LocationConfig) -> Option<String> {
    if marker_position(location.latitude, location.longitude).is_some() {
        return None;
    }
    Some(format!(
        "{} is outside the Dutch radar — the map stays empty",
        location.display_label()
    ))
}

#[cfg(test)]
mod tests {
    use super::{coverage_notice, provenance};
    use crate::location::{LocationConfig, LocationSource};

    fn at(latitude: f64, longitude: f64, label: &str) -> LocationConfig {
        LocationConfig {
            latitude,
            longitude,
            label: String::from(label),
            source: LocationSource::DesktopSetting,
        }
    }

    #[test]
    fn every_source_gets_its_own_caption_and_only_one_reads_as_a_fallback() {
        // Given
        let sources = [
            LocationSource::Environment,
            LocationSource::DesktopSetting,
            LocationSource::BuiltInDefault,
        ];

        // When
        let captions = sources.map(provenance);

        // Then
        assert_eq!(captions, ["Via env override", "Via Omarchy", "No setting"]);
        assert_eq!(
            captions
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            sources.len(),
            "two sources sharing a caption would be indistinguishable on screen"
        );
        assert!(
            !provenance(LocationSource::BuiltInDefault).starts_with("Via "),
            "nothing answered, so the fallback has no source to name"
        );
        assert!(
            provenance(LocationSource::BuiltInDefault).contains("No setting"),
            "the fallback has to read as an absence rather than as a choice"
        );
    }

    #[test]
    fn a_location_the_radar_covers_says_nothing_at_all() {
        // Given: the four corners of the raster and the configured default.
        let covered = [
            at(52.37, 4.84, "Amsterdam-West"),
            at(49.5, 0.0, "the south-west corner"),
            at(54.8, 10.0, "the north-east corner"),
            at(53.22, 6.57, "Groningen"),
        ];

        // When / Then
        for location in covered {
            assert_eq!(
                coverage_notice(&location),
                None,
                "{} is inside the radar",
                location.label
            );
        }
    }

    #[test]
    fn a_location_beyond_the_raster_names_itself_and_says_the_map_stays_empty() {
        // Given: one miss on each edge, plus a coordinate that is not a number.
        let uncovered = [
            at(48.85, 2.35, "Paris"),
            at(51.5, -0.13, "London"),
            at(52.52, 13.4, "Berlin"),
            at(55.7, 12.6, "Copenhagen"),
            at(f64::NAN, 5.0, "Nowhere"),
        ];

        // When / Then
        for location in uncovered {
            let notice = coverage_notice(&location).expect("outside the radar");
            assert!(
                notice.starts_with(&location.label),
                "{notice:?} should name the place"
            );
            assert!(notice.contains("outside the Dutch radar"), "{notice:?}");
            assert!(notice.contains("the map stays empty"), "{notice:?}");
        }
    }

    #[test]
    fn a_nameless_location_borrows_the_placeholder_the_header_shows() {
        // Given
        let nameless = at(48.85, 2.35, "   ");

        // When
        let notice = coverage_notice(&nameless).expect("outside the radar");

        // Then
        assert!(notice.starts_with("Your location is outside"), "{notice:?}");
    }
}
