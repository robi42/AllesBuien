//! Whether the run Buienradar answered with is recent enough to animate.
//!
//! The sprite window is anchored to the vendor's *run* timestamp rather than to
//! our clock, so asking for history sometimes lands on an older archived run and
//! slides the whole animation into the past. That is intermittent rather than
//! structural, which is why the query alone cannot fix it: `history=2` answered
//! with a seven-minute-old run on 31 August at 20:22 and with a ninety-two
//! minute-old one on 2 September at 15:50, while `history=0` answered that same
//! afternoon with a run two minutes *ahead* of the wall clock. Nothing in the
//! request or the response says which of the two arrived, so the run has to be
//! dated and judged on its own timestamp.
//!
//! Radar metadata publishes those stamps in UTC, unlike the weather feed's naive
//! Amsterdam readings; [`crate::radar_timeline`] owns that reading and the
//! parser for it. Dating a run therefore needs no timezone database — and it
//! needs no clock either, because the caller supplies the instant. That is the
//! whole reason every judgement here is a parameter rather than an ambient
//! reading: a comparison against `Timestamp::now()` inside this module would
//! pass in Amsterdam and fail in the timezone matrix.

use jiff::Timestamp;

use crate::radar_timeline::{CIVIL_OFFSET_HOURS, instant};

/// Minutes a run may trail the present before its history is refused.
///
/// A healthy run trails by roughly five to fifteen minutes, and legitimately
/// leads the clock by a minute or two: the vendor answered `15:35` against a
/// 15:33 wall clock. A poisoned one trailed by 78 minutes at 15:33, and another
/// by 92 at 15:50. Thirty minutes is double the worst healthy reading and under
/// half the mildest poisoned one, so it separates the two populations with
/// margin on both sides and six consecutively skipped publications would still
/// keep their history.
///
/// Erring high is deliberate. Accepting a slightly old run costs a few minutes
/// of slightly old past; refusing a healthy one costs a second HTTP request on
/// every single refresh, forever, for nothing.
pub(crate) const STALE_RUN_THRESHOLD_MINUTES: i64 = 30;

/// How a run's own timestamp compares with an instant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunFreshness {
    /// The run trails by no more than [`STALE_RUN_THRESHOLD_MINUTES`], or leads.
    Current,
    /// The run trails by more than [`STALE_RUN_THRESHOLD_MINUTES`].
    Stale {
        /// Whole minutes the run's timestamp sits behind the supplied instant.
        behind_minutes: i64,
    },
    /// Nothing in the payload dates the run, so it is not condemned on a guess.
    Undecidable,
}

/// Dates a run's timestamp against a caller-supplied instant.
///
/// `run_timestamp` is the metadata's top-level `timestamp`, read as UTC.
/// `vendor_offset_hours` is its `timeOffset`, and it deliberately does not move
/// the run along the timeline: the stamp is already UTC, so applying the offset
/// would misdate every healthy run by the Dutch offset and fire the fallback on
/// every refresh in summer. What the field decides is whether the UTC reading
/// may be trusted at all. `timeOffset` is the vendor stating the Dutch offset
/// for these stamps, and a value no civil zone can declare means this is not the
/// payload that reading was measured against — so the run goes undecided rather
/// than being condemned on a zone nobody can vouch for. An *absent* field is not
/// that signal; it is the documented optional case, and the stamp is UTC whether
/// or not the vendor also volunteers the offset a caption would want.
///
/// A run ahead of `now` is current, not suspicious. The vendor publishes runs a
/// couple of minutes early on purpose, and only lateness is evidence of an
/// archived answer.
#[must_use]
pub(crate) fn run_freshness(
    run_timestamp: &str,
    vendor_offset_hours: Option<i32>,
    now: Timestamp,
) -> RunFreshness {
    let declared_offset_is_civil =
        vendor_offset_hours.is_none_or(|hours| CIVIL_OFFSET_HOURS.contains(&hours));
    if !declared_offset_is_civil {
        return RunFreshness::Undecidable;
    }
    let Some(run) = instant(run_timestamp) else {
        return RunFreshness::Undecidable;
    };
    let behind_minutes = now.duration_since(run).as_mins();
    if behind_minutes > STALE_RUN_THRESHOLD_MINUTES {
        RunFreshness::Stale { behind_minutes }
    } else {
        RunFreshness::Current
    }
}

