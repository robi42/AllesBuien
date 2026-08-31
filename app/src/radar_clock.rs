//! Reads Buienradar radar frame stamps on this machine's clock.
//!
//! Radar metadata does not follow the weather feed's convention: it publishes
//! UTC, reporting `2026-08-31T15:50:00` while the Dutch wall clock read 17:50.
//! [`LocalTimestamp`] assumes the Amsterdam-naive convention, so reading a radar
//! stamp through it alone renders every frame two hours behind in summer.
//!
//! Reading such a stamp through [`LocalTimestamp::from_feed`] under-shifts it by
//! exactly Amsterdam's UTC offset, because it resolved a UTC reading as if it
//! were Dutch local time. Adding that offset back — core's UTC-minus-Amsterdam
//! shift, negated — corrects it. Both terms are evaluated at the stamp's own
//! instant, so the pair stays right across the CET/CEST transition, and the
//! timezone database stays in core.

use allesbuien_core::datetime::{LocalTimestamp, clock_shift_minutes_in, shift_clock_label};

/// IANA name of the zone radar metadata publishes its stamps in.
const RADAR_TIMEZONE: &str = "UTC";

/// Projects a UTC radar frame stamp onto this machine's `HH:MM` clock.
///
/// Unparsable input degrades to readable text rather than a blank caption.
#[must_use]
pub fn frame_label(utc_stamp: &str) -> String {
    let read_as_dutch = LocalTimestamp::from_feed(utc_stamp).time_label;
    let amsterdam_offset = -clock_shift_minutes_in(RADAR_TIMEZONE, utc_stamp);
    shift_clock_label(&read_as_dutch, amsterdam_offset)
}
#[cfg(test)]
mod tests {
    use allesbuien_core::datetime::LocalTimestamp;

    use super::frame_label;

    fn minutes_of(label: &str) -> i32 {
        let (hours, minutes) = label.split_once(':').expect("expected a HH:MM label");
        hours.parse::<i32>().expect("hours") * 60 + minutes.parse::<i32>().expect("minutes")
    }

    #[test]
    fn a_utc_stamp_is_never_read_as_a_dutch_wall_clock() {
        // Given: a summer instant, when Amsterdam runs two hours ahead of UTC.
        let stamp = "2026-08-31T15:25:00";

        // When
        let label = frame_label(stamp);

        // Then
        assert_eq!(label.len(), 5, "expected a HH:MM label, got {label}");
        assert_eq!(
            (minutes_of(&label) - minutes_of(&LocalTimestamp::from_feed(stamp).time_label))
                .rem_euclid(24 * 60),
            120,
            "radar stamps were read as an Amsterdam wall clock again"
        );
    }

    #[test]
    fn a_winter_stamp_uses_the_one_hour_cet_offset() {
        // Given / When
        let label = frame_label("2026-01-15T12:00:00");

        // Then
        assert_eq!(
            (minutes_of(&label)
                - minutes_of(&LocalTimestamp::from_feed("2026-01-15T12:00:00").time_label))
            .rem_euclid(24 * 60),
            60
        );
    }

    #[test]
    fn consecutive_frames_stay_five_minutes_apart() {
        // Given / When
        let first = frame_label("2026-08-31T15:25:00");
        let second = frame_label("2026-08-31T15:30:00");

        // Then
        assert_eq!(
            (minutes_of(&second) - minutes_of(&first)).rem_euclid(24 * 60),
            5
        );
    }

    #[test]
    fn a_malformed_stamp_degrades_to_readable_text() {
        // Given / When / Then
        assert_eq!(frame_label("not-a-timestamp"), "not-a");
        assert!(frame_label("").is_empty());
    }
}
