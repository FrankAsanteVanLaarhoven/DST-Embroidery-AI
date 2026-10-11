use crate::error::PlanError;
use crate::geometry::resample_polyline;
use crate::model::{Point, Stitch, StitchKind, StitchPlan};

use super::{commit, ensure_finite, push_run};

/// Scanline fill parameters.
///
/// `angle_degrees` is the stitch direction, with 0° sewing along +X.
/// Rows are inset by half a spacing so the first and last pass are inside the
/// outline. A non-positive `underlay_spacing_mm` skips the perpendicular underlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillConfig {
    pub angle_degrees: f64,
    pub row_spacing_mm: f64,
    pub max_stitch_length_mm: f64,
    pub underlay_spacing_mm: f64,
}

impl Default for FillConfig {
    fn default() -> Self {
        Self {
            angle_degrees: 45.0,
            row_spacing_mm: 0.4,
            max_stitch_length_mm: 5.0,
            underlay_spacing_mm: 2.0,
        }
    }
}

/// Fill one simple outline.
///
/// The outline is treated as a closed loop. The last edge is added when the
/// first and last points differ. This entry does not sew holes. Use
/// [`generate_fill_with_holes`] for one outer ring and explicit holes.
/// Self-intersections and curved angle fields are not repaired: an odd number
/// of crossings on a row is an error and the plan is not changed. Rows
/// alternate direction to shorten travel.
pub fn generate_fill(
    plan: &mut StitchPlan,
    object_id: u64,
    color_index: usize,
    polygon: &[Point],
    config: &FillConfig,
) -> Result<(), PlanError> {
    validate_config(config)?;
    let ring = open_ring(polygon);
    if ring.len() < 3 {
        return Err(PlanError::new(
            "Fill needs a closed outline with at least three distinct points. Close the polygon and try again.",
        ));
    }
    ensure_finite(&ring, "Fill outline")?;

    let mut batch = Vec::new();
    let plan_pen = plan.stitches.last().map(|stitch| stitch.to);
    if config.underlay_spacing_mm > 0.0 {
        sew_rows(
            &mut batch,
            plan_pen,
            &ring,
            &RowPass {
                angle_degrees: config.angle_degrees + 90.0,
                spacing_mm: config.underlay_spacing_mm,
                max_stitch_mm: config.max_stitch_length_mm,
                kind: StitchKind::Running,
                color_index,
                object_id,
            },
        )?;
    }
    sew_rows(
        &mut batch,
        plan_pen,
        &ring,
        &RowPass {
            angle_degrees: config.angle_degrees,
            spacing_mm: config.row_spacing_mm,
            max_stitch_mm: config.max_stitch_length_mm,
            kind: StitchKind::Fill,
            color_index,
            object_id,
        },
    )?;

    if !batch.iter().any(|stitch| stitch.kind == StitchKind::Fill) {
        return Err(PlanError::new(
            "Fill produced no stitches. The row spacing is larger than the shape, or the outline is degenerate. Tighten the outline or reduce row_spacing_mm.",
        ));
    }
    commit(plan, batch)
}

fn validate_config(config: &FillConfig) -> Result<(), PlanError> {
    if !config.angle_degrees.is_finite() {
        return Err(PlanError::new(
            "Fill angle_degrees must be a finite angle. Use 0 for horizontal stitches.",
        ));
    }
    if !config.row_spacing_mm.is_finite() || config.row_spacing_mm <= 0.0 {
        return Err(PlanError::new(
            "Fill row_spacing_mm must be a positive distance between rows.",
        ));
    }
    if !config.max_stitch_length_mm.is_finite() || config.max_stitch_length_mm <= 0.0 {
        return Err(PlanError::new(
            "Fill max_stitch_length_mm must be a positive stitch length.",
        ));
    }
    if !config.underlay_spacing_mm.is_finite() || config.underlay_spacing_mm < 0.0 {
        return Err(PlanError::new(
            "Fill underlay_spacing_mm cannot be negative. Use 0 to skip underlay.",
        ));
    }
    Ok(())
}

fn open_ring(points: &[Point]) -> Vec<Point> {
    let mut ring = points.to_vec();
    if ring.len() >= 2 && ring[0].distance(*ring.last().expect("length checked")) <= 1e-9 {
        ring.pop();
    }
    ring
}

struct Span {
    start: Point,
    end: Point,
}

struct RowPass {
    angle_degrees: f64,
    spacing_mm: f64,
    max_stitch_mm: f64,
    kind: StitchKind,
    color_index: usize,
    object_id: u64,
}

