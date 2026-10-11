use crate::model::Point;

const MAX_SEGMENTS: usize = 1_000_000;

/// Split every span longer than `max_segment_mm`.
///
/// The input vertices are kept. Asking for more than one million segments
/// panics: generators refuse that request with a [`crate::PlanError`] before
/// they call this function.
pub fn resample_polyline(points: &[Point], max_segment_mm: f64) -> Vec<Point> {
    assert!(
        max_segment_mm > 0.0 && max_segment_mm.is_finite(),
        "max_segment_mm must be finite and positive"
    );
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut out = Vec::with_capacity(points.len() * 2);
    out.push(points[0]);

    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        let length = start.distance(end);
        if length <= max_segment_mm {
            if length > 0.0 {
                out.push(end);
            }
            continue;
        }

        let segments = (length / max_segment_mm).ceil() as usize;
        assert!(
            segments <= MAX_SEGMENTS,
            "resample_polyline would create {segments} segments; increase max_segment_mm"
        );
        for index in 1..segments {
            let t = index as f64 / segments as f64;
            out.push(start.lerp(end, t));
        }
        out.push(end);
    }

    out
}

pub fn polyline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|window| window[0].distance(window[1]))
        .sum()
}

/// Offset a polyline along its local perpendicular.
///
/// This is the naive normal offset. It is correct for a straight line and for
/// gentle curves. It crosses itself on sharp corners, narrow channels, and
/// self-intersections. Do not use it as the production inset for satin rails
/// or fill holes; the satin generator builds rails from arc-length normals,
/// and a clipper-style offset is still required before this can replace that.
pub fn offset_polyline(points: &[Point], offset_mm: f64) -> Vec<Point> {
    let count = points.len();
    if count < 2 {
        return points.to_vec();
    }

    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let previous = points[index.saturating_sub(1)];
        let next = points[(index + 1).min(count - 1)];
        let current = points[index];
        let dx = next.x - previous.x;
        let dy = next.y - previous.y;
        let norm = (dx * dx + dy * dy).sqrt();
        if norm == 0.0 {
            out.push(current);
            continue;
        }
        // 90° counter-clockwise in this Y-down coordinate system.
        let nx = -dy / norm;
        let ny = dx / norm;
        out.push(Point {
            x: current.x + nx * offset_mm,
            y: current.y + ny * offset_mm,
        });
    }
    out
}

/// A point on a polyline and the unit normal to the left of travel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolylineSample {
    pub point: Point,
    pub normal: Point,
}

/// Sample a polyline at approximately `spacing_mm`, always including both ends.
///
/// The spacing is rounded to an equal division of the total length so the last
/// sample lands on the end vertex. Returns an empty vector when the line has
/// fewer than two distinct points.
pub fn sample_polyline(points: &[Point], spacing_mm: f64) -> Vec<PolylineSample> {
    assert!(
        spacing_mm > 0.0 && spacing_mm.is_finite(),
        "spacing_mm must be finite and positive"
    );
    let cleaned = dedup_points(points);
    if cleaned.len() < 2 {
        return Vec::new();
    }
    let length = polyline_length(&cleaned);
    if length == 0.0 {
        return Vec::new();
    }
    let intervals = (length / spacing_mm).round().max(1.0) as usize;
    assert!(
        intervals <= MAX_SEGMENTS,
        "sample_polyline would create {intervals} samples; increase spacing_mm"
    );

    let mut out = Vec::with_capacity(intervals + 1);
    for index in 0..=intervals {
        let distance = length * (index as f64) / (intervals as f64);
        out.push(sample_at(&cleaned, distance));
    }
    out
}

fn dedup_points(points: &[Point]) -> Vec<Point> {
    let mut out = Vec::with_capacity(points.len());
    for point in points {
        if out
            .last()
            .is_none_or(|previous: &Point| previous.distance(*point) > 1e-9)
        {
            out.push(*point);
        }
    }
    out
}

fn sample_at(points: &[Point], distance: f64) -> PolylineSample {
    let mut walked = 0.0;
    let mut last_segment = (points[0], points[1]);
    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        let span = start.distance(end);
        if span == 0.0 {
            continue;
        }
        last_segment = (start, end);
        if walked + span >= distance - 1e-9 {
            let t = ((distance - walked) / span).clamp(0.0, 1.0);
            return PolylineSample {
                point: start.lerp(end, t),
                normal: left_normal(end.x - start.x, end.y - start.y),
            };
        }
        walked += span;
    }
    let (start, end) = last_segment;
    PolylineSample {
        point: *points.last().unwrap_or(&start),
        normal: left_normal(end.x - start.x, end.y - start.y),
    }
}

fn left_normal(dx: f64, dy: f64) -> Point {
    let norm = (dx * dx + dy * dy).sqrt();
    Point {
        x: -dy / norm,
        y: dx / norm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_keeps_short_spans_and_splits_long_ones() {
        let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let sampled = resample_polyline(&line, 3.0);
        assert_eq!(sampled.len(), 5);
        assert_eq!(sampled[0], Point::new(0.0, 0.0));
        assert!((sampled[1].x - 2.5).abs() < 1e-9);
        assert_eq!(sampled[4], Point::new(10.0, 0.0));
        assert!((polyline_length(&sampled) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn resample_drops_zero_length_duplicates() {
        let line = [
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
        ];
        let sampled = resample_polyline(&line, 1.0);
        assert_eq!(sampled.len(), 2);
    }

    #[test]
    fn offset_of_a_horizontal_line_is_vertical() {
        let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let offset = offset_polyline(&line, 1.5);
        assert_eq!(offset.len(), 2);
        assert!((offset[0].y - 1.5).abs() < 1e-9);
        assert!((offset[1].y - 1.5).abs() < 1e-9);
        assert!((offset[1].x - 10.0).abs() < 1e-9);
    }

    #[test]
    fn sample_includes_endpoints_and_unit_normals() {
        let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let samples = sample_polyline(&line, 5.0);
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].point, Point::new(0.0, 0.0));
        assert_eq!(samples[1].point, Point::new(5.0, 0.0));
        assert_eq!(samples[2].point, Point::new(10.0, 0.0));
        assert!((samples[1].normal.x).abs() < 1e-9);
        assert!((samples[1].normal.y - 1.0).abs() < 1e-9);
    }
}
