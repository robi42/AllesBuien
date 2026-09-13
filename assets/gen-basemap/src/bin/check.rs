use std::{error::Error, path::PathBuf};

use gen_basemap::{HEIGHT, LAND, MapProjection, WATER, WIDTH, geojson::GeoPoint, repository_root};
use image::{GenericImageView as _, Rgb};

const TOLERANCE: u8 = 12;
const SAMPLES: [(&str, GeoPoint, [u8; 3]); 6] = [
    ("IJsselmeer", GeoPoint::new(5.35, 52.65), WATER),
    ("North Sea", GeoPoint::new(3.0, 53.5), WATER),
    ("Markermeer", GeoPoint::new(5.25, 52.55), WATER),
    ("Utrecht", GeoPoint::new(5.12, 52.09), LAND),
    ("Eindhoven", GeoPoint::new(5.47, 51.44), LAND),
    ("Groningen", GeoPoint::new(6.57, 53.22), LAND),
];

fn main() -> Result<(), Box<dyn Error>> {
    let input = input_path()?;
    let image = image::open(&input)?;
    let dimensions = image.dimensions();
    if dimensions != (WIDTH, HEIGHT) {
        return Err(format!(
            "{} has dimensions {}x{}, expected {WIDTH}x{HEIGHT}",
            input.display(),
            dimensions.0,
            dimensions.1
        )
        .into());
    }

    let rgb = image.to_rgb8();
    let projection = MapProjection::default();
    let mut failures = Vec::new();
    for (name, point, expected) in SAMPLES {
        let (column, row) = projection.to_pixel_index(point);
        let actual = *rgb.get_pixel(column, row);
        if !matches_color(actual, Rgb(expected)) {
            failures.push(format!(
                "{name}: expected #{:02X}{:02X}{:02X}, found #{:02X}{:02X}{:02X}",
                expected[0], expected[1], expected[2], actual[0], actual[1], actual[2]
            ));
        }
    }
    if failures.is_empty() {
        println!(
            "{}: dimensions and all 6 geography samples pass",
            input.display()
        );
        return Ok(());
    }
    Err(failures.join("\n").into())
}

fn input_path() -> Result<PathBuf, Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(repository_root()?.join("app/assets/basemap-nl.png")),
        [flag, path] if flag == "--input" => Ok(PathBuf::from(path)),
        _ => Err("usage: check [--input <path>]".into()),
    }
}

fn matches_color(actual: Rgb<u8>, expected: Rgb<u8>) -> bool {
    actual
        .0
        .iter()
        .zip(expected.0)
        .all(|(actual, expected)| actual.abs_diff(expected) <= TOLERANCE)
}
