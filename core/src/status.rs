//! Human-facing wording for the inline status lines.
//!
//! What a failure *is* and what a viewer should be *told* are different things.
//! The technical rendering — reqwest's URL, the DNS or TLS cause, the JSON
//! offset — belongs in the log, where a maintainer can act on it. On screen it
//! is noise: a weather panel that prints
//! `error sending request for url (https://image.buienradar.nl/2.0/metadata/...)`
//! across three wrapped lines has told the viewer nothing they can use and
//! buried the one fact that matters, which is whether the problem is theirs.
//!
//! So each message names the subject, says whether anything is still on screen,
//! and points at the likely culprit — the viewer's own connection versus the
//! upstream service — without ever quoting a URL, host, header, or file path.
//!
//! The placeholders below belong here for the same reason the failure wording
//! does: a caption the viewer reads is a product decision, and one shown before
//! the first refresh has to agree word for word with the one shown after a
//! failure. Spread across the bridges, the same sentence drifted into two
//! spellings in two files.

use crate::{radar::RadarError, weather::WeatherError};

/// Stands in for a single reading the station did not publish.
pub const UNAVAILABLE: &str = "Unavailable";
/// Stands in for the one reading that is set as a headline rather than a chip.
///
/// The same word cannot serve both ranks. The metric chips are captions with a
/// column each and `Unavailable` fits them, but the temperature is rendered at
/// 38 pt across a 380 px column, where the word runs past the end and elides to
/// `Unavaila…` — which reads as a truncated measurement rather than an absent
/// one, and is the worst of the three possible impressions: it looks like the
/// app has a value it is too small to show. A dash is the shortest mark that
/// says "no number here" without pretending to be a number, it cannot elide at
/// any size this window can reach, and it leaves the sentence under it and the
/// status line below to say *why* — which the word was never doing either.
pub const NO_READING: &str = "—";
/// Inline status lines shown before the first successful refresh.
pub const CONDITIONS_UNAVAILABLE: &str = "Current conditions unavailable";
pub const WEATHER_UNAVAILABLE: &str = "Weather unavailable";
pub const RADAR_UNAVAILABLE: &str = "Radar unavailable";
pub const RAIN_UNAVAILABLE: &str = "Rain forecast unavailable";
/// Shown while the first radar run is still downloading its frames.
pub const RADAR_LOADING: &str = "Loading radar frames";

const CONNECTION_HINT: &str = "check your connection";
const SERVICE_HINT: &str = "Buienradar is not responding";
const DATA_HINT: &str = "unexpected data from Buienradar";

fn compose(subject: &str, hint: &str, stale: bool) -> String {
    if stale {
        format!("{subject} may be out of date — {hint}")
    } else {
        format!("{subject} unavailable — {hint}")
    }
}

/// Wording for a failed radar refresh.
///
/// `stale` says whether a previous run is still being shown, which changes the
/// claim from "unavailable" to "may be out of date" — the frames on screen are
/// real, just older than they look.
#[must_use]
pub fn radar_message(error: &RadarError, stale: bool) -> String {
    let hint = match error {
        RadarError::Request(_) => CONNECTION_HINT,
        RadarError::HttpStatus { .. } => SERVICE_HINT,
        RadarError::Metadata(_) | RadarError::InvalidRunTimestamp { .. } => DATA_HINT,
        RadarError::Cache(_) => "could not write the radar cache",
    };
    compose("Radar", hint, stale)
}

/// Wording for a failed weather refresh.
#[must_use]
pub fn weather_message(error: &WeatherError, stale: bool) -> String {
    let hint = match error {
        WeatherError::ClientBuild(_)
        | WeatherError::FeedRequest(_)
        | WeatherError::RaintextRequest(_) => CONNECTION_HINT,
        WeatherError::FeedDecode(_) => DATA_HINT,
    };
    compose("Weather", hint, stale)
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{radar_message, weather_message};
    use crate::{radar::RadarError, weather::WeatherError};

    fn radar_samples() -> Vec<RadarError> {
        vec![
            RadarError::HttpStatus { status: 502 },
            RadarError::InvalidRunTimestamp {
                timestamp: "https://image.buienradar.nl/nonsense".to_owned(),
            },
            RadarError::Cache(io::Error::other(
                "/home/someone/.local/state/allesbuien denied",
            )),
        ]
    }

    fn weather_samples() -> Vec<WeatherError> {
        vec![
            WeatherError::ClientBuild(endpoint_error()),
            WeatherError::FeedRequest(endpoint_error()),
            WeatherError::RaintextRequest(endpoint_error()),
            WeatherError::FeedDecode(decode_error()),
        ]
    }

    #[test]
    fn never_leaks_a_url_host_or_path_to_the_viewer() {
        // Given: variants whose own Display would quote internals verbatim,
        // asserted to still do so, because a harmless sample proves nothing.
        let radar = radar_samples();
        let weather = weather_samples();
        assert!(
            weather
                .iter()
                .any(|error| error.to_string().contains("buienradar.nl")),
            "a transport sample has to carry the address the wording must not"
        );
        let messages = radar
            .iter()
            .flat_map(|error| [false, true].map(|stale| radar_message(error, stale)))
            .chain(
                weather
                    .iter()
                    .flat_map(|error| [false, true].map(|stale| weather_message(error, stale))),
            );

        // When / Then
        for message in messages {
            for leak in ["http", "://", "buienradar.nl", "/home/", ".local"] {
                assert!(
                    !message.to_ascii_lowercase().contains(leak),
                    "message {message:?} leaked {leak:?}"
                );
            }
        }
    }

    #[test]
    fn blames_the_connection_for_transport_faults_and_the_service_for_5xx() {
        // Given / When
        let service = radar_message(&RadarError::HttpStatus { status: 502 }, false);
        let data = radar_message(
            &RadarError::InvalidRunTimestamp {
                timestamp: "bad".to_owned(),
            },
            false,
        );

        // Then
        assert!(service.contains("not responding"), "{service}");
        assert!(data.contains("unexpected data"), "{data}");
    }

    #[test]
    fn softens_the_claim_when_older_frames_are_still_on_screen() {
        // Given
        let error = RadarError::HttpStatus { status: 502 };

        // When
        let fresh = radar_message(&error, false);
        let stale = radar_message(&error, true);

        // Then
        assert!(fresh.starts_with("Radar unavailable"), "{fresh}");
        assert!(stale.starts_with("Radar may be out of date"), "{stale}");
    }

    #[test]
    fn names_its_own_subject_so_the_two_panels_stay_distinguishable() {
        // Given / When
        let radar = radar_message(&RadarError::HttpStatus { status: 500 }, false);
        let weather = weather_message(&WeatherError::FeedDecode(decode_error()), false);

        // Then
        assert!(radar.starts_with("Radar"), "{radar}");
        assert!(weather.starts_with("Weather"), "{weather}");
    }

    fn decode_error() -> serde_json::Error {
        serde_json::from_str::<u8>("nope").expect_err("invalid json")
    }

    /// A reqwest error carrying the endpoint it failed on, as a live one does.
    ///
    /// Built offline through the hostless-scheme rejection rather than by
    /// dialling anything: what this fixture has to reproduce is the URL in the
    /// error's `Display`, and a unit test that needs a socket to reproduce it
    /// would be a slower, flakier way to assert the same wording.
    fn endpoint_error() -> reqwest::Error {
        reqwest::Client::new()
            .get("data:text/plain,https://data.buienradar.nl/2.0/feed/json")
            .build()
            .expect_err("a scheme carrying no host cannot become a request")
    }
}
