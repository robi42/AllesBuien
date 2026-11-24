//! Offline coverage for the Buienradar weather transport: the captured answers
//! it parses, the requests it builds, the clients it sends them on, and the
//! limit each kind of answer is read under.
//!
//! Answers are built in memory and handed to the same per-kind readers the
//! transport calls, so a limit is pinned to its kind of answer without a
//! socket.

use super::{
    MAX_FEED_BYTES, MAX_RAINTEXT_BYTES, ReqwestFeedApi, WeatherError, feed_text, is_https,
    parse_feed, parse_series, raintext_text,
};
use crate::{
    config::{DEFAULT_FEED_URL, DEFAULT_RAINTEXT_URL},
    test_support::block_on,
};

const FEED_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/feed.json"
));
const RAINTEXT_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/raintext.txt"
));

#[test]
fn live_fixtures_parse_offline() {
    // Given / When
    let weather = parse_feed(FEED_FIXTURE, 52.37, 4.84).expect("fixture should parse");
    let rain = parse_series(RAINTEXT_FIXTURE, 0);

    // Then
    assert_eq!(weather.forecast.len(), 5);
    assert_ne!(rain.line, "");
}

#[test]
fn client_accepts_owned_and_borrowed_endpoint_urls() {
    // Given / When
    let client = ReqwestFeedApi::new(DEFAULT_FEED_URL.to_owned(), DEFAULT_RAINTEXT_URL)
        .expect("client should build");

    // Then
    assert_eq!(client.feed_url, DEFAULT_FEED_URL);
    assert_eq!(client.raintext_url, DEFAULT_RAINTEXT_URL);
}

#[test]
fn rain_request_asks_getrr_with_the_coordinates_at_four_places() {
    // Given: the shipped endpoints.
    let api =
        ReqwestFeedApi::new(DEFAULT_FEED_URL, DEFAULT_RAINTEXT_URL).expect("client should build");

    // When
    let url_for = |latitude, longitude| {
        api.raintext_request(latitude, longitude)
            .build()
            .expect("rain request should build")
            .url()
            .as_str()
            .to_owned()
    };

    // Then: the literal, not the constant, is what pins the address the
    // terms ask for, and the second place shows the rounding.
    assert_eq!(
        url_for(52.37, 4.84),
        "https://gps.buienradar.nl/getrr.php?lat=52.3700&lon=4.8400"
    );
    assert_eq!(
        url_for(53.219_38, 6.566_52),
        "https://gps.buienradar.nl/getrr.php?lat=53.2194&lon=6.5665"
    );
}

#[test]
fn rain_request_is_sent_on_a_client_that_refuses_plain_http() {
    // Given: the shipped pair, and a plain-HTTP feed stub beside the shipped
    // rain address, which must not lift the rain request's floor.
    let pairs = [
        (DEFAULT_FEED_URL, DEFAULT_RAINTEXT_URL),
        ("http://127.0.0.1:9", DEFAULT_RAINTEXT_URL),
    ];
    for (feed_url, raintext_url) in pairs {
        let api = ReqwestFeedApi::new(feed_url, raintext_url).expect("client should build");
        let (client, request) = api.raintext_request(52.37, 4.84).build_split();
        request.expect("rain request should build");

        // When: the client the rain request would be sent on is handed a
        // plain-HTTP address. A client without the floor accepts it, and
        // reqwest then panics with "there is no reactor running" as it
        // starts the request's timeout, before anything is dialled: that
        // panic, not an assertion, is what a red run of this test shows.
        let refused = block_on(client.get("http://127.0.0.1:9/getrr.php").send())
            .expect_err("plain HTTP should be refused");

        // Then
        assert!(refused.is_builder(), "{feed_url}: {refused}");
    }
}

#[test]
fn only_an_https_address_holds_its_client_to_https() {
    // Given / When / Then
    assert!(is_https(DEFAULT_RAINTEXT_URL));
    assert!(is_https("HTTPS://gps.buienradar.nl/getrr.php"));
    assert!(!is_https("http://127.0.0.1:9"));
    assert!(!is_https("not a URL"));
}

/// A successful answer of `length` bytes that declares no length, so a reader
/// can only refuse it as it streams.
fn answered(length: usize) -> reqwest::Response {
    http::Response::new(vec![b'7'; length]).into()
}

#[test]
fn each_answer_is_refused_one_byte_past_its_own_limit() {
    // Given / When
    let feed = block_on(feed_text(answered(MAX_FEED_BYTES + 1)));
    let raintext = block_on(raintext_text(answered(MAX_RAINTEXT_BYTES + 1)));

    // Then: each names its own answer and carries its own limit.
    assert!(
        matches!(
            feed,
            Err(WeatherError::BodyTooLarge {
                response: "weather feed",
                limit: MAX_FEED_BYTES
            })
        ),
        "{feed:?}"
    );
    assert!(
        matches!(
            raintext,
            Err(WeatherError::BodyTooLarge {
                response: "raintext",
                limit: MAX_RAINTEXT_BYTES
            })
        ),
        "{raintext:?}"
    );
}

#[test]
fn each_answer_at_its_own_limit_is_read_whole() {
    // Given / When
    let feed = block_on(feed_text(answered(MAX_FEED_BYTES))).expect("a feed at its limit");
    let raintext =
        block_on(raintext_text(answered(MAX_RAINTEXT_BYTES))).expect("a raintext at its limit");

    // Then
    assert_eq!(feed.len(), MAX_FEED_BYTES);
    assert_eq!(raintext.len(), MAX_RAINTEXT_BYTES);
}
