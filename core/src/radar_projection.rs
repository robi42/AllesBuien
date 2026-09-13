//! Geographic projection of Buienradar's `RadarMapRainWebmercatorNL` raster.
//!
//! Buienradar publishes no bounding box for this product: the metadata endpoint
//! carries the pixel dimensions and nothing georeferenced. The bounds below are
//! reverse-engineered from `Kevinjil/ha-map-card-buienradar`, which overlays the
//! same product at the same 1058x915 resolution through Leaflet's
//! `L.latLngBounds([[49.5, 0], [54.8, 10]])`.
//!
//! Two independent checks back that: converting the corners to EPSG:3857 metres
//! gives a bbox aspect of 1.155947 against the image's 1058/915 = 1.156284, a
//! 0.03% match; and correlating a live `renderBackground=true` frame against the
//! Natural Earth basemap in `assets/gen-basemap` — which is projected with these
//! same bounds — locks the two coastlines to within 3 pixels.
//!
//! Positions are returned as fractions of the image extent rather than pixels,
//! because the caller scales the raster into a panel and only ever needs a
//! relative position. That also keeps this module independent of the resolution
//! the endpoint happens to be asked for.

use std::f64::consts::PI;

/// Semi-major axis of the sphere EPSG:3857 projects onto, in metres.
const EARTH_RADIUS_METERS: f64 = 6_378_137.0;
/// Southern edge of the radar raster, in degrees.
pub(crate) const SOUTH_LATITUDE: f64 = 49.5;
/// Northern edge of the radar raster, in degrees.
pub(crate) const NORTH_LATITUDE: f64 = 54.8;
/// Western edge of the radar raster, in degrees.
pub(crate) const WEST_LONGITUDE: f64 = 0.0;
/// Eastern edge of the radar raster, in degrees.
pub(crate) const EAST_LONGITUDE: f64 = 10.0;

/// A point inside the radar raster, as a fraction of its width and height.
///
/// `x` runs west to east and `y` runs **north to south**, matching image row
/// order: the northern edge is `y == 0.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerPosition {
    pub x: f64,
    pub y: f64,
}

/// Projects a coordinate onto the radar raster.
///
/// Returns `None` when the coordinate falls outside the radar's coverage, which
/// is a configuration the app must render as "no marker" rather than clamp to an
/// edge and claim the location sits on the North Sea.
#[must_use]
pub fn marker_position(latitude: f64, longitude: f64) -> Option<MarkerPosition> {
    if !(SOUTH_LATITUDE..=NORTH_LATITUDE).contains(&latitude)
        || !(WEST_LONGITUDE..=EAST_LONGITUDE).contains(&longitude)
    {
        return None;
    }
    let (west, east) = (mercator_x(WEST_LONGITUDE), mercator_x(EAST_LONGITUDE));
    let (south, north) = (mercator_y(SOUTH_LATITUDE), mercator_y(NORTH_LATITUDE));
    Some(MarkerPosition {
        x: (mercator_x(longitude) - west) / (east - west),
        y: (north - mercator_y(latitude)) / (north - south),
    })
}

fn mercator_x(longitude: f64) -> f64 {
    EARTH_RADIUS_METERS * longitude.to_radians()
}

fn mercator_y(latitude: f64) -> f64 {
    EARTH_RADIUS_METERS * (PI / 4.0 + latitude.to_radians() / 2.0).tan().ln()
}

#[cfg(test)]
mod tests {
    use super::{EAST_LONGITUDE, NORTH_LATITUDE, SOUTH_LATITUDE, WEST_LONGITUDE, marker_position};

    fn position(latitude: f64, longitude: f64) -> (f64, f64) {
        let marker = marker_position(latitude, longitude).expect("inside the radar bounds");
        (marker.x, marker.y)
    }

    #[test]
    fn the_bounding_corners_land_on_the_corners_of_the_image() {
        // Given / When
        let north_west = position(NORTH_LATITUDE, WEST_LONGITUDE);
        let south_east = position(SOUTH_LATITUDE, EAST_LONGITUDE);

        // Then: north is row zero, so the north-west corner is the origin.
        assert!(north_west.0.abs() < 1e-12, "west edge at {}", north_west.0);
        assert!(north_west.1.abs() < 1e-12, "north edge at {}", north_west.1);
        assert!((south_east.0 - 1.0).abs() < 1e-12, "east edge");
        assert!((south_east.1 - 1.0).abs() < 1e-12, "south edge");
    }

    #[test]
    fn the_configured_default_lands_where_the_landmark_check_measured_it() {
        // Given: the Amsterdam-West default from `config.rs`, measured at pixel
        // (512.07, 433.11) of the 1058x915 raster during bbox validation.
        // When
        let (x, y) = position(52.37, 4.84);

        // Then
        assert!((x * 1058.0 - 512.07).abs() < 0.05, "x pixel {}", x * 1058.0);
        assert!((y * 915.0 - 433.11).abs() < 0.05, "y pixel {}", y * 915.0);
    }

    #[test]
    fn latitude_is_mercator_stretched_rather_than_linear_in_degrees() {
        // Given: the exact midpoint latitude of the bounding box.
        let midpoint = f64::midpoint(SOUTH_LATITUDE, NORTH_LATITUDE);

        // When
        let (_, y) = position(midpoint, 5.0);

        // Then: a linear-in-degrees projection would put this at 0.5.
        assert!((y - 0.514_896_630_4).abs() < 1e-9, "midpoint y {y}");
        assert!(y > 0.51, "latitude was not Mercator-projected");
    }

    #[test]
    fn longitude_stays_linear_because_mercator_x_is_a_scaled_angle() {
        // Given / When
        let (x, _) = position(52.0, 2.5);

        // Then
        assert!((x - 0.25).abs() < 1e-12, "x {x}");
    }

    #[test]
    fn positions_order_west_to_east_and_north_to_south() {
        // Given: Groningen is north-east of Maastricht.
        // When
        let (groningen_x, groningen_y) = position(53.22, 6.57);
        let (maastricht_x, maastricht_y) = position(50.85, 5.69);

        // Then
        assert!(groningen_x > maastricht_x, "east is a larger x");
        assert!(groningen_y < maastricht_y, "north is a smaller y");
    }

    #[test]
    fn a_location_outside_the_dutch_radar_has_no_marker() {
        // Given / When / Then
        assert_eq!(marker_position(48.85, 2.35), None, "Paris is south of it");
        assert_eq!(marker_position(51.5, -0.13), None, "London is west of it");
        assert_eq!(marker_position(55.7, 12.6), None, "Copenhagen is beyond it");
        assert_eq!(marker_position(f64::NAN, 5.0), None, "an unusable latitude");
    }
}
