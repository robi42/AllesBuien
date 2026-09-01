//! Renders an error together with everything that caused it.
//!
//! `thiserror`'s `#[error("...: {0}")]` prints only the outermost error. For a
//! transport failure that outer layer is `reqwest`'s "error sending request for
//! url (...)", which names the URL but never says whether DNS failed, the
//! connection was reset, TLS was rejected, or the request timed out. Every
//! distinct fault renders as the same sentence, so the status line cannot tell
//! the viewer — or a maintainer reading a screenshot — what actually broke.

use std::error::Error;

/// Cap on how many `source()` hops are followed.
///
/// Chains are short in practice; the bound only stops a cyclic or pathological
/// implementation from hanging the caller.
const MAX_DEPTH: usize = 8;

/// Formats `error` followed by its `source()` chain, innermost cause last.
///
/// Consecutive duplicates are collapsed: wrapper types commonly re-`Display`
/// their source verbatim, which would otherwise repeat the same sentence.
#[must_use]
pub fn describe(error: &dyn Error) -> String {
    let mut rendered = error.to_string();
    let mut source = error.source();
    let mut depth = 0;

    while let Some(cause) = source {
        let text = cause.to_string();
        if !text.is_empty() && !rendered.ends_with(&text) {
            rendered.push_str(": ");
            rendered.push_str(&text);
        }
        depth += 1;
        if depth == MAX_DEPTH {
            break;
        }
        source = cause.source();
    }

    rendered
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fmt};

    use super::describe;

    #[derive(Debug)]
    struct Layer {
        message: &'static str,
        source: Option<Box<Layer>>,
    }

    impl fmt::Display for Layer {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.message)
        }
    }

    impl Error for Layer {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            self.source
                .as_deref()
                .map(|layer| layer as &(dyn Error + 'static))
        }
    }

    fn layer(message: &'static str, source: Option<Layer>) -> Layer {
        Layer {
            message,
            source: source.map(Box::new),
        }
    }

    #[test]
    fn reports_the_error_alone_when_nothing_caused_it() {
        // Given
        let error = layer("radar request failed", None);

        // When / Then
        assert_eq!(describe(&error), "radar request failed");
    }

    #[test]
    fn appends_each_cause_so_the_root_fault_is_visible() {
        // Given: the shape a reqwest transport failure actually has.
        let error = layer(
            "radar request failed",
            Some(layer(
                "error sending request for url (https://image.buienradar.nl/...)",
                Some(layer("dns error", Some(layer("no record found", None)))),
            )),
        );

        // When
        let described = describe(&error);

        // Then
        assert_eq!(
            described,
            "radar request failed: \
             error sending request for url (https://image.buienradar.nl/...): \
             dns error: no record found"
        );
    }

    #[test]
    fn collapses_a_cause_the_wrapper_already_repeated_verbatim() {
        // Given: `#[error("...: {0}")]` types re-render their own source.
        let error = layer(
            "radar request failed: connection reset",
            Some(layer("connection reset", None)),
        );

        // When / Then
        assert_eq!(describe(&error), "radar request failed: connection reset");
    }

    #[test]
    fn stops_following_a_chain_that_never_ends() {
        // Given: nine nested causes against a bound of eight.
        let mut error = layer("root", None);
        for _ in 0..9 {
            error = layer("wrap", Some(error));
        }

        // When
        let described = describe(&error);

        // Then: bounded, and the runaway tail is dropped rather than hanging.
        assert!(!described.contains("root"));
        assert!(described.starts_with("wrap"));
    }
}
