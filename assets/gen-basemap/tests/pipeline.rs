use gen_basemap::{
    LAND, MapProjection, WATER,
    geojson::{GeoPoint, Polygon},
    raster::fill_polygon,
};
use image::{Rgb, RgbImage};

#[test]
fn projection_maps_extent_corners_to_canvas_when_using_web_mercator() {
    // Given
    let projection = MapProjection::default();

    // When
    let northwest = projection.to_pixel(GeoPoint::new(0.0, 54.8));
    let southeast = projection.to_pixel(GeoPoint::new(10.0, 49.5));

    // Then
    assert!((northwest.x - 0.0).abs() < f64::EPSILON);
    assert!((northwest.y - 0.0).abs() < f64::EPSILON);
    assert!((southeast.x - 1057.0).abs() < f64::EPSILON);
    assert!((southeast.y - 914.0).abs() < f64::EPSILON);
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the fixed land and hole samples are inside the test projection extent, so rounding deliberately selects their non-negative canvas pixels"
)]
fn scanline_fill_leaves_hole_water_when_polygon_has_interior_ring() {
    // Given
    let projection = MapProjection::default();
    let polygon = Polygon::new(vec![
        vec![
            GeoPoint::new(1.0, 50.0),
            GeoPoint::new(9.0, 50.0),
            GeoPoint::new(9.0, 54.0),
            GeoPoint::new(1.0, 54.0),
            GeoPoint::new(1.0, 50.0),
        ],
        vec![
            GeoPoint::new(4.0, 51.0),
            GeoPoint::new(6.0, 51.0),
            GeoPoint::new(6.0, 53.0),
            GeoPoint::new(4.0, 53.0),
            GeoPoint::new(4.0, 51.0),
        ],
    ]);
    let mut image = RgbImage::from_pixel(1058, 915, Rgb(WATER));

    // When
    fill_polygon(&mut image, &projection, &polygon, Rgb(LAND));

    // Then
    let land = projection.to_pixel(GeoPoint::new(2.0, 52.0));
    let hole = projection.to_pixel(GeoPoint::new(5.0, 52.0));
    assert_eq!(
        *image.get_pixel(land.x.round() as u32, land.y.round() as u32),
        Rgb(LAND)
    );
    assert_eq!(
        *image.get_pixel(hole.x.round() as u32, hole.y.round() as u32),
        Rgb(WATER)
    );
}