fn sew_rows(
    batch: &mut Vec<Stitch>,
    plan_pen: Option<Point>,
    ring: &[Point],
    pass: &RowPass,
) -> Result<(), PlanError> {
    let theta = pass.angle_degrees.to_radians();
    let direction = Point::new(theta.cos(), theta.sin());
    let normal = Point::new(-direction.y, direction.x);

    let mut min_across = f64::INFINITY;
    let mut max_across = f64::NEG_INFINITY;
    for point in ring {
        let across = project(*point, normal);
        min_across = min_across.min(across);
        max_across = max_across.max(across);
    }
    if !min_across.is_finite() || max_across - min_across <= 1e-9 {
        return Ok(());
    }

    let mut reverse = false;
    let mut index = 0usize;
    loop {
        let across = min_across + pass.spacing_mm * (index as f64 + 0.5);
        if across >= max_across - 1e-9 {
            break;
        }
        index += 1;
        if index > 1_000_000 {
            return Err(PlanError::new(
                "Fill would create more than 1000000 rows. Increase row_spacing_mm or split the outline.",
            ));
        }

        let mut along = crossings(ring, across, normal, direction)?;
        if along.len() < 2 {
            reverse = !reverse;
            continue;
        }
        along.sort_by(|left, right| left.total_cmp(right));
        let mut spans = Vec::new();
        for pair in along.chunks(2) {
            if pair.len() < 2 || (pair[1] - pair[0]).abs() <= 1e-6 {
                continue;
            }
            spans.push(Span {
                start: unproject(pair[0], across, direction, normal),
                end: unproject(pair[1], across, direction, normal),
            });
        }
        if reverse {
            spans.reverse();
            for span in &mut spans {
                std::mem::swap(&mut span.start, &mut span.end);
            }
        }
        for span in spans {
            let line = resample_polyline(&[span.start, span.end], pass.max_stitch_mm);
            push_run(
                batch,
                plan_pen,
                &line,
                pass.kind,
                pass.color_index,
                pass.object_id,
            );
        }
        reverse = !reverse;
    }
    Ok(())
}

fn crossings(
    ring: &[Point],
    across: f64,
    normal: Point,
    direction: Point,
) -> Result<Vec<f64>, PlanError> {
    let mut along = Vec::new();
    let count = ring.len();
    for index in 0..count {
        let p0 = ring[index];
        let p1 = ring[(index + 1) % count];
        let d0 = project(p0, normal);
        let d1 = project(p1, normal);
        if (d0 - d1).abs() <= 1e-12 {
            continue;
        }
        let (start, end, low, high) = if d0 < d1 {
            (p0, p1, d0, d1)
        } else {
            (p1, p0, d1, d0)
        };
        // Half-open [low, high) so a vertex shared by two edges is counted once.
        if across >= low && across < high {
            let t = (across - low) / (high - low);
            let point = start.lerp(end, t);
            along.push(project(point, direction));
        }
    }
    if along.len() % 2 == 1 {
        return Err(PlanError::new(format!(
            "Fill row at {across:.3} mm crossed the outline {} times. The outline is self-intersecting or not a simple loop. Simplify it and fill again. The plan was not changed.",
            along.len()
        )));
    }
    Ok(along)
}

fn project(point: Point, axis: Point) -> f64 {
    point.x * axis.x + point.y * axis.y
}

fn unproject(along: f64, across: f64, direction: Point, normal: Point) -> Point {
    Point::new(
        along * direction.x + across * normal.x,
        along * direction.y + across * normal.y,
    )
}

/// Largest hole count accepted by [`validate_fill_region`].
const MAX_HOLES: usize = 32;
/// Largest vertex count of one ring after a duplicate closing point is dropped.
const MAX_RING_POINTS: usize = 4_096;
/// Largest vertex count of the outer ring and its holes together.
const MAX_REGION_POINTS: usize = 8_192;
/// Commands, including jumps, allowed in one holed fill before generation stops.
const MAX_REGION_COMMANDS: usize = 200_000;
/// A point this close to a ring is on that ring for admission.
const REGION_EPS_MM: f64 = 1e-6;
/// Sewing samples within this distance of a boundary stay on the boundary.
const SEW_TOLERANCE_MM: f64 = 1e-4;

