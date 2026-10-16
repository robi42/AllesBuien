//! The background half of a refresh: the work a bridge spawns onto Tokio.
//!
//! A bridge's `refresh` guards its ports, clones them, spawns one of these, and
//! queues the result back onto the Qt thread. Nothing here holds a
//! `Pin<&mut Self>`, which is what lets the retry ladder be written once for both
//! feeds and exercised without a `QGuiApplication`.

use std::{future::Future, path::PathBuf, sync::Arc};

use allesbuien_core::{
    error_chain,
    location::LocationConfig,
    radar::{GenerationId, RadarApi, RadarClient, RadarError},
    retry,
    weather::{FeedApi, WeatherData, WeatherError},
};

use crate::{
    image_provider::FrameBytes, radar_frames::LoadedRefresh, services::NotificationService,
};

/// Runs `attempt` until it succeeds, fails for good, or uses up its attempts.
///
/// A failure `retryable` rejects is returned at once; a retryable one waits out
/// [`retry::backoff`] and tries again, [`retry::MAX_ATTEMPTS`] times in total,
/// and the last result is returned whatever it was.
pub(crate) async fn with_retries<T, E, Fut>(
    retryable: fn(&E) -> bool,
    mut attempt: impl FnMut() -> Fut,
) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    let mut result = attempt().await;
    for attempt_number in 2..=retry::MAX_ATTEMPTS {
        match &result {
            Err(error) if retryable(error) => {}
            _ => break,
        }
        tokio::time::sleep(retry::backoff(attempt_number)).await;
        result = attempt().await;
    }
    result
}

/// Downloads one radar run for `generation` and reads every frame into memory.
///
/// # Errors
///
/// Returns the run's transport, metadata, or cache error once retries are
/// exhausted, or [`RadarError::Cache`] when a downloaded frame cannot be read.
pub(crate) async fn load_radar_run(
    api: Arc<dyn RadarApi>,
    state_dir: PathBuf,
    generation: GenerationId,
) -> Result<LoadedRefresh, RadarError> {
    let client = RadarClient::new(api, state_dir);
    let refresh = with_retries(retry::is_retryable, || client.refresh(generation)).await?;
    let frames = refresh
        .frames
        .iter()
        .map(|frame| std::fs::read(&frame.path).map(FrameBytes::from))
        .collect::<Result<Vec<_>, _>>()
        .map_err(RadarError::Cache)?;
    Ok(LoadedRefresh { refresh, frames })
}

/// Fetches the feed for `location` and announces coming rain.
///
/// A failed notification is logged and does not fail the refresh: the forecast
/// is still worth showing when `notify-send` is missing.
///
/// # Errors
///
/// Returns the feed's error once retries are exhausted.
pub(crate) async fn fetch_weather(
    feed: Arc<dyn FeedApi>,
    notifications: NotificationService,
    location: LocationConfig,
) -> Result<WeatherData, WeatherError> {
    let data = with_retries(retry::is_weather_retryable, || {
        feed.fetch(location.latitude, location.longitude)
    })
    .await?;
    let notified = notifications
        .notify_rain(data.rain_soon.as_ref(), location.display_label())
        .await;
    if let Err(error) = notified {
        eprintln!(
            "allesbuien: notification failed: {}",
            error_chain::describe(&error)
        );
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::{
        future::ready,
        sync::atomic::{AtomicU32, Ordering},
    };

    use allesbuien_core::retry;

    use super::with_retries;

    fn transient(error: &&str) -> bool {
        *error == "transient"
    }

    fn run<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("a test runtime")
            .block_on(future)
    }

    #[test]
    fn a_permanent_failure_is_returned_after_one_attempt() {
        // Given
        let attempts = AtomicU32::new(0);

        // When
        let result: Result<(), &str> = run(with_retries(transient, || {
            attempts.fetch_add(1, Ordering::Relaxed);
            ready(Err("permanent"))
        }));

        // Then
        assert_eq!(result, Err("permanent"));
        assert_eq!(attempts.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_transient_failure_is_retried_until_the_last_attempt() {
        // Given
        let attempts = AtomicU32::new(0);

        // When
        let result: Result<(), &str> = run(with_retries(transient, || {
            attempts.fetch_add(1, Ordering::Relaxed);
            ready(Err("transient"))
        }));

        // Then
        assert_eq!(result, Err("transient"));
        assert_eq!(attempts.load(Ordering::Relaxed), retry::MAX_ATTEMPTS);
    }

    #[test]
    fn retrying_stops_at_the_first_success() {
        // Given
        let attempts = AtomicU32::new(0);

        // When
        let result = run(with_retries(transient, || {
            let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
            ready(if attempt == 2 {
                Ok(attempt)
            } else {
                Err("transient")
            })
        }));

        // Then
        assert_eq!(result, Ok(2));
        assert_eq!(attempts.load(Ordering::Relaxed), 2);
    }
}
