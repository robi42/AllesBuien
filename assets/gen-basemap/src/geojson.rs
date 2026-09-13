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
    parse_features(bytes, "Polygon", "MultiPolygon", parse_polygon)
}

/// Parses line-string and multiline-string features from a `GeoJSON` document.
///
/// # Errors
/// Returns [`GeoJsonError`] when the document is invalid or contains a missing,
/// malformed, or unsupported geometry.
pub fn parse_lines(bytes: &[u8]) -> Result<Vec<Vec<GeoPoint>>, GeoJsonError> {
    parse_features(bytes, "LineString", "MultiLineString", parse_ring)
}

/// Collects every feature whose geometry is `single`, or `multi` of it.
///
/// `GeoJSON` pairs each geometry with a multi- form whose coordinates are an
/// array of the single form's, so one walk serves both feature kinds: a single
/// geometry parses once, a multi geometry parses each member, and any other type
/// is refused by name rather than skipped.
fn parse_features<T>(
    bytes: &[u8],
    single: &str,
    multi: &str,
    parse_one: impl Fn(&Value) -> Result<T, GeoJsonError>,
) -> Result<Vec<T>, GeoJsonError> {
    let document: Value = serde_json::from_slice(bytes)
        .map_err(|error| GeoJsonError(format!("invalid GeoJSON: {error}")))?;
    let geometries = feature_geometries(&document)?;
    let mut parsed = Vec::new();
    for geometry in geometries {
        let geometry_type = string_field(geometry, "type")?;
        let coordinates = geometry
            .get("coordinates")
            .ok_or_else(|| GeoJsonError("geometry has no coordinates".to_owned()))?;
        if geometry_type == single {
            parsed.push(parse_one(coordinates)?);
        } else if geometry_type == multi {
            for member in array(coordinates, &format!("{multi} coordinates"))? {
                parsed.push(parse_one(member)?);
            }
        } else {
            return Err(GeoJsonError(format!(
                "expected {single} or {multi}, found {geometry_type}"
            )));
        }
    }
    Ok(parsed)
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

#[cfg(test)]
mod tests {
    use super::{GeoPoint, parse_lines, parse_polygons};

    #[test]
    fn a_single_and_a_multi_geometry_both_yield_their_members() {
        // Given
        let document = br#"{"features":[
            {"geometry":{"type":"LineString","coordinates":[[1,50],[2,51]]}},
            {"geometry":{"type":"MultiLineString","coordinates":[[[3,52],[4,53]],[[5,54],[6,55]]]}}
        ]}"#;

        // When
        let lines = parse_lines(document).expect("both geometry forms parse");

        // Then
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2][1], GeoPoint::new(6.0, 55.0));
    }

    #[test]
    fn a_geometry_of_the_other_family_is_refused_by_name() {
        // Given
        let document = br#"{"features":[
            {"geometry":{"type":"LineString","coordinates":[[1,50],[2,51]]}}
        ]}"#;

        // When
        let error = parse_polygons(document).expect_err("a line is not a polygon");

        // Then
        assert_eq!(
            error.to_string(),
            "expected Polygon or MultiPolygon, found LineString"
        );
    }

    #[test]
    fn a_multi_geometry_whose_coordinates_are_not_an_array_names_its_type() {
        // Given
        let document = br#"{"features":[
            {"geometry":{"type":"MultiPolygon","coordinates":7}}
        ]}"#;

        // When
        let error = parse_polygons(document).expect_err("coordinates must be an array");

        // Then
        assert_eq!(
            error.to_string(),
            "MultiPolygon coordinates is not an array"
        );
    }
}
