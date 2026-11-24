//! The one place a response body is read, and the limit it is read under.
//!
//! Every transport in the app reads its answers through [`read_bounded`]: the
//! feed and the rain expectation in [`crate::weather`], the radar's metadata and
//! frames in [`crate::radar_transport`], and the condition icons in the app's
//! icon cache. Each passes the limit for its own kind of answer, measured and
//! recorded beside it, so an answer far larger than any the vendor has sent is
//! refused instead of filling memory and, for the radar, the disk. A timeout
//! alone bounds how long a body may take, not how large it may grow.
//!
//! The limit is checked twice. A `Content-Length` over it is refused before the
//! body is polled at all, so a declared oversize costs one header. The header is
//! read directly rather than through `Response::content_length`, which reports
//! the body's size hint: the check then rests on what the server declared, not
//! on how reqwest relays it. A header that is no number declares nothing. The
//! body is then read chunk by chunk and refused at the chunk that would take it
//! past the limit, which is what catches an answer with no length, or one
//! longer than its header claims, and dropping the response there ends the
//! download.
//!
//! A limit bounds what is accepted and kept, not the traffic: the refused chunk
//! has already arrived, and a retry asks again.

use reqwest::{Response, header::CONTENT_LENGTH};

/// Why a body was not read.
#[derive(Debug, thiserror::Error)]
pub enum BodyError {
    /// The answer declared, or streamed, more than `limit` bytes.
    #[error("response body exceeds its {limit}-byte limit")]
    TooLarge { limit: usize },
    /// The body failed to arrive, as any interrupted request can.
    #[error("response body could not be read: {0}")]
    Transport(#[from] reqwest::Error),
}

/// Reads `response`'s whole body, refusing one longer than `limit` bytes.
///
/// # Errors
/// Returns [`BodyError::TooLarge`] when the declared length or the streamed
/// body passes `limit`, and [`BodyError::Transport`] when a chunk fails.
pub async fn read_bounded(mut response: Response, limit: usize) -> Result<Vec<u8>, BodyError> {
    if declared_length(&response)
        .is_some_and(|declared| u64::try_from(limit).is_ok_and(|limit| declared > limit))
    {
        return Err(BodyError::TooLarge { limit });
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            return Err(BodyError::TooLarge { limit });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// The length the server declared, or [`None`] when it declared none it could
/// mean: an absent header, or one that is no number.
fn declared_length(response: &Response) -> Option<u64> {
    response
        .headers()
        .get(CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        io,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll},
    };

    use http_body::Frame;
    use reqwest::header::CONTENT_LENGTH;

    use super::{BodyError, read_bounded};
    use crate::test_support::block_on;

    /// One step of a scripted body: a chunk, or a failure in place of one.
    enum Step {
        Chunk(&'static [u8]),
        Fail,
    }

    /// A body of unknown size that serves its steps in order and counts how
    /// many it has been asked for, so a test can see where reading stopped.
    struct ScriptedBody {
        steps: VecDeque<Step>,
        served: Arc<AtomicUsize>,
    }

    impl http_body::Body for ScriptedBody {
        type Data = &'static [u8];
        type Error = io::Error;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            let Some(step) = self.steps.pop_front() else {
                return Poll::Ready(None);
            };
            self.served.fetch_add(1, Ordering::Relaxed);
            Poll::Ready(Some(match step {
                Step::Chunk(chunk) => Ok(Frame::data(chunk)),
                Step::Fail => Err(io::Error::other("connection reset")),
            }))
        }
    }

    /// An answered response over `steps`, with `declared` as its
    /// `Content-Length` header when given, and the count of steps served.
    fn answered(declared: Option<&str>, steps: Vec<Step>) -> (reqwest::Response, Arc<AtomicUsize>) {
        let served = Arc::new(AtomicUsize::new(0));
        let body = ScriptedBody {
            steps: steps.into(),
            served: Arc::clone(&served),
        };
        let mut builder = http::Response::builder();
        if let Some(declared) = declared {
            builder = builder.header(CONTENT_LENGTH, declared);
        }
        let response = builder
            .body(reqwest::Body::wrap(body))
            .expect("an in-memory response");
        (response.into(), served)
    }

    #[test]
    fn a_declared_length_over_the_limit_is_refused_before_the_body_is_read() {
        // Given: a header one byte over, on a body whose own size is unknown.
        let (response, served) = answered(
            Some("11"),
            vec![Step::Chunk(b"0123456789"), Step::Chunk(b"x")],
        );

        // When
        let read = block_on(read_bounded(response, 10));

        // Then
        assert!(
            matches!(read, Err(BodyError::TooLarge { limit: 10 })),
            "{read:?}"
        );
        assert_eq!(served.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn an_undeclared_body_is_refused_at_the_chunk_that_passes_the_limit() {
        // Given: no header, and a second chunk that takes the body past 10.
        let (response, served) = answered(
            None,
            vec![
                Step::Chunk(b"01234"),
                Step::Chunk(b"567890"),
                Step::Chunk(b"never asked for"),
            ],
        );

        // When
        let read = block_on(read_bounded(response, 10));

        // Then
        assert!(
            matches!(read, Err(BodyError::TooLarge { limit: 10 })),
            "{read:?}"
        );
        assert_eq!(served.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_body_of_exactly_the_limit_comes_back_whole_declared_or_not() {
        for declared in [Some("10"), None] {
            // Given
            let (response, _served) =
                answered(declared, vec![Step::Chunk(b"01234"), Step::Chunk(b"56789")]);

            // When
            let read = block_on(read_bounded(response, 10));

            // Then
            assert_eq!(
                read.expect("a body at the limit is accepted"),
                b"0123456789",
                "declared {declared:?}"
            );
        }
    }

    #[test]
    fn a_length_that_is_no_number_leaves_the_stream_to_decide() {
        // Given: the same unusable header on a short body and a long one.
        let (short, _) = answered(Some("lots"), vec![Step::Chunk(b"0123")]);
        let (long, served) = answered(
            Some("lots"),
            vec![Step::Chunk(b"01234"), Step::Chunk(b"567890")],
        );

        // When
        let short = block_on(read_bounded(short, 10));
        let long = block_on(read_bounded(long, 10));

        // Then
        assert_eq!(short.expect("a short body is accepted"), b"0123");
        assert!(
            matches!(long, Err(BodyError::TooLarge { limit: 10 })),
            "{long:?}"
        );
        assert_eq!(served.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_body_that_fails_midway_is_a_transport_failure() {
        // Given
        let (response, _served) = answered(None, vec![Step::Chunk(b"0123"), Step::Fail]);

        // When
        let read = block_on(read_bounded(response, 10));

        // Then
        assert!(matches!(read, Err(BodyError::Transport(_))), "{read:?}");
    }
}
