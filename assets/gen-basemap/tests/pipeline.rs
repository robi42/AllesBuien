use gen_basemap::{
    LAND, MapProjection, WATER,
    geojson::{GeoPoint, Polygon},
    raster::fill_polygon,
    repository_root,
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
    let (land_column, land_row) = projection.to_pixel_index(GeoPoint::new(2.0, 52.0));
    let (hole_column, hole_row) = projection.to_pixel_index(GeoPoint::new(5.0, 52.0));
    assert_eq!(*image.get_pixel(land_column, land_row), Rgb(LAND));
    assert_eq!(*image.get_pixel(hole_column, hole_row), Rgb(WATER));
}

#[test]
fn pixel_index_rounds_to_the_nearest_pixel_and_saturates_outside_the_extent() {
    // Given
    let projection = MapProjection::default();

    // When
    let southeast = projection.to_pixel_index(GeoPoint::new(10.0, 49.5));
    let beyond_northwest = projection.to_pixel_index(GeoPoint::new(-1.0, 56.0));

    // Then
    assert_eq!(southeast, (1057, 914));
    assert_eq!(beyond_northwest, (0, 0));
}

#[test]
fn repository_root_is_the_checkout_holding_the_basemap_licence() {
    // Given / When
    let root = repository_root().expect("the crate sits two levels below the root");

    // Then
    assert!(root.join("assets/BASEMAP-LICENSE").is_file());
}
