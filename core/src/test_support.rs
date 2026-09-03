//! The one piece of scaffolding this crate's unit tests share.
//!
//! `allesbuien-core` is pure Rust with no async runtime of its own: reqwest
//! brings one for the application, and taking tokio as a dev-dependency to drive
//! futures that are ready on their first poll would buy a thread pool nothing
//! here needs. Every future these tests await is hand-written against a fake and
//! completes synchronously, so polling it to completion is the whole
//! requirement — and one home for that loop is why the transport's tests did not
//! arrive carrying a second copy of it.

use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

/// Drives a future to completion on the calling thread.
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
