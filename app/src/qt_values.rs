//! Qt-side value policies the bridges share.
//!
//! QML timers take `int` milliseconds, sizes are `int`s, and text bindings take
//! `QString`, while the Rust side holds `Duration`, `u32`, and `Option<String>`.
//! Each conversion has exactly one policy — saturate rather than wrap, blank
//! rather than absent — and it lives here so the three bridges cannot drift
//! apart on it.

use std::time::Duration;

use cxx_qt_lib::QString;

/// A timer interval as the `int` milliseconds a QML `Timer` reads, saturating at
/// `i32::MAX` rather than wrapping into a negative interval.
pub(crate) fn interval_ms(interval: Duration) -> i32 {
    i32::try_from(interval.as_millis()).unwrap_or(i32::MAX)
}

/// A size or count as the `int` a QML property holds, saturating at `i32::MAX`
/// rather than wrapping into a negative one.
pub(crate) fn qt_int(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// An optional display string as the `QString` QML binds, empty when absent.
pub(crate) fn or_empty(value: Option<String>) -> QString {
    value.map_or_else(QString::default, QString::from)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{interval_ms, or_empty, qt_int};

    #[test]
    fn intervals_convert_to_milliseconds_and_saturate_instead_of_wrapping() {
        // Given / When / Then
        assert_eq!(interval_ms(Duration::from_secs(2)), 2_000);
        assert_eq!(interval_ms(Duration::ZERO), 0);
        assert_eq!(interval_ms(Duration::from_hours(1_000)), i32::MAX);
    }

    #[test]
    fn sizes_convert_to_ints_and_saturate_instead_of_wrapping() {
        // Given / When / Then
        assert_eq!(qt_int(1_512), 1_512);
        assert_eq!(qt_int(u32::MAX), i32::MAX);
    }

    #[test]
    fn an_absent_string_binds_as_empty_text() {
        // Given / When / Then
        assert_eq!(
            String::from(&or_empty(Some(String::from("#1e1e2e")))),
            "#1e1e2e"
        );
        assert!(or_empty(None).is_empty());
    }
}
