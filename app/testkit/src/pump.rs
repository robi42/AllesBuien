//! Waiting for a Qt-driven condition without hardcoding an iteration count, and
//! driving a future that needs no runtime at all.
//!
//! The harnesses used to spin `for _ in 0..20_000 { process_events(); … }`. The
//! number said nothing about how long the wait was allowed to take, and falling
//! out of the loop was indistinguishable from succeeding in it: a condition that
//! could never hold spun to the cap and then failed on whichever later assertion
//! happened to read the value, reporting a symptom instead of the wait.

use std::{
    future::Future,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

/// How long a condition driven by the Qt event loop gets to settle.
///
/// Chosen for the failure path, not the happy one: every wait in the suite is
/// satisfied in milliseconds on a healthy run, so this only bounds how long a
/// genuinely broken build takes to say so.
pub const SETTLE_BUDGET: Duration = Duration::from_secs(30);

/// Runs `attempt` until it reports the wait is over, or `budget` runs out.
///
/// `attempt` both drives whatever queue makes progress possible — the Qt event
/// loop, in the GUI harnesses — and reports whether `condition` now holds. It
/// always runs at least once.
///
/// # Panics
///
/// When `budget` elapses without `attempt` reporting success, naming
/// `condition` so the failure describes what never became true rather than
/// whatever a later assertion reads out of the unfinished state.
pub fn pump_until(condition: &str, budget: Duration, mut attempt: impl FnMut() -> bool) {
    let deadline = Instant::now() + budget;
    loop {
        if attempt() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "waited {budget:?} for {condition}, which never became true"
        );
        std::thread::yield_now();
    }
}

/// Drives `future` to completion on the calling thread, yielding between polls.
///
/// For futures written against fakes that complete without a reactor: taking a
/// Tokio runtime to poll them would buy a thread pool the test does not need.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, future::poll_fn, task::Poll, time::Duration};

    use super::{block_on, pump_until};

    #[test]
    fn a_future_that_is_pending_at_first_is_still_driven_to_its_output() {
        // Given: a future that asks to be polled again twice before answering.
        let polls = Cell::new(0_u32);
        let future = poll_fn(|context| {
            polls.set(polls.get() + 1);
            if polls.get() < 3 {
                context.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready("done")
            }
        });

        // When / Then
        assert_eq!(block_on(future), "done");
        assert_eq!(polls.get(), 3);
    }

    #[test]
    fn stops_pumping_once_the_condition_holds() {
        // Given: work that only reports success on its third attempt.
        let attempts = Cell::new(0_u32);

        // When: the helper drives it under a budget far larger than it needs.
        pump_until("the third attempt lands", Duration::from_secs(30), || {
            attempts.set(attempts.get() + 1);
            attempts.get() == 3
        });

        // Then: it returned as soon as the condition held, and not before.
        assert_eq!(attempts.get(), 3);
    }

    #[test]
    #[should_panic(expected = "the radar reaches three frames")]
    fn names_the_condition_that_never_became_true() {
        // Given: work that never succeeds, under a budget short enough to spend.
        // When: the helper exhausts that budget.
        // Then: it panics naming the condition rather than falling through to a
        // later assertion that reports some unrelated value instead.
        pump_until(
            "the radar reaches three frames",
            Duration::from_millis(20),
            || false,
        );
    }
}