/// Fill one outer ring and zero or more holes.
///
/// An empty `holes` list calls [`generate_fill`] and does not add checks.
/// Supported topology is one simple ring, plus simple holes that lie strictly
/// inside it and do not touch, overlap, or contain one another. Needle-down
/// fill and perpendicular underlay stay in the closure of that region.
/// Travel across a hole is a jump. Rejection leaves `plan` unchanged.
///
/// Admission treats a point within 0.000001 mm of a ring as on that ring.
/// Sewing samples within 0.0001 mm of a boundary may lie on it. Endpoints may
/// lie on the permitted boundary. The open segment must not enter a hole or
/// leave the outer ring.
pub fn generate_fill_with_holes(
    plan: &mut StitchPlan,
    object_id: u64,
    color_index: usize,
    polygon: &[Point],
    holes: &[Vec<Point>],
    config: &FillConfig,
) -> Result<(), PlanError> {
    if holes.is_empty() {
        return generate_fill(plan, object_id, color_index, polygon, config);
    }
    validate_fill_region(polygon, holes)?;
    validate_config(config)?;
    let outer = open_ring(polygon);
    let opened: Vec<Vec<Point>> = holes.iter().map(|hole| open_ring(hole)).collect();
    let mut rings: Vec<&[Point]> = Vec::with_capacity(opened.len() + 1);
    rings.push(&outer);
    for hole in &opened {
        rings.push(hole);
    }

    let mut batch = Vec::new();
    let plan_pen = plan.stitches.last().map(|stitch| stitch.to);
    if config.underlay_spacing_mm > 0.0 {
        sew_region(
            &mut batch,
            plan_pen,
            &rings,
            &RowPass {
                angle_degrees: config.angle_degrees + 90.0,
                spacing_mm: config.underlay_spacing_mm,
                max_stitch_mm: config.max_stitch_length_mm,
                kind: StitchKind::Running,
                color_index,
                object_id,
            },
        )?;
    }
    sew_region(
        &mut batch,
        plan_pen,
        &rings,
        &RowPass {
            angle_degrees: config.angle_degrees,
            spacing_mm: config.row_spacing_mm,
            max_stitch_mm: config.max_stitch_length_mm,
            kind: StitchKind::Fill,
            color_index,
            object_id,
        },
    )?;
    if !batch.iter().any(|stitch| stitch.kind == StitchKind::Fill) {
        return Err(PlanError::new(
            "Fill produced no stitches. The row spacing is larger than the shape, or the outline is degenerate. Tighten the outline or reduce row_spacing_mm.",
        ));
    }
    ensure_sewing_contained(&batch, &outer, &opened)?;
    commit(plan, batch)
}

/// Admit one outer ring and its holes, or explain the first topology failure.
///
/// This does not sew and does not modify a stitch plan. The same limits and
/// topology rules apply during generation.
pub fn validate_fill_region(outer: &[Point], holes: &[Vec<Point>]) -> Result<(), PlanError> {
    if holes.len() > MAX_HOLES {
        return Err(PlanError::new(format!(
            "This fill has {} holes. This increment allows at most {MAX_HOLES}. Remove some holes and generate again. The plan was not changed.",
            holes.len()
        )));
    }
    let outer_ring = open_ring(outer);
    if outer_ring.len() > MAX_RING_POINTS {
        return Err(PlanError::new(format!(
            "The outer outline has {} points. Simplify it to at most {MAX_RING_POINTS} points and generate again. The plan was not changed.",
            outer_ring.len()
        )));
    }
    let mut total = outer_ring.len();
    let mut opened = Vec::with_capacity(holes.len());
    for (index, hole) in holes.iter().enumerate() {
        let ring = open_ring(hole);
        if ring.len() > MAX_RING_POINTS {
            return Err(PlanError::new(format!(
                "Hole {} has {} points. Simplify it to at most {MAX_RING_POINTS} points and generate again. The plan was not changed.",
                index + 1,
                ring.len()
            )));
        }
        total += ring.len();
        opened.push(ring);
    }
    if total > MAX_REGION_POINTS {
        return Err(PlanError::new(format!(
            "The outline and holes together have {total} points. Simplify them below {MAX_REGION_POINTS} points and generate again. The plan was not changed."
        )));
    }
    check_simple(&outer_ring, "The outer outline")?;
    for (index, ring) in opened.iter().enumerate() {
        check_simple(ring, &format!("Hole {}", index + 1))?;
    }
    for (index, ring) in opened.iter().enumerate() {
        ensure_hole_inside(&outer_ring, ring, index + 1)?;
    }
    for left in 0..opened.len() {
        for right in (left + 1)..opened.len() {
            ensure_holes_separate(&opened[left], &opened[right], left + 1, right + 1)?;
        }
    }
    Ok(())
}

fn check_simple(ring: &[Point], label: &str) -> Result<(), PlanError> {
    ensure_finite(ring, label)?;
    if ring.len() < 3 {
        return Err(PlanError::new(format!(
            "{label} needs at least three distinct points. Close the ring and generate again. The plan was not changed."
        )));
    }
    let count = ring.len();
    let mut area = 0.0;
    for index in 0..count {
        let next = (index + 1) % count;
        if ring[index].distance(ring[next]) < 1e-6 {
            return Err(PlanError::new(format!(
                "{label} has a zero-length edge. Remove the duplicate point and generate again. The plan was not changed."
            )));
        }
        area += ring[index].x * ring[next].y - ring[next].x * ring[index].y;
        for other in (index + 1)..count {
            let other_next = (other + 1) % count;
            if next == other || other_next == index {
                continue;
            }
            let (a, b, c, d) = (ring[index], ring[next], ring[other], ring[other_next]);
            if segments_cross(a, b, c, d) || segments_touch(a, b, c, d) {
                return Err(PlanError::new(format!(
                    "{label} crosses itself. Repair that ring and generate again. The plan was not changed."
                )));
            }
        }
    }
    if area.abs() < 1e-6 {
        return Err(PlanError::new(format!(
            "{label} has zero area. Draw a real opening and generate again. The plan was not changed."
        )));
    }
    Ok(())
}

