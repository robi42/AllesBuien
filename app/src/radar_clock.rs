//! Reads Buienradar radar frame stamps on this machine's clock.
//!
//! Radar metadata does not follow the weather feed's convention: it publishes
//! UTC, reporting `2026-08-31T15:50:00` while the Dutch wall clock read 17:50.
//! [`LocalTimestamp`] assumes the Amsterdam-naive convention, so reading a radar
//! stamp through it alone renders every frame two hours behind in summer.
//!
//! Reading such a stamp through [`LocalTimestamp::from_feed`] under-shifts it by
//! exactly Amsterdam's UTC offset, because it resolved a UTC reading as if it
//! were Dutch local time. Adding that offset back corrects it.
//!
//! That offset is preferably Buienradar's own: the metadata response carries a
//! top-level `timeOffset` (measured `2` alongside a `18:40` UTC run at a 20:40
//! Dutch wall clock), which is the vendor stating the offset itself. Deriving it
//! instead makes the label depend on the host's tzdb agreeing with the vendor's,
//! which already broke once between an Amsterdam host and a UTC container. The
//! derivation stays as the fallback for runs whose metadata omits the field, and
//! it is evaluated at the stamp's own instant so it survives the CET/CEST
//! transition without a second tzdb in the app layer.

use allesbuien_core::{
    datetime::{LocalTimestamp, clock_shift_minutes_in, shift_clock_label},
    radar_timeline::CIVIL_OFFSET_HOURS,
};

/// IANA name of the zone radar metadata publishes its stamps in.
const RADAR_TIMEZONE: &str = "UTC";

/// Projects a UTC radar frame stamp onto this machine's `HH:MM` clock.
///
/// `vendor_offset_hours` is Buienradar's published `timeOffset` for the run.
/// Unparsable input degrades to readable text rather than a blank caption.
#[must_use]
pub fn frame_label(utc_stamp: &str, vendor_offset_hours: Option<i32>) -> String {
    let read_as_dutch = LocalTimestamp::from_feed(utc_stamp).time_label;
    let source_offset = source_offset_minutes(utc_stamp, vendor_offset_hours);
    shift_clock_label(&read_as_dutch, source_offset)
}

fn source_offset_minutes(utc_stamp: &str, vendor_offset_hours: Option<i32>) -> i32 {
    vendor_offset_hours
        .filter(|hours| CIVIL_OFFSET_HOURS.contains(hours))
        .map_or_else(
            || -clock_shift_minutes_in(RADAR_TIMEZONE, utc_stamp),
            |hours| hours * 60,
        )
}

/// Minutes past midnight of a `HH:MM` caption, for tests that pin a shift
/// rather than digits so they hold in every host timezone.
#[cfg(test)]
pub(crate) fn minutes_of(label: &str) -> i32 {
    let (hours, minutes) = label.split_once(':').expect("expected a HH:MM label");
    hours.parse::<i32>().expect("hours") * 60 + minutes.parse::<i32>().expect("minutes")
}

#[cfg(test)]
mod tests {
    use allesbuien_core::datetime::LocalTimestamp;

    use super::{frame_label, minutes_of};

    /// Minutes the label was shifted past the same stamp read as Dutch wall time.
    ///
    /// Both readings share this host's timezone term, so their difference is the
    /// applied source offset on every machine — the only assertion about
    /// `frame_label` that is stable in CI's UTC container.
    fn applied_offset(stamp: &str, vendor_offset_hours: Option<i32>) -> i32 {
        let label = frame_label(stamp, vendor_offset_hours);
        assert_eq!(label.len(), 5, "expected a HH:MM label, got {label}");
        (minutes_of(&label) - minutes_of(&LocalTimestamp::from_feed(stamp).time_label))
            .rem_euclid(24 * 60)
    }

    #[test]
    fn a_utc_stamp_is_never_read_as_a_dutch_wall_clock() {
        // Given: a summer instant, when Amsterdam runs two hours ahead of UTC.
        let stamp = "2026-08-31T15:25:00";

        // When / Then
        assert_eq!(
            applied_offset(stamp, None),
            120,
            "radar stamps were read as an Amsterdam wall clock again"
        );
    }

    #[test]
    fn a_winter_stamp_uses_the_one_hour_cet_offset() {
        // Given / When / Then
        assert_eq!(applied_offset("2026-01-15T12:00:00", None), 60);
    }

    #[test]
    fn the_vendor_offset_wins_over_the_derived_one() {
        // Given: a winter stamp the derivation would place one hour ahead of UTC,
        // labelled by a vendor that declares two.
        let stamp = "2026-01-15T12:00:00";

        // When / Then
        assert_eq!(applied_offset(stamp, Some(2)), 120);
        assert_eq!(applied_offset(stamp, Some(-3)), 24 * 60 - 180);
        assert_eq!(applied_offset(stamp, Some(0)), 0);
    }

    #[test]
    fn the_measured_vendor_offset_agrees_with_the_summer_derivation() {
        // Given / When / Then
        assert_eq!(applied_offset("2026-08-31T18:40:00", Some(2)), 120);
    }

    #[test]
    fn an_impossible_vendor_offset_falls_back_to_the_derivation() {
        // Given / When / Then
        assert_eq!(applied_offset("2026-01-15T12:00:00", Some(99)), 60);
        assert_eq!(applied_offset("2026-01-15T12:00:00", Some(-99)), 60);
    }

    #[test]
    fn consecutive_frames_stay_five_minutes_apart() {
        // Given / When
        let first = frame_label("2026-08-31T15:25:00", Some(2));
        let second = frame_label("2026-08-31T15:30:00", Some(2));

        // Then
        assert_eq!(
            (minutes_of(&second) - minutes_of(&first)).rem_euclid(24 * 60),
            5
        );
    }

    #[test]
    fn a_malformed_stamp_degrades_to_readable_text() {
        // Given / When / Then
        assert_eq!(frame_label("not-a-timestamp", Some(2)), "not-a");
        assert_eq!(frame_label("not-a-timestamp", None), "not-a");
        assert_eq!(frame_label("", Some(2)), "");
    }
}
