use image::{Rgb, RgbImage};

use crate::{HEIGHT, MapProjection, ScreenPoint, WIDTH, geojson::Polygon};

#[derive(Clone, Copy)]
struct Edge {
    start: ScreenPoint,
    end: ScreenPoint,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "scanline coordinates are clipped to the fixed 1058x915 canvas before deliberate integer-pixel rounding, and every row index is at most 914"
)]
pub fn fill_polygon(
    image: &mut RgbImage,
    projection: &MapProjection,
    polygon: &Polygon,
    color: Rgb<u8>,
) {
    let mut scanlines = vec![Vec::<Edge>::new(); HEIGHT as usize];
    for ring in polygon.rings() {
        for pair in ring.windows(2) {
            let start = projection.to_pixel(pair[0]);
            let end = projection.to_pixel(pair[1]);
            if (start.y - end.y).abs() < f64::EPSILON {
                continue;
            }
            let first_y = start.y.min(end.y).ceil().max(0.0) as u32;
            let last_y = start.y.max(end.y).ceil().min(f64::from(HEIGHT)) as u32;
            for y in first_y..last_y {
                if let Some(bucket) = scanlines.get_mut(y as usize) {
                    bucket.push(Edge { start, end });
                }
            }
        }
    }

    for (y, edges) in scanlines.iter().enumerate() {
        let scan_y = y as f64 + 0.5;
        let mut intersections = edges
            .iter()
            .map(|edge| {
                edge.start.x
                    + (scan_y - edge.start.y) * (edge.end.x - edge.start.x)
                        / (edge.end.y - edge.start.y)
            })
            .collect::<Vec<_>>();
        intersections.sort_by(f64::total_cmp);
        for span in intersections.as_chunks::<2>().0 {
            let start_x = span[0].ceil().max(0.0) as u32;
            let end_x = span[1].floor().min(f64::from(WIDTH - 1)) as u32;
            if start_x <= end_x {
                for x in start_x..=end_x {
                    image.put_pixel(x, y as u32, color);
                }
            }
        }
    }
}

pub fn draw_line(
    image: &mut RgbImage,
    projection: &MapProjection,
    line: &[crate::geojson::GeoPoint],
    color: Rgb<u8>,
) {
    for pair in line.windows(2) {
        let start = projection.to_pixel(pair[0]);
        let end = projection.to_pixel(pair[1]);
        if let Some((clipped_start, clipped_end)) = clip_to_canvas(start, end) {
            draw_clipped_segment(image, clipped_start, clipped_end, color);
        }
    }
}

/// Clips a segment to the canvas with the Liang-Barsky algorithm.
///
/// The segment is `start + t * (end - start)` for `t` in `0..=1`. Each canvas
/// edge contributes one `(p, q)` test: `p` is the segment's direction across
/// that edge and `q` the start point's distance inside it. A negative `p` means
/// the segment enters through that edge and raises the entry `t`; a positive one
/// means it leaves and lowers the exit `t`. A zero `p` runs parallel to the edge
/// and is either wholly inside (`q >= 0`) or wholly outside it.
fn clip_to_canvas(start: ScreenPoint, end: ScreenPoint) -> Option<(ScreenPoint, ScreenPoint)> {
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let edge_tests = [
        (-delta_x, start.x),
        (delta_x, f64::from(WIDTH - 1) - start.x),
        (-delta_y, start.y),
        (delta_y, f64::from(HEIGHT - 1) - start.y),
    ];
    let mut enter = 0.0_f64;
    let mut leave = 1.0_f64;
    for (direction, distance) in edge_tests {
        if direction.abs() < f64::EPSILON {
            if distance < 0.0 {
                return None;
            }
            continue;
        }
        let ratio = distance / direction;
        if direction < 0.0 {
            enter = enter.max(ratio);
        } else {
            leave = leave.min(ratio);
        }
    }
    if enter > leave {
        return None;
    }
    Some((
        ScreenPoint {
            x: start.x + enter * delta_x,
            y: start.y + enter * delta_y,
        },
        ScreenPoint {
            x: start.x + leave * delta_x,
            y: start.y + leave * delta_y,
        },
    ))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Liang-Barsky clipping bounds both endpoints to the 1058x915 canvas before Bresenham rounds them to integer pixels"
)]
/// Draws an already-clipped segment pixel by pixel with Bresenham's algorithm.
fn draw_clipped_segment(
    image: &mut RgbImage,
    start: ScreenPoint,
    end: ScreenPoint,
    color: Rgb<u8>,
) {
    let mut x = start.x.round() as i32;
    let mut y = start.y.round() as i32;
    let target_x = end.x.round() as i32;
    let target_y = end.y.round() as i32;
    let delta_x = (target_x - x).abs();
    let step_x = if x < target_x { 1 } else { -1 };
    let delta_y = -(target_y - y).abs();
    let step_y = if y < target_y { 1 } else { -1 };
    let mut error = delta_x + delta_y;

    loop {
        if let (Ok(pixel_x), Ok(pixel_y)) = (u32::try_from(x), u32::try_from(y))
            && pixel_x < WIDTH
            && pixel_y < HEIGHT
        {
            image.put_pixel(pixel_x, pixel_y, color);
        }
        if x == target_x && y == target_y {
            break;
        }
        let doubled_error = 2 * error;
        if doubled_error >= delta_y {
            error += delta_y;
            x += step_x;
        }
        if doubled_error <= delta_x {
            error += delta_x;
            y += step_y;
        }
    }
}
