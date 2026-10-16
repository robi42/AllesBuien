//! Timezone-correct interpretation of Buienradar's naive Dutch wall-clock stamps.
//!
//! Buienradar's weather feed publishes every timestamp as naive
//! `Europe/Amsterdam` local time with no UTC offset (`2026-08-30T17:50:00`), and
//! raintext labels are bare `HH:MM` readings of that same clock; radar metadata
//! is UTC instead, and [`crate::radar_timeline`] reads it. Rendering those
//! digits verbatim is only correct while the viewer happens to run in the
//! Dutch zone. This module resolves the feed values in `Europe/Amsterdam` —
//! honouring the CET/CEST transition — and re-projects them into the viewer's
//! own timezone.
//!
//! Every entry point is total: unparsable input degrades to a passthrough of the
//! original text rather than panicking or yielding an empty label.

use jiff::{Unit, Zoned, civil::DateTime, fmt::strtime, tz::TimeZone};

/// Timezone every Buienradar wall-clock value is expressed in.
pub const SOURCE_TIMEZONE: &str = "Europe/Amsterdam";

/// One Buienradar timestamp resolved into the viewer's local timezone.
///
/// `source` always retains the untouched feed text so callers can correlate
/// back to the payload; the remaining fields are display-ready.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalTimestamp {
    /// Naive `Europe/Amsterdam` text exactly as the feed delivered it.
    pub source: String,
    /// `HH:MM` in the viewer's timezone.
    pub time_label: String,
    /// `YYYY-MM-DD` in the viewer's timezone.
    pub date_label: String,
    /// Abbreviated weekday (`Mon`) in the viewer's timezone; empty when unknown.
    pub weekday_label: String,
    /// Offset-qualified instant, e.g. `2026-08-30T17:50:00+02:00`.
    pub rfc3339: String,
}

impl LocalTimestamp {
    /// Resolves a feed timestamp into the timezone this machine is configured for.
    #[must_use]
    pub fn from_feed(naive: &str) -> Self {
        Self::resolve(naive, &TimeZone::system())
    }

    /// Resolves a feed timestamp into an explicitly named IANA timezone.
    ///
    /// Unknown timezone names fall back to the system zone, which keeps the
    /// function total for caller-supplied configuration values.
    #[must_use]
    pub fn from_feed_in(naive: &str, target_timezone: &str) -> Self {
        let target = TimeZone::get(target_timezone).unwrap_or_else(|_| TimeZone::system());
        Self::resolve(naive, &target)
    }

    fn resolve(naive: &str, target: &TimeZone) -> Self {
        zoned_in(naive, target).map_or_else(
            || Self::passthrough(naive),
            |zoned| Self {
                source: naive.trim().to_owned(),
                time_label: label(&zoned, "%H:%M"),
                date_label: label(&zoned, "%Y-%m-%d"),
                weekday_label: label(&zoned, "%a"),
                rfc3339: label(&zoned, "%Y-%m-%dT%H:%M:%S%:z"),
            },
        )
    }

    fn passthrough(naive: &str) -> Self {
        let trimmed = naive.trim();
        let (date, time) = trimmed.split_once('T').unwrap_or((trimmed, trimmed));
        Self {
            source: trimmed.to_owned(),
            time_label: time.get(..5).unwrap_or(time).to_owned(),
            date_label: date.to_owned(),
            weekday_label: String::new(),
            rfc3339: trimmed.to_owned(),
        }
    }
}

/// Minutes to add to a Dutch wall-clock reading to obtain this machine's local reading.
///
/// The value is evaluated against the current instant so the correct CET or
/// CEST offset is applied.
#[must_use]
pub fn clock_shift_minutes() -> i32 {
    let now = Zoned::now().with_time_zone(source_timezone());
    shift_between(&now.datetime(), &TimeZone::system())
}

