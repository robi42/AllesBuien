//! Retry policy for weather and radar refreshes.
//!
//! Buienradar's metadata endpoint is fast almost always and occasionally
//! stalls hard: sampled sequentially it answered in 0.05-0.36 s nineteen times
//! out of twenty, took 5.28 s once, and on a separate idle attempt returned
//! nothing at all for over 20 s. The transport allows 15 s, so a single stall
//! used to surface as "Radar unavailable" and stay there until the next
//! five-minute tick, which is what made a cold start look broken while the very
//! next launch worked.
//!
//! Retrying costs almost nothing here: [`crate::radar::RadarClient::refresh`]
//! skips any frame already on disk, so a second attempt re-fetches only the
//! metadata and whatever frames the first attempt did not reach.
//!
//! Deciding *whether* to retry is a pure judgement about the error and belongs
//! here; performing the wait is a side effect and belongs to the app layer.

use std::time::Duration;

use crate::{radar::RadarError, weather::WeatherError};

/// Total attempts, including the first.
///
/// Three bounds the worst case at two waits plus three transport timeouts,
/// while covering the isolated stalls actually observed.
pub const MAX_ATTEMPTS: u32 = 3;

const BASE_DELAY: Duration = Duration::from_millis(400);

/// How soon to refresh again once all attempts have failed.
///
/// The normal cadence is five minutes, which is right for a working feed and
/// far too long for a broken one: a stall during startup would leave "Radar
/// unavailable" on screen for five minutes with nothing else happening. Thirty
/// seconds recovers from a passing outage quickly without hammering a service
/// that is genuinely down.
pub const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(30);

/// Whether `error` is worth another attempt.
///
/// Transport faults and 5xx replies are treated as transient. A 4xx means the
/// request itself is wrong, and malformed metadata, an unusable run timestamp,
/// or a cache failure all reproduce identically on a retry, so those surface
/// immediately instead of stalling the viewer behind pointless waits.
#[must_use]
pub fn is_retryable(error: &RadarError) -> bool {
    match error {
        RadarError::Request(_) => true,
        RadarError::HttpStatus { status } => *status >= 500,
        RadarError::Metadata(_) | RadarError::InvalidRunTimestamp { .. } | RadarError::Cache(_) => {
            false
        }
    }
}

/// Whether a weather `error` is worth another attempt.
///
/// Building the client cannot recover when the controller repeats `fetch` on
/// the same feed, and malformed JSON reproduces from the same vendor payload,
/// so both surface immediately. Feed and raintext request errors retain the
/// status produced by `error_for_status`: a missing status is a request that
/// failed before a reply, and a 5xx is a transient service fault, while a 4xx
/// means the request itself is wrong and must not hold the viewer behind waits.
/// The status split follows the radar precedent and reqwest's contract but is
/// not unit-tested here: constructing that error requires driving reqwest's
/// async client inside Tokio, while core deliberately has no async runtime.
#[must_use]
pub fn is_weather_retryable(error: &WeatherError) -> bool {
    match error {
        WeatherError::FeedRequest(error) | WeatherError::RaintextRequest(error) => {
            error.status().is_none_or(|status| status.is_server_error())
        }
        WeatherError::ClientBuild(_) | WeatherError::FeedDecode(_) => false,
    }
}

/// How long to wait before `attempt`, which is 1-based and counts the first try.
///
/// Doubles per attempt: 400 ms, then 800 ms. Deliberately short — someone is
/// watching a "Loading radar frames" placeholder while this runs.
#[must_use]
pub fn backoff(attempt: u32) -> Duration {
    if attempt <= 1 {
        return Duration::ZERO;
    }
    BASE_DELAY * 2_u32.saturating_pow(attempt.saturating_sub(2))
}

#[cfg(test)]
mod tests {
    use std::{io, time::Duration};

    use super::{MAX_ATTEMPTS, backoff, is_retryable, is_weather_retryable};
    use crate::{radar::RadarError, weather::WeatherError};

    #[test]
    fn retries_a_transport_fault_because_the_endpoint_stalls_intermittently() {
        // Given: the shape a timed-out metadata request arrives as.
        let error = RadarError::HttpStatus { status: 503 };

        // When / Then
        assert!(is_retryable(&error));
    }

    #[test]
    fn gives_up_on_a_client_error_because_the_request_itself_is_wrong() {
        // Given / When / Then
        assert!(!is_retryable(&RadarError::HttpStatus { status: 404 }));
        assert!(!is_retryable(&RadarError::HttpStatus { status: 400 }));
    }

    #[test]
    fn gives_up_on_faults_that_reproduce_identically() {
        // Given
        let malformed = RadarError::InvalidRunTimestamp {
            timestamp: "not-a-timestamp".to_owned(),
        };
        let cache = RadarError::Cache(io::Error::other("disk full"));

        // When / Then
        assert!(!is_retryable(&malformed));
        assert!(!is_retryable(&cache));
    }

    #[test]
    fn retries_a_weather_request_that_received_no_http_status() {
        // Given: the no-response shape used by a failed weather request.
        let error = WeatherError::FeedRequest(endpoint_error());

        // When / Then
        assert!(is_weather_retryable(&error));
    }

    #[test]
    fn gives_up_on_weather_failures_that_repeat_identically() {
        // Given: failures caused before requests start and after a body arrives.
        let client = WeatherError::ClientBuild(endpoint_error());
        let malformed = WeatherError::FeedDecode(
            serde_json::from_str::<u8>("not json").expect_err("fixture must be malformed"),
        );

        // When / Then
        assert!(!is_weather_retryable(&client));
        assert!(!is_weather_retryable(&malformed));
    }

    #[test]
    fn waits_nothing_before_the_first_attempt_then_backs_off() {
        // Given / When / Then
        assert_eq!(backoff(1), Duration::ZERO);
        assert_eq!(backoff(2), Duration::from_millis(400));
        assert_eq!(backoff(3), Duration::from_millis(800));
    }

    #[test]
    fn recovers_far_sooner_than_the_normal_cadence_after_a_failure() {
        // Given: the configured cadence for a healthy feed.
        let normal = Duration::from_mins(5);

        // When / Then
        assert!(super::RETRY_AFTER_FAILURE < normal);
        assert!(super::RETRY_AFTER_FAILURE >= Duration::from_secs(15));
    }

    #[test]
    fn bounds_the_total_wait_so_a_stalled_endpoint_cannot_hang_the_view() {
        // Given / When
        let total: Duration = (1..=MAX_ATTEMPTS).map(backoff).sum();

        // Then
        assert_eq!(total, Duration::from_millis(1200));
    }

    fn endpoint_error() -> reqwest::Error {
        reqwest::Client::new()
            .get("not a URL")
            .build()
            .expect_err("the fixture URL must be invalid")
    }
}