fn ensure_hole_inside(outer: &[Point], hole: &[Point], index: usize) -> Result<(), PlanError> {
    for point in hole {
        match containment(outer, *point) {
            Containment::Outside => {
                return Err(PlanError::new(format!(
                    "Hole {index} is not strictly inside the outer boundary. Move it fully inside, leaving a gap, and generate again. The plan was not changed."
                )));
            }
            Containment::Boundary => {
                return Err(PlanError::new(format!(
                    "Hole {index} touches the outer boundary. This increment needs a gap around every hole. Move it inward and generate again. The plan was not changed."
                )));
            }
            Containment::Inside => {}
        }
    }
    if let Some(crossed) = ring_pair_fault(hole, outer) {
        return Err(PlanError::new(match crossed {
            PairFault::Cross => format!(
                "Hole {index} crosses the outer boundary. Move it fully inside, leaving a gap, and generate again. The plan was not changed."
            ),
            PairFault::Touch => format!(
                "Hole {index} touches the outer boundary. This increment needs a gap around every hole. Move it inward and generate again. The plan was not changed."
            ),
        }));
    }
    Ok(())
}

fn ensure_holes_separate(
    left: &[Point],
    right: &[Point],
    left_index: usize,
    right_index: usize,
) -> Result<(), PlanError> {
    if let Some(fault) = ring_pair_fault(left, right) {
        return Err(PlanError::new(match fault {
            PairFault::Cross => format!(
                "Hole {left_index} overlaps hole {right_index}. Separate them so their boundaries do not meet, and generate again. The plan was not changed."
            ),
            PairFault::Touch => format!(
                "Hole {left_index} touches hole {right_index}. Leave a gap between holes and generate again. The plan was not changed."
            ),
        }));
    }
    if hole_inside(left, right) {
        return Err(PlanError::new(format!(
            "Hole {left_index} sits inside hole {right_index}. This increment does not sew a hole inside a hole. Remove the inner ring and generate again. The plan was not changed."
        )));
    }
    if hole_inside(right, left) {
        return Err(PlanError::new(format!(
            "Hole {right_index} sits inside hole {left_index}. This increment does not sew a hole inside a hole. Remove the inner ring and generate again. The plan was not changed."
        )));
    }
    Ok(())
}

fn hole_inside(inner: &[Point], outer: &[Point]) -> bool {
    inner
        .iter()
        .any(|point| matches!(containment(outer, *point), Containment::Inside))
}

enum PairFault {
    Cross,
    Touch,
}

fn ring_pair_fault(left: &[Point], right: &[Point]) -> Option<PairFault> {
    let left_count = left.len();
    let right_count = right.len();
    for i in 0..left_count {
        let a = left[i];
        let b = left[(i + 1) % left_count];
        for j in 0..right_count {
            let c = right[j];
            let d = right[(j + 1) % right_count];
            if segments_cross(a, b, c, d) {
                return Some(PairFault::Cross);
            }
            if segments_touch(a, b, c, d) {
                return Some(PairFault::Touch);
            }
        }
    }
    None
}

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn on_segment(a: Point, b: Point, c: Point) -> bool {
    c.x >= a.x.min(b.x) - 1e-9
        && c.x <= a.x.max(b.x) + 1e-9
        && c.y >= a.y.min(b.y) - 1e-9
        && c.y <= a.y.max(b.y) + 1e-9
}

fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);
    o1 * o2 < 0.0 && o3 * o4 < 0.0
}

fn segments_touch(a: Point, b: Point, c: Point, d: Point) -> bool {
    if segments_cross(a, b, c, d) {
        return false;
    }
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);
    (o1.abs() < 1e-9 && on_segment(a, b, c))
        || (o2.abs() < 1e-9 && on_segment(a, b, d))
        || (o3.abs() < 1e-9 && on_segment(c, d, a))
        || (o4.abs() < 1e-9 && on_segment(c, d, b))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Containment {
    Outside,
    Boundary,
    Inside,
}

fn containment(ring: &[Point], point: Point) -> Containment {
    if distance_to_ring(ring, point) <= REGION_EPS_MM {
        return Containment::Boundary;
    }
    let mut inside = false;
    let count = ring.len();
    for index in 0..count {
        let start = ring[index];
        let end = ring[(index + 1) % count];
        let crosses = (start.y > point.y) != (end.y > point.y)
            && point.x < (end.x - start.x) * (point.y - start.y) / (end.y - start.y) + start.x;
        if crosses {
            inside = !inside;
        }
    }
    if inside {
        Containment::Inside
    } else {
        Containment::Outside
    }
}