/// Minutes to add to a Dutch wall-clock reading to obtain the reading in
/// `target_timezone`, evaluated at the naive Dutch instant `at`.
///
/// Exposed so callers — and tests — can pin both the zone and the instant and
/// therefore observe DST transitions deterministically.
#[must_use]
pub fn clock_shift_minutes_in(target_timezone: &str, at: &str) -> i32 {
    let Ok(civil) = at.trim().parse::<DateTime>() else {
        return 0;
    };
    let target = TimeZone::get(target_timezone).unwrap_or_else(|_| TimeZone::system());
    shift_between(&civil, &target)
}

/// Shifts an `HH:MM` label by whole minutes, wrapping around midnight.
///
/// Anything that is not a valid 24-hour label is returned trimmed but otherwise
/// untouched, so malformed feed rows never surface as blanks.
#[must_use]
pub fn shift_clock_label(label: &str, shift_minutes: i32) -> String {
    let Some(minutes) = minutes_of_day(label) else {
        return label.trim().to_owned();
    };
    let total = (minutes + shift_minutes).rem_euclid(24 * 60);
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Reads a bare `HH:MM` clock label as minutes past midnight.
///
/// Surrounding whitespace is tolerated; anything that is not an hour in `0..24`
/// and a minute in `0..60` yields `None`.
pub(crate) fn minutes_of_day(label: &str) -> Option<i32> {
    let (hours, minutes) = label.trim().split_once(':')?;
    let hours = hours.parse::<i32>().ok().filter(|h| (0..24).contains(h))?;
    let minutes = minutes
        .parse::<i32>()
        .ok()
        .filter(|m| (0..60).contains(m))?;
    Some(hours * 60 + minutes)
}

fn source_timezone() -> TimeZone {
    TimeZone::get(SOURCE_TIMEZONE).unwrap_or(TimeZone::UTC)
}

fn zoned_in(naive: &str, target: &TimeZone) -> Option<Zoned> {
    let civil: DateTime = naive.trim().parse().ok()?;
    let zoned = civil.to_zoned(source_timezone()).ok()?;
    Some(zoned.with_time_zone(target.clone()))
}

fn shift_between(dutch: &DateTime, target: &TimeZone) -> i32 {
    let Ok(zoned) = dutch.to_zoned(source_timezone()) else {
        return 0;
    };
    let projected = zoned.with_time_zone(target.clone()).datetime();
    dutch
        .until((Unit::Minute, projected))
        .ok()
        .and_then(|span| i32::try_from(span.get_minutes()).ok())
        .unwrap_or(0)
}

fn label(zoned: &Zoned, format: &str) -> String {
    strtime::format(format, zoned).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        LocalTimestamp, SOURCE_TIMEZONE, clock_shift_minutes, clock_shift_minutes_in,
        minutes_of_day, shift_clock_label,
    };

    #[test]
    fn clock_labels_read_as_minutes_past_midnight_only_when_well_formed() {
        // Given / When / Then
        assert_eq!(minutes_of_day("00:00"), Some(0));
        assert_eq!(minutes_of_day(" 23:59 "), Some(23 * 60 + 59));
        assert_eq!(minutes_of_day("24:00"), None);
        assert_eq!(minutes_of_day("12:60"), None);
        assert_eq!(minutes_of_day("12"), None);
        assert_eq!(minutes_of_day("ab:cd"), None);
    }

    #[test]
    fn summer_stamp_converts_from_cest_to_utc() {
        // Given
        let naive = "2026-08-30T17:50:00";

        // When
        let local = LocalTimestamp::from_feed_in(naive, "UTC");

        // Then
        assert_eq!(local.time_label, "15:50");
        assert_eq!(local.date_label, "2026-08-30");
        assert_eq!(local.rfc3339, "2026-08-30T15:50:00+00:00");
        assert_eq!(local.source, naive);
    }

    #[test]
    fn winter_stamp_converts_from_cet_to_utc() {
        // Given
        let naive = "2026-01-15T12:00:00";

        // When
        let local = LocalTimestamp::from_feed_in(naive, "UTC");

        // Then
        assert_eq!(local.time_label, "11:00");
        assert_eq!(local.rfc3339, "2026-01-15T11:00:00+00:00");
    }

    #[test]
    fn conversion_can_roll_over_to_the_next_calendar_day() {
        // Given
        let naive = "2026-08-30T17:50:00";

        // When
        let local = LocalTimestamp::from_feed_in(naive, "Asia/Tokyo");

        // Then
        assert_eq!(local.time_label, "00:50");
        assert_eq!(local.date_label, "2026-08-31");
        assert_eq!(local.weekday_label, "Mon");
    }

    #[test]
    fn amsterdam_viewer_sees_the_unmodified_wall_clock() {
        // Given / When
        let local = LocalTimestamp::from_feed_in("2026-08-30T17:50:00", SOURCE_TIMEZONE);

        // Then
        assert_eq!(local.time_label, "17:50");
        assert_eq!(local.rfc3339, "2026-08-30T17:50:00+02:00");
    }

    #[test]
    fn unparsable_timestamp_degrades_to_a_passthrough_label() {
        // Given / When
        let local = LocalTimestamp::from_feed_in("not-a-timestamp", "UTC");

        // Then
        assert_eq!(local.source, "not-a-timestamp");
        assert_eq!(local.time_label, "not-a");
        assert_eq!(local.date_label, "not-a-timestamp");
        assert!(local.weekday_label.is_empty());
        assert!(
            !LocalTimestamp::from_feed_in("2026-13-99T99:99:99", "UTC")
                .time_label
                .is_empty()
        );
    }

    #[test]
    fn unknown_target_timezone_falls_back_without_panicking() {
        // Given / When
        let local = LocalTimestamp::from_feed_in("2026-08-30T17:50:00", "Mars/Olympus_Mons");

        // Then
        assert!(!local.time_label.is_empty());
        assert_eq!(local.date_label.len(), 10);
    }

    #[test]
    fn clock_shift_is_dst_aware_for_a_fixed_target_zone() {
        // Given / When
        let summer = clock_shift_minutes_in("UTC", "2026-08-30T17:50:00");
        let winter = clock_shift_minutes_in("UTC", "2026-01-15T12:00:00");

        // Then
        assert_eq!(summer, -120);
        assert_eq!(winter, -60);
    }

    #[test]
    fn clock_shift_is_zero_for_the_source_timezone_and_for_junk_input() {
        // Given / When
        let same_zone = clock_shift_minutes_in(SOURCE_TIMEZONE, "2026-08-30T17:50:00");
        let junk = clock_shift_minutes_in("UTC", "");

        // Then
        assert_eq!(same_zone, 0);
        assert_eq!(junk, 0);
    }

    #[test]
    fn system_clock_shift_stays_within_the_range_of_real_utc_offsets() {
        // Given / When
        let shift = clock_shift_minutes();

        // Then
        assert!((-14 * 60..=14 * 60).contains(&shift), "shift {shift}");
    }

    #[test]
    fn clock_labels_shift_forward_and_wrap_at_midnight() {
        // Given / When / Then
        assert_eq!(shift_clock_label("23:50", 60), "00:50");
        assert_eq!(shift_clock_label("00:10", -60), "23:10");
        assert_eq!(shift_clock_label(" 18:05 ", 0), "18:05");
        assert_eq!(shift_clock_label("09:30", 570), "19:00");
    }

    #[test]
    fn malformed_clock_labels_are_returned_untouched() {
        // Given / When / Then
        assert_eq!(shift_clock_label("soon", 60), "soon");
        assert_eq!(shift_clock_label("aa:bb", 60), "aa:bb");
        assert_eq!(shift_clock_label("25:00", 60), "25:00");
        assert_eq!(shift_clock_label("12:99", 60), "12:99");
    }
}
