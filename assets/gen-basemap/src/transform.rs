use std::f64::consts::PI;

use crate::{HEIGHT, WIDTH, geojson::GeoPoint};

const EARTH_RADIUS_METERS: f64 = 6_378_137.0;
const MIN_LONGITUDE: f64 = 0.0;
const MAX_LONGITUDE: f64 = 10.0;
const MIN_LATITUDE: f64 = 49.5;
const MAX_LATITUDE: f64 = 54.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct MapProjection {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Default for MapProjection {
    fn default() -> Self {
        let min_x = mercator_x(MIN_LONGITUDE);
        let max_x = mercator_x(MAX_LONGITUDE);
        let min_y = mercator_y(MIN_LATITUDE);
        let max_y = mercator_y(MAX_LATITUDE);
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }
}

impl MapProjection {
    #[must_use]
    pub fn to_pixel(self, point: GeoPoint) -> ScreenPoint {
        let x = (mercator_x(point.longitude()) - self.min_x) / (self.max_x - self.min_x)
            * f64::from(WIDTH - 1);
        let y = (self.max_y - mercator_y(point.latitude())) / (self.max_y - self.min_y)
            * f64::from(HEIGHT - 1);
        ScreenPoint { x, y }
    }
}

fn mercator_x(longitude: f64) -> f64 {
    EARTH_RADIUS_METERS * longitude.to_radians()
}

fn mercator_y(latitude: f64) -> f64 {
    let latitude_radians = latitude.to_radians();
    EARTH_RADIUS_METERS * (PI / 4.0 + latitude_radians / 2.0).tan().ln()
}