fn distance_to_ring(ring: &[Point], point: Point) -> f64 {
    let count = ring.len();
    let mut best = f64::INFINITY;
    for index in 0..count {
        best = best.min(distance_to_segment(
            point,
            ring[index],
            ring[(index + 1) % count],
        ));
    }
    best
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len2 = dx * dx + dy * dy;
    if len2 <= 1e-24 {
        return point.distance(start);
    }
    let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    point.distance(Point::new(start.x + dx * t, start.y + dy * t))
}

fn sew_region(
    batch: &mut Vec<Stitch>,
    plan_pen: Option<Point>,
    rings: &[&[Point]],
    pass: &RowPass,
) -> Result<(), PlanError> {
    let theta = pass.angle_degrees.to_radians();
    let direction = Point::new(theta.cos(), theta.sin());
    let normal = Point::new(-direction.y, direction.x);
    let outer = rings[0];
    let mut min_across = f64::INFINITY;
    let mut max_across = f64::NEG_INFINITY;
    for point in outer {
        let across = project(*point, normal);
        min_across = min_across.min(across);
        max_across = max_across.max(across);
    }
    if !min_across.is_finite() || max_across - min_across <= 1e-9 {
        return Ok(());
    }

    let mut reverse = false;
    let mut index = 0usize;
    loop {
        let across = min_across + pass.spacing_mm * (index as f64 + 0.5);
        if across >= max_across - 1e-9 {
            break;
        }
        index += 1;
        if index > 1_000_000 {
            return Err(PlanError::new(
                "Fill would create more than 1000000 rows. Increase row_spacing_mm or split the outline.",
            ));
        }
        let mut along = region_crossings(rings, across, normal, direction)?;
        if along.len() < 2 {
            reverse = !reverse;
            continue;
        }
        along.sort_by(|left, right| left.total_cmp(right));
        let mut spans = Vec::new();
        for pair in along.chunks(2) {
            if pair.len() < 2 || (pair[1] - pair[0]).abs() <= 1e-6 {
                continue;
            }
            spans.push(Span {
                start: unproject(pair[0], across, direction, normal),
                end: unproject(pair[1], across, direction, normal),
            });
        }
        if reverse {
            spans.reverse();
            for span in &mut spans {
                std::mem::swap(&mut span.start, &mut span.end);
            }
        }
        for span in spans {
            let pieces = span.start.distance(span.end) / pass.max_stitch_mm;
            if !pieces.is_finite() || pieces > 100_000.0 {
                return Err(PlanError::new(
                    "One fill span would be split into more than 100000 stitches. Increase max_stitch_length_mm and generate again. The plan was not changed.",
                ));
            }
            let line = resample_polyline(&[span.start, span.end], pass.max_stitch_mm);
            push_run(
                batch,
                plan_pen,
                &line,
                pass.kind,
                pass.color_index,
                pass.object_id,
            );
            if batch.len() > MAX_REGION_COMMANDS {
                return Err(PlanError::new(format!(
                    "Fill would create more than {MAX_REGION_COMMANDS} commands. Increase row spacing or stitch length and generate again. The plan was not changed."
                )));
            }
        }
        reverse = !reverse;
    }
    Ok(())
}

fn region_crossings(
    rings: &[&[Point]],
    across: f64,
    normal: Point,
    direction: Point,
) -> Result<Vec<f64>, PlanError> {
    let mut along = Vec::new();
    for ring in rings {
        along.extend(ring_hits(ring, across, normal, direction));
    }
    // A hole edge parallel to the row is invisible to the half-open crossing
    // test. When that edge lies on the row, its endpoints are the gap.
    for ring in rings.iter().skip(1) {
        along.extend(collinear_hole_hits(ring, across, normal, direction, &along));
    }
    if along.len() % 2 == 1 {
        return Err(PlanError::new(format!(
            "Fill row at {across:.3} mm crossed the region {} times. The outline or a hole is not a simple loop. Simplify it and fill again. The plan was not changed.",
            along.len()
        )));
    }
    Ok(along)
}

fn ring_hits(ring: &[Point], across: f64, normal: Point, direction: Point) -> Vec<f64> {
    let mut along = Vec::new();
    let count = ring.len();
    for index in 0..count {
        let p0 = ring[index];
        let p1 = ring[(index + 1) % count];
        let d0 = project(p0, normal);
        let d1 = project(p1, normal);
        if (d0 - d1).abs() <= 1e-12 {
            continue;
        }
        let (start, end, low, high) = if d0 < d1 {
            (p0, p1, d0, d1)
        } else {
            (p1, p0, d1, d0)
        };
        if across >= low && across < high {
            let t = (across - low) / (high - low);
            along.push(project(start.lerp(end, t), direction));
        }
    }
    along
}

