//! The inert stubs and fixture location more than one GUI harness needs.
//!
//! Each of these is inert on purpose. A harness reaches for one when the port
//! is a dependency of the thing under test rather than the subject of it: the
//! theme harness needs a feed only because `AppBuilder` demands one. When a
//! harness wants a port to behave — to script generations, to serve two
//! distinguishable frames, to count dispatches — it writes that fake beside its
//! own assertions instead.

use allesbuien_core::{
    location::{LocationConfig, LocationSource},
    location_settings::LocationSettings,
    notify::{Notifier, NotifyError},
    radar::{RadarApi, RadarError, RadarFuture},
    weather::{FeedApi, FeedFuture, WeatherData},
};

/// The configured location the harnesses boot with.
///
/// The coordinates are load-bearing twice over: the label reaches the window
/// header and the notification title, and `radar_controller` projects this
/// latitude and longitude onto a pixel of the basemap raster and asserts where
/// the marker lands. Moving the fixture moves that expectation with it.
///
/// It is pinned rather than watched so that a harness asserting something else
/// entirely cannot be moved by whatever `weather.json` the machine running the
/// suite happens to hold. The one harness that *is* about following that file
/// builds its own settings over a temporary home.
#[must_use]
pub fn amsterdam_west() -> LocationSettings {
    LocationSettings::pinned(LocationConfig {
        latitude: 52.37,
        longitude: 4.84,
        label: String::from("Amsterdam-West"),
        source: LocationSource::BuiltInDefault,
    })
}

/// A feed that always answers, and always with nothing.
pub struct StubFeedApi;

impl FeedApi for StubFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(async {
            Ok(WeatherData {
                current: None,
                forecast: Vec::new(),
                rain_line: String::new(),
                rain_soon: None,
                rain_series: Vec::new(),
            })
        })
    }
}

/// A radar whose run metadata always fails, so no frame is ever reached.
///
/// The failing metadata is the point: it leaves the radar view on its inline
/// error status without a network call, which is what a harness watching some
/// other view wants from it.
pub struct StubRadarApi;

impl RadarApi for StubRadarApi {
    fn metadata(&self) -> RadarFuture<'_, Vec<u8>> {
        Box::pin(async {
            Err(RadarError::InvalidRunTimestamp {
                timestamp: String::new(),
            })
        })
    }

    fn frame<'a>(&'a self, url: &'a str) -> RadarFuture<'a, Vec<u8>> {
        Box::pin(async move { unreachable!("a run that failed has no frame to fetch: {url}") })
    }
}

/// A notifier that accepts every dispatch and remembers none of them.
pub struct StubNotifier;

impl Notifier for StubNotifier {
    fn send(&self, _title: &str, _body: &str) -> Result<(), NotifyError> {
        Ok(())
    }
}
