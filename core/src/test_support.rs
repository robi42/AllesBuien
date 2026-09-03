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
    path::Path,
    task::{Context, Poll, Waker},
};

use crate::config::EnvSource;

/// A fixed environment table, so no test has to mutate the process's own.
pub(crate) struct FakeEnv<const N: usize>(pub(crate) [(&'static str, &'static str); N]);

impl<const N: usize> EnvSource for FakeEnv<N> {
    fn var(&self, key: &str) -> Option<String> {
        self.0
            .iter()
            .find_map(|(candidate, value)| (*candidate == key).then(|| (*value).to_owned()))
    }
}

/// Writes Omarchy's `weather.json` under a throwaway home directory.
pub(crate) fn write_weather(home: &Path, json: &str) {
    let path = home.join(".local/state/omarchy/settings/weather.json");
    std::fs::create_dir_all(path.parent().expect("weather parent")).expect("create settings");
    std::fs::write(path, json).expect("write weather fixture");
}

/// Removes it again, as `omarchy-weather-location --clear` does.
pub(crate) fn clear_weather(home: &Path) {
    std::fs::remove_file(home.join(".local/state/omarchy/settings/weather.json"))
        .expect("remove weather fixture");
}

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