fn collinear_hole_hits(
    ring: &[Point],
    across: f64,
    normal: Point,
    direction: Point,
    existing: &[f64],
) -> Vec<f64> {
    let mut extra = Vec::new();
    let count = ring.len();
    for index in 0..count {
        let start = ring[index];
        let end = ring[(index + 1) % count];
        let low = project(start, normal);
        let high = project(end, normal);
        if (low - high).abs() > 1e-9 || (low - across).abs() > 1e-9 {
            continue;
        }
        let left = project(start, direction);
        let right = project(end, direction);
        let present = |value: f64| existing.iter().any(|hit| (hit - value).abs() <= 1e-6);
        if !present(left) && !present(right) {
            extra.push(left);
            extra.push(right);
        }
    }
    extra
}

fn ensure_sewing_contained(
    batch: &[Stitch],
    outer: &[Point],
    holes: &[Vec<Point>],
) -> Result<(), PlanError> {
    for stitch in batch {
        if !stitch.kind.is_sewing() {
            continue;
        }
        if segment_excluded(stitch.from, stitch.to, outer, holes) {
            return Err(PlanError::new(format!(
                "A needle-down segment from ({:.2}, {:.2}) to ({:.2}, {:.2}) mm leaves the sewable region or crosses a hole. Widen the gap or simplify the outline and generate again. The plan was not changed.",
                stitch.from.x, stitch.from.y, stitch.to.x, stitch.to.y
            )));
        }
    }
    Ok(())
}

fn segment_excluded(from: Point, to: Point, outer: &[Point], holes: &[Vec<Point>]) -> bool {
    for step in 1..8 {
        let point = from.lerp(to, step as f64 / 8.0);
        if strictly_outside(outer, point) {
            return true;
        }
        if holes.iter().any(|hole| strictly_inside(hole, point)) {
            return true;
        }
    }
    let rings = std::iter::once(outer).chain(holes.iter().map(Vec::as_slice));
    for ring in rings {
        let count = ring.len();
        for index in 0..count {
            if interior_crossing(from, to, ring[index], ring[(index + 1) % count]) {
                return true;
            }
        }
    }
    false
}

fn strictly_outside(ring: &[Point], point: Point) -> bool {
    distance_to_ring(ring, point) > SEW_TOLERANCE_MM
        && matches!(containment(ring, point), Containment::Outside)
}

fn strictly_inside(ring: &[Point], point: Point) -> bool {
    distance_to_ring(ring, point) > SEW_TOLERANCE_MM
        && matches!(containment(ring, point), Containment::Inside)
}

