use std::{error::Error, fmt};

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoPoint {
    longitude: f64,
    latitude: f64,
}

impl GeoPoint {
    #[must_use]
    pub const fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    #[must_use]
    pub const fn longitude(self) -> f64 {
        self.longitude
    }

    #[must_use]
    pub const fn latitude(self) -> f64 {
        self.latitude
    }
}

#[derive(Debug)]
pub struct Polygon {
    rings: Vec<Vec<GeoPoint>>,
}

impl Polygon {
    #[must_use]
    pub const fn new(rings: Vec<Vec<GeoPoint>>) -> Self {
        Self { rings }
    }

    #[must_use]
    pub fn rings(&self) -> &[Vec<GeoPoint>] {
        &self.rings
    }
}

#[derive(Debug)]
pub struct GeoJsonError(String);

impl fmt::Display for GeoJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for GeoJsonError {}

/// Parses polygon and multipolygon features from a `GeoJSON` document.
///
/// # Errors
/// Returns [`GeoJsonError`] when the document is invalid or contains a missing,
/// malformed, or unsupported geometry.
pub fn parse_polygons(bytes: &[u8]) -> Result<Vec<Polygon>, GeoJsonError> {
    let document: Value = serde_json::from_slice(bytes)
        .map_err(|error| GeoJsonError(format!("invalid GeoJSON: {error}")))?;
    let geometries = feature_geometries(&document)?;
    let mut polygons = Vec::new();
    for geometry in geometries {
        let geometry_type = string_field(geometry, "type")?;
        let coordinates = geometry
            .get("coordinates")
            .ok_or_else(|| GeoJsonError("geometry has no coordinates".to_owned()))?;
        match geometry_type {
            "Polygon" => polygons.push(parse_polygon(coordinates)?),
            "MultiPolygon" => {
                for polygon in array(coordinates, "MultiPolygon coordinates")? {
                    polygons.push(parse_polygon(polygon)?);
                }
            }
            other => {
                return Err(GeoJsonError(format!(
                    "expected Polygon or MultiPolygon, found {other}"
                )));
            }
        }
    }
    Ok(polygons)
}

/// Parses line-string and multiline-string features from a `GeoJSON` document.
///
/// # Errors
/// Returns [`GeoJsonError`] when the document is invalid or contains a missing,
/// malformed, or unsupported geometry.
pub fn parse_lines(bytes: &[u8]) -> Result<Vec<Vec<GeoPoint>>, GeoJsonError> {
    let document: Value = serde_json::from_slice(bytes)
        .map_err(|error| GeoJsonError(format!("invalid GeoJSON: {error}")))?;
    let geometries = feature_geometries(&document)?;
    let mut lines = Vec::new();
    for geometry in geometries {
        let geometry_type = string_field(geometry, "type")?;
        let coordinates = geometry
            .get("coordinates")
            .ok_or_else(|| GeoJsonError("geometry has no coordinates".to_owned()))?;
        match geometry_type {
            "LineString" => lines.push(parse_ring(coordinates)?),
            "MultiLineString" => {
                for line in array(coordinates, "MultiLineString coordinates")? {
                    lines.push(parse_ring(line)?);
                }
            }
            other => {
                return Err(GeoJsonError(format!(
                    "expected LineString or MultiLineString, found {other}"
                )));
            }
        }
    }
    Ok(lines)
}

fn feature_geometries(document: &Value) -> Result<Vec<&Value>, GeoJsonError> {
    let features = document
        .get("features")
        .ok_or_else(|| GeoJsonError("GeoJSON has no features".to_owned()))?;
    array(features, "features")?
        .iter()
        .map(|feature| {
            feature
                .get("geometry")
                .ok_or_else(|| GeoJsonError("feature has no geometry".to_owned()))
        })
        .collect()
}

fn parse_polygon(value: &Value) -> Result<Polygon, GeoJsonError> {
    let rings = array(value, "polygon coordinates")?
        .iter()
        .map(parse_ring)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Polygon::new(rings))
}

fn parse_ring(value: &Value) -> Result<Vec<GeoPoint>, GeoJsonError> {
    array(value, "line coordinates")?
        .iter()
        .map(|position| {
            let numbers = array(position, "position")?;
            let longitude = numbers
                .first()
                .and_then(Value::as_f64)
                .ok_or_else(|| GeoJsonError("position has no longitude".to_owned()))?;
            let latitude = numbers
                .get(1)
                .and_then(Value::as_f64)
                .ok_or_else(|| GeoJsonError("position has no latitude".to_owned()))?;
            Ok(GeoPoint::new(longitude, latitude))
        })
        .collect()
}

fn array<'a>(value: &'a Value, context: &str) -> Result<&'a Vec<Value>, GeoJsonError> {
    value
        .as_array()
        .ok_or_else(|| GeoJsonError(format!("{context} is not an array")))
}

fn string_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, GeoJsonError> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| GeoJsonError(format!("missing string field {name}")))
}