/// Dates the run described by a raw sprite-metadata body.
///
/// Only the two fields that date the run are parsed; the frames, dimensions, and
/// sprite URLs are [`crate::radar`]'s business and are skipped here so a schema
/// change in them cannot turn into a fabricated staleness verdict. A body this
/// cannot read is undecidable rather than stale: refetching would not make it
/// parseable, and [`crate::radar`] reports the real error a moment later.
#[must_use]
pub fn metadata_freshness(metadata: &[u8], now: Timestamp) -> RunFreshness {
    let Ok(run) = serde_json::from_slice::<RunHeader>(metadata) else {
        return RunFreshness::Undecidable;
    };
    run_freshness(&run.timestamp, run.time_offset, now)
}

#[derive(serde::Deserialize)]
struct RunHeader {
    timestamp: String,
    #[serde(rename = "timeOffset", default)]
    time_offset: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::{RunFreshness, metadata_freshness, run_freshness};
    use crate::test_support::sample_instant;

    #[test]
    fn a_run_five_minutes_behind_the_present_keeps_its_history() {
        // Given: 13:45 UTC is 15:45 Amsterdam, five minutes before the sample.
        // When / Then
        assert_eq!(
            run_freshness("2026-09-02T13:45:00", Some(2), sample_instant()),
            RunFreshness::Current
        );
    }

    #[test]
    fn the_archived_run_the_endpoint_serves_is_refused() {
        // Given: the run history=2 actually answered with, ninety-two minutes
        // behind the sample instant.
        // When
        let freshness = run_freshness("2026-09-02T12:18:00", Some(2), sample_instant());

        // Then
        assert_eq!(freshness, RunFreshness::Stale { behind_minutes: 92 });
    }

    #[test]
    fn a_run_the_vendor_published_slightly_early_is_current() {
        // Given: 13:52 UTC is 15:52 Amsterdam, two minutes after the sample.
        // When / Then
        assert_eq!(
            run_freshness("2026-09-02T13:52:00", Some(2), sample_instant()),
            RunFreshness::Current
        );
    }

    #[test]
    fn the_threshold_admits_thirty_minutes_and_refuses_thirty_one() {
        // Given / When / Then
        assert_eq!(
            run_freshness("2026-09-02T13:20:00", Some(2), sample_instant()),
            RunFreshness::Current
        );
        assert_eq!(
            run_freshness("2026-09-02T13:19:00", Some(2), sample_instant()),
            RunFreshness::Stale { behind_minutes: 31 }
        );
    }

    #[test]
    fn a_run_without_a_declared_offset_is_still_dated() {
        // Given: the stamp is UTC whether or not the vendor volunteers the
        // Dutch offset, so an absent field decides exactly as a present one.
        // When / Then
        assert_eq!(
            run_freshness("2026-09-02T13:45:00", None, sample_instant()),
            RunFreshness::Current
        );
        assert_eq!(
            run_freshness("2026-09-02T12:18:00", None, sample_instant()),
            RunFreshness::Stale { behind_minutes: 92 }
        );
    }

    #[test]
    fn an_offset_no_civil_zone_declares_leaves_the_run_undecided() {
        // Given / When / Then
        for impossible in [99, -99] {
            assert_eq!(
                run_freshness("2026-09-02T12:18:00", Some(impossible), sample_instant()),
                RunFreshness::Undecidable
            );
        }
    }

    #[test]
    fn an_unreadable_run_timestamp_leaves_the_run_undecided() {
        // Given / When / Then
        for unreadable in ["", "not-a-timestamp", "2026-13-99T99:99:99"] {
            assert_eq!(
                run_freshness(unreadable, Some(2), sample_instant()),
                RunFreshness::Undecidable
            );
        }
    }

    #[test]
    fn a_metadata_body_is_judged_on_its_own_two_fields() {
        // Given: the shape the endpoint returns, frames and all.
        let stale = br#"{"timestamp":"2026-09-02T12:18:00","timeOffset":2,
            "times":[{"timestamp":"2026-09-02T12:10:00","url":"fake://frame"}]}"#;
        let current = br#"{"timestamp":"2026-09-02T13:45:00","timeOffset":2,
            "times":[{"timestamp":"2026-09-02T13:45:00","url":"fake://frame"}]}"#;

        // When / Then
        assert_eq!(
            metadata_freshness(stale, sample_instant()),
            RunFreshness::Stale { behind_minutes: 92 }
        );
        assert_eq!(
            metadata_freshness(current, sample_instant()),
            RunFreshness::Current
        );
    }

    #[test]
    fn a_body_that_is_not_this_schema_leaves_the_run_undecided() {
        // Given / When / Then
        for unreadable in [b"not json at all".as_slice(), b"{}", b"{\"timeOffset\":2}"] {
            assert_eq!(
                metadata_freshness(unreadable, sample_instant()),
                RunFreshness::Undecidable
            );
        }
    }
}