/// Proper crossing of the stitch interior, away from its endpoints.
fn interior_crossing(from: Point, to: Point, start: Point, end: Point) -> bool {
    let rx = to.x - from.x;
    let ry = to.y - from.y;
    let sx = end.x - start.x;
    let sy = end.y - start.y;
    let denom = rx * sy - ry * sx;
    if denom.abs() <= 1e-12 {
        return false;
    }
    let qpx = start.x - from.x;
    let qpy = start.y - from.y;
    let t = (qpx * sy - qpy * sx) / denom;
    let u = (qpx * ry - qpy * rx) / denom;
    (0.02..0.98).contains(&t) && (0.02..0.98).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StitchPlan;

    fn square() -> Vec<Point> {
        vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ]
    }

    #[test]
    fn horizontal_fill_covers_inset_rows_and_alternates() {
        let mut plan = StitchPlan::new("fill");
        generate_fill(
            &mut plan,
            4,
            2,
            &square(),
            &FillConfig {
                angle_degrees: 0.0,
                row_spacing_mm: 2.0,
                max_stitch_length_mm: 5.0,
                underlay_spacing_mm: 0.0,
            },
        )
        .unwrap();

        let fills: Vec<_> = plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Fill)
            .collect();
        assert_eq!(fills.len(), 10);
        assert!(fills.iter().all(|stitch| {
            let mid = stitch.from.lerp(stitch.to, 0.5);
            mid.x > -1e-6 && mid.x < 10.0 + 1e-6 && mid.y > 0.0 && mid.y < 10.0
        }));
        assert_eq!(
            plan.stitches
                .iter()
                .filter(|stitch| stitch.kind == StitchKind::Jump)
                .count(),
            4
        );
        assert!(plan
            .stitches
            .windows(2)
            .all(|pair| pair[0].to == pair[1].from));
    }

    #[test]
    fn notch_is_left_unfilled() {
        let notch = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 4.0),
            Point::new(4.0, 4.0),
            Point::new(4.0, 6.0),
            Point::new(10.0, 6.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let mut plan = StitchPlan::new("notch");
        generate_fill(
            &mut plan,
            1,
            0,
            &notch,
            &FillConfig {
                angle_degrees: 0.0,
                row_spacing_mm: 10.0,
                max_stitch_length_mm: 12.0,
                underlay_spacing_mm: 0.0,
            },
        )
        .unwrap();
        let fills: Vec<_> = plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Fill)
            .collect();
        assert_eq!(fills.len(), 1);
        let stitch = fills[0];
        let left = stitch.from.x.min(stitch.to.x);
        let right = stitch.from.x.max(stitch.to.x);
        assert!((left - 0.0).abs() < 1e-6);
        assert!((right - 4.0).abs() < 1e-6);
        assert!((stitch.from.y - 5.0).abs() < 1e-6);
    }

    #[test]
    fn open_duplicate_endpoint_is_accepted() {
        let mut polygon = square();
        polygon.push(Point::new(0.0, 0.0));
        let mut plan = StitchPlan::new("closed");
        generate_fill(
            &mut plan,
            1,
            0,
            &polygon,
            &FillConfig {
                angle_degrees: 0.0,
                row_spacing_mm: 5.0,
                max_stitch_length_mm: 12.0,
                underlay_spacing_mm: 0.0,
            },
        )
        .unwrap();
        assert!(plan.sewing_stitch_count() > 0);
    }

    fn config_of(fixture: &FillFixture) -> FillConfig {
        FillConfig {
            angle_degrees: fixture.angle_degrees,
            row_spacing_mm: fixture.row_spacing_mm,
            max_stitch_length_mm: fixture.max_stitch_length_mm,
            underlay_spacing_mm: fixture.underlay_spacing_mm,
        }
    }

    fn sew(fixture: &FillFixture) -> Result<StitchPlan, crate::error::PlanError> {
        let mut plan = StitchPlan::new(&fixture.name);
        generate_fill_with_holes(
            &mut plan,
            1,
            0,
            &fixture.outer,
            &fixture.holes,
            &config_of(fixture),
        )?;
        Ok(plan)
    }

    #[test]
    fn empty_holes_match_the_no_hole_generator() {
        let polygon = square();
        let config = FillConfig {
            angle_degrees: 0.0,
            row_spacing_mm: 2.0,
            max_stitch_length_mm: 5.0,
            underlay_spacing_mm: 0.0,
        };
        let mut direct = StitchPlan::new("fill");
        let mut wrapped = StitchPlan::new("fill");
        generate_fill(&mut direct, 4, 2, &polygon, &config).unwrap();
        generate_fill_with_holes(&mut wrapped, 4, 2, &polygon, &[], &config).unwrap();
        assert_eq!(direct.stitches, wrapped.stitches);
    }

    #[test]
    fn declared_hole_fixtures_cover_valid_and_rejected_topology() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/fill-holes");
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|_| panic!("missing {}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        files.sort();
        assert!(
            files.len() >= 12,
            "expected at least 12 hole fixtures, found {}",
            files.len()
        );
        let mut accepted = 0usize;
        let mut rejected = 0usize;
        for path in files {
            let fixture: FillFixture =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let mut seeded = StitchPlan::new("seed");
            seeded.stitches.push(crate::model::Stitch {
                kind: StitchKind::Jump,
                from: Point::new(0.0, 0.0),
                to: Point::new(0.0, 0.0),
                color_index: 0,
                object_id: 9,
            });
            let before = seeded.clone();
            if fixture.expect == "ok" {
                let first = sew(&fixture)
                    .unwrap_or_else(|error| panic!("{} should sew: {error}", fixture.name));
                let second = sew(&fixture).unwrap();
                assert_eq!(first.stitches, second.stitches, "{}", fixture.name);
                assert!(
                    first
                        .stitches
                        .iter()
                        .any(|stitch| stitch.kind == StitchKind::Fill),
                    "{}",
                    fixture.name
                );
                assert_sewing_avoids_holes(&first, &fixture);
                assert!(
                    jump_crosses_a_hole(&first, &fixture),
                    "{} should jump across a hole",
                    fixture.name
                );
                accepted += 1;
            } else {
                let error = generate_fill_with_holes(
                    &mut seeded,
                    1,
                    0,
                    &fixture.outer,
                    &fixture.holes,
                    &config_of(&fixture),
                )
                .unwrap_err();
                assert!(
                    error.message().contains(&fixture.reject_contains),
                    "{} message {:?} does not contain {}",
                    fixture.name,
                    error.message(),
                    fixture.reject_contains
                );
                assert_eq!(seeded, before, "{}", fixture.name);
                rejected += 1;
            }
        }
        assert!(accepted >= 8, "accepted fixtures {accepted}");
        assert!(rejected >= 4, "rejected fixtures {rejected}");
    }

    #[test]
    fn limits_stop_unbounded_geometry_without_changing_the_plan() {
        let mut plan = StitchPlan::new("limits");
        let outer: Vec<Point> = (0..4_097)
            .map(|index| Point::new(index as f64, 0.0))
            .collect();
        let hole = vec![
            Point::new(1.0, 1.0),
            Point::new(2.0, 1.0),
            Point::new(1.5, 2.0),
        ];
        let error = generate_fill_with_holes(
            &mut plan,
            1,
            0,
            &outer,
            &[hole],
            &FillConfig {
                angle_degrees: 0.0,
                row_spacing_mm: 1.0,
                max_stitch_length_mm: 5.0,
                underlay_spacing_mm: 0.0,
            },
        )
        .unwrap_err();
        assert!(error.message().contains("4096"));
        assert!(plan.stitches.is_empty());

        let tiny = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.5, 1.0),
        ];
        let many = vec![tiny; 33];
        let error = validate_fill_region(&square(), &many).unwrap_err();
        assert!(error.message().contains("32"));

        let mut dense = StitchPlan::new("dense");
        let error = generate_fill_with_holes(
            &mut dense,
            1,
            0,
            &[
                Point::new(0.0, 0.0),
                Point::new(50.0, 0.0),
                Point::new(50.0, 50.0),
                Point::new(0.0, 50.0),
            ],
            &[vec![
                Point::new(20.0, 20.0),
                Point::new(30.0, 20.0),
                Point::new(30.0, 30.0),
                Point::new(20.0, 30.0),
            ]],
            &FillConfig {
                angle_degrees: 0.0,
                row_spacing_mm: 0.02,
                max_stitch_length_mm: 0.4,
                underlay_spacing_mm: 0.0,
            },
        )
        .unwrap_err();
        assert!(error.message().contains("200000"), "{error}");
        assert!(dense.stitches.is_empty());
    }

    fn assert_sewing_avoids_holes(plan: &StitchPlan, fixture: &FillFixture) {
        let outer = opened(&fixture.outer);
        let holes: Vec<Vec<Point>> = fixture.holes.iter().map(|hole| opened(hole)).collect();
        for stitch in &plan.stitches {
            if !stitch.kind.is_sewing() {
                continue;
            }
            for step in 1..8 {
                let point = stitch.from.lerp(stitch.to, step as f64 / 8.0);
                assert!(
                    !probe_outside(&outer, point),
                    "{} sewing leaves the outline at {:?}",
                    fixture.name,
                    point
                );
                for hole in &holes {
                    assert!(
                        !probe_inside(hole, point),
                        "{} {:?} sewing enters a hole at {:?}",
                        fixture.name,
                        stitch.kind,
                        point
                    );
                }
            }
        }
    }

    fn jump_crosses_a_hole(plan: &StitchPlan, fixture: &FillFixture) -> bool {
        let holes: Vec<Vec<Point>> = fixture.holes.iter().map(|hole| opened(hole)).collect();
        plan.stitches.iter().any(|stitch| {
            stitch.kind == StitchKind::Jump
                && holes
                    .iter()
                    .any(|hole| probe_inside(hole, stitch.from.lerp(stitch.to, 0.5)))
        })
    }

    fn opened(points: &[Point]) -> Vec<Point> {
        let mut ring = points.to_vec();
        if ring.len() >= 2 && ring[0].distance(*ring.last().unwrap()) <= 1e-9 {
            ring.pop();
        }
        ring
    }

    fn probe_inside(ring: &[Point], point: Point) -> bool {
        probe_distance(ring, point) > 1e-4 && probe_ray(ring, point)
    }

    fn probe_outside(ring: &[Point], point: Point) -> bool {
        probe_distance(ring, point) > 1e-4 && !probe_ray(ring, point)
    }

    fn probe_ray(ring: &[Point], point: Point) -> bool {
        let mut inside = false;
        for index in 0..ring.len() {
            let start = ring[index];
            let end = ring[(index + 1) % ring.len()];
            if (start.y > point.y) != (end.y > point.y)
                && point.x < (end.x - start.x) * (point.y - start.y) / (end.y - start.y) + start.x
            {
                inside = !inside;
            }
        }
        inside
    }

    fn probe_distance(ring: &[Point], point: Point) -> f64 {
        let mut best = f64::INFINITY;
        for index in 0..ring.len() {
            let start = ring[index];
            let end = ring[(index + 1) % ring.len()];
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let len2 = dx * dx + dy * dy;
            let t = if len2 <= 1e-24 {
                0.0
            } else {
                ((point.x - start.x) * dx + (point.y - start.y) * dy) / len2
            };
            let t = t.clamp(0.0, 1.0);
            best = best.min(point.distance(Point::new(start.x + dx * t, start.y + dy * t)));
        }
        best
    }
}

#[cfg(test)]
#[derive(Debug, serde::Deserialize)]
struct FillFixture {
    name: String,
    expect: String,
    outer: Vec<Point>,
    holes: Vec<Vec<Point>>,
    angle_degrees: f64,
    row_spacing_mm: f64,
    max_stitch_length_mm: f64,
    underlay_spacing_mm: f64,
    #[serde(default)]
    reject_contains: String,
}
