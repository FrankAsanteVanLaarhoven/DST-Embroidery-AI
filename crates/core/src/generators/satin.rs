use crate::error::PlanError;
use crate::geometry::{polyline_length, resample_polyline, sample_polyline};
use crate::model::{
    mm_to_units, Point, Stitch, StitchKind, StitchPlan, DST_MAX_DELTA_UNITS, DST_MAX_STITCH_MM,
};

/// Axis span at which a variable-satin sewing movement is split.
///
/// This is the implemented split policy, not a manufacturing limit. The
/// constant-width generator does not use it: an over-wide constant rung is
/// rejected. Encoded DST records are checked separately at 12.1 mm per axis.
const SPLIT_AXIS_MM: f64 = 12.0;
const MAX_SPLIT_PIECES: usize = 16;
const MIN_SEWING_MM: f64 = 0.2;
/// Bisector length below this is a reversal the miter does not interpret.
const MIN_BISECTOR: f64 = 0.25;

use super::{commit, ensure_finite, push, push_run, travel};

/// Column satin parameters.
///
/// `density_mm` is the distance along the centreline between needle stations.
/// `pull_compensation_mm` pushes each rail outward from the nominal half-width.
/// `centre_run_mm` and `edge_run_mm` select those passes. `None` skips that
/// pass. Both `Some` values, including two equal lengths, keep the historical
/// order: centre run, then the left edge forward, then the right edge reversed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SatinConfig {
    pub max_column_width_mm: f64,
    pub density_mm: f64,
    pub pull_compensation_mm: f64,
    pub centre_run_mm: Option<f64>,
    pub edge_run_mm: Option<f64>,
}

impl Default for SatinConfig {
    fn default() -> Self {
        Self {
            max_column_width_mm: 8.0,
            density_mm: 0.4,
            pull_compensation_mm: 0.15,
            centre_run_mm: Some(2.5),
            edge_run_mm: Some(2.5),
        }
    }
}

/// Append the selected underlay passes and a zigzag column.
///
/// Rails come from the arc-length normal at each station, so compensation is
/// applied at the matching point on the centreline. This is not yet variable
/// width, short-stitch corner handling, or a clipper offset. On failure the
/// plan is left as it was.
pub fn generate_satin(
    plan: &mut StitchPlan,
    object_id: u64,
    color_index: usize,
    centerline: &[Point],
    config: &SatinConfig,
) -> Result<(), PlanError> {
    if centerline.len() < 2 {
        return Err(PlanError::new(
            "Satin needs at least two centreline points. Extend the centreline and try again.",
        ));
    }
    if !config.max_column_width_mm.is_finite() || config.max_column_width_mm <= 0.0 {
        return Err(PlanError::new(
            "Satin max_column_width_mm must be a positive width in millimetres.",
        ));
    }
    if !config.density_mm.is_finite() || config.density_mm <= 0.0 {
        return Err(PlanError::new(
            "Satin density_mm must be a positive distance between column stations.",
        ));
    }
    if !config.pull_compensation_mm.is_finite() || config.pull_compensation_mm < 0.0 {
        return Err(PlanError::new(
            "Satin pull_compensation_mm cannot be negative. Use 0 to disable pull compensation.",
        ));
    }
    validate_run_lengths(config)?;
    ensure_finite(centerline, "Satin centreline")?;

    let length = polyline_length(centerline);
    if length / config.density_mm > 1_000_000.0 {
        return Err(PlanError::new(
            "Satin centreline is too long for this density. Increase density_mm or split the column.",
        ));
    }

    let samples = sample_polyline(centerline, config.density_mm);
    if samples.len() < 2 {
        return Err(PlanError::new(
            "Satin centreline has no length. Separate the centreline points and try again.",
        ));
    }

    let half = config.max_column_width_mm / 2.0;
    let satin_half = half + config.pull_compensation_mm;
    let left: Vec<Point> = samples
        .iter()
        .map(|sample| shift(sample.point, sample.normal, satin_half))
        .collect();
    let right: Vec<Point> = samples
        .iter()
        .map(|sample| shift(sample.point, sample.normal, -satin_half))
        .collect();

    let mut batch = Vec::new();
    let mut stats = MoveStats::default();
    let plan_pen = plan.stitches.last().map(|stitch| stitch.to);
    let nominal_left: Vec<Point> = samples
        .iter()
        .map(|sample| shift(sample.point, sample.normal, half))
        .collect();
    let nominal_right: Vec<Point> = samples
        .iter()
        .map(|sample| shift(sample.point, sample.normal, -half))
        .collect();
    sew_selected_underlay(
        &mut batch,
        &mut stats,
        plan_pen,
        UnderlayPaths {
            centerline,
            nominal_left: &nominal_left,
            nominal_right: &nominal_right,
        },
        config,
        color_index,
        object_id,
    )?;

    let top_at = batch.len();
    travel(&mut batch, plan_pen, left[0], color_index, object_id);
    for index in 0..left.len() {
        if index > 0 {
            push(
                &mut batch,
                StitchKind::Satin,
                right[index - 1],
                left[index],
                color_index,
                object_id,
            );
        }
        push(
            &mut batch,
            StitchKind::Satin,
            left[index],
            right[index],
            color_index,
            object_id,
        );
    }
    note_added(&batch, top_at, "top", &mut stats);

    if !batch.iter().any(|stitch| stitch.kind == StitchKind::Satin) {
        return Err(PlanError::new(
            "Satin generation produced no column stitches. Check the centreline and try again.",
        ));
    }
    let sequence = top_sequence(&batch);
    commit(plan, batch)?;
    record_phases(plan, object_id, &stats, &sequence, false);
    Ok(())
}

/// Column satin whose width is given at each centreline point.
///
/// Empty widths are not accepted here. Call [`generate_satin`] for the
/// constant-width column, which still rejects a rung past one DST record.
/// This path splits an over-wide sewing movement at 12 mm on each axis, limits
/// a sharp corner miter to twice the half-width, and omits a satin segment
/// shorter than 0.2 mm. A folded, reversed, or crossing column is rejected
/// before any batch is committed. The artwork is not repaired. Centre-run and
/// edge-run underlay are selected independently. Both passes keep the historical
/// traversal. This is not a new inset and it is not a clipper offset.
pub fn generate_satin_variable(
    plan: &mut StitchPlan,
    object_id: u64,
    color_index: usize,
    centerline: &[Point],
    widths_mm: &[f64],
    config: &SatinConfig,
) -> Result<(), PlanError> {
    if centerline.len() < 2 {
        return Err(PlanError::new(
            "Satin needs at least two centreline points. Extend the centreline and try again.",
        ));
    }
    if widths_mm.len() != centerline.len() {
        return Err(PlanError::new(
            "Satin needs one column width for each centreline point. Match the widths to the points and generate again. The plan was not changed.",
        ));
    }
    if !config.density_mm.is_finite() || config.density_mm <= 0.0 {
        return Err(PlanError::new(
            "Satin density_mm must be a positive distance between column stations.",
        ));
    }
    if !config.pull_compensation_mm.is_finite() || config.pull_compensation_mm < 0.0 {
        return Err(PlanError::new(
            "Satin pull_compensation_mm cannot be negative. Use 0 to disable pull compensation.",
        ));
    }
    validate_run_lengths(config)?;
    ensure_finite(centerline, "Satin centreline")?;
    for (index, width) in widths_mm.iter().enumerate() {
        if !width.is_finite() || *width <= 0.0 || *width > 48.0 {
            return Err(PlanError::new(format!(
                "Satin width {} must be greater than 0 mm and at most 48 mm. Reduce that width and generate again. The plan was not changed.",
                index + 1
            )));
        }
    }
    for window in centerline.windows(2) {
        if window[0].distance(window[1]) <= 1e-9 {
            return Err(PlanError::new(
                "Satin centreline repeats a point. Separate the points so each width has its own station, then generate again. The plan was not changed.",
            ));
        }
    }
    let length = polyline_length(centerline);
    if length / config.density_mm > 1_000_000.0 {
        return Err(PlanError::new(
            "Satin centreline is too long for this density. Increase density_mm or split the column.",
        ));
    }
    let samples = sample_polyline(centerline, config.density_mm);
    if samples.len() < 2 {
        return Err(PlanError::new(
            "Satin centreline has no length. Separate the centreline points and try again.",
        ));
    }

    admit_centreline(object_id, centerline)?;
    let intervals = samples.len() - 1;
    let sample_widths: Vec<f64> = (0..samples.len())
        .map(|index| {
            let distance = length * index as f64 / intervals as f64;
            width_at_distance(centerline, widths_mm, distance)
        })
        .collect();
    let nominal = column_stations(object_id, centerline, &samples, &sample_widths, 0.0)?;
    admit_column(object_id, &nominal, "nominal column")?;
    let compensated = column_stations(
        object_id,
        centerline,
        &samples,
        &sample_widths,
        config.pull_compensation_mm,
    )?;
    admit_column(object_id, &compensated, "column after pull compensation")?;
    let satin_left: Vec<Point> = compensated.iter().map(|station| station.left).collect();
    let satin_right: Vec<Point> = compensated.iter().map(|station| station.right).collect();

    let mut batch = Vec::new();
    let mut stats = MoveStats::default();
    let plan_pen = plan.stitches.last().map(|stitch| stitch.to);
    let nominal_left: Vec<Point> = nominal.iter().map(|station| station.left).collect();
    let nominal_right: Vec<Point> = nominal.iter().map(|station| station.right).collect();
    sew_selected_underlay(
        &mut batch,
        &mut stats,
        plan_pen,
        UnderlayPaths {
            centerline,
            nominal_left: &nominal_left,
            nominal_right: &nominal_right,
        },
        config,
        color_index,
        object_id,
    )?;

    let mut pen = satin_left[0];
    let mut sewed_rung = false;
    let top_at = batch.len();
    travel(&mut batch, plan_pen, pen, color_index, object_id);
    for index in 0..satin_left.len() {
        if index > 0 {
            pen = push_limited(
                &mut batch,
                pen,
                satin_left[index],
                color_index,
                object_id,
                &mut stats,
            )?;
        }
        let before = batch.len();
        pen = push_limited(
            &mut batch,
            pen,
            satin_right[index],
            color_index,
            object_id,
            &mut stats,
        )?;
        sewed_rung |= batch.len() > before;
    }
    note_added(&batch, top_at, "top", &mut stats);
    if !sewed_rung {
        return Err(PlanError::new(
            "Satin generation produced no column stitches. Widen the column above 0.2 mm and try again. The plan was not changed.",
        ));
    }
    admit_movements(object_id, &batch)?;
    let sequence = top_sequence(&batch);
    commit(plan, batch)?;
    record_phases(plan, object_id, &stats, &sequence, true);
    Ok(())
}

fn width_at_distance(centerline: &[Point], widths_mm: &[f64], distance: f64) -> f64 {
    let mut walked = 0.0;
    for index in 0..centerline.len() - 1 {
        let span = centerline[index].distance(centerline[index + 1]);
        if walked + span >= distance - 1e-6 || index + 2 == centerline.len() {
            let t = if span <= 1e-9 {
                0.0
            } else {
                ((distance - walked) / span).clamp(0.0, 1.0)
            };
            return widths_mm[index] * (1.0 - t) + widths_mm[index + 1] * t;
        }
        walked += span;
    }
    widths_mm[widths_mm.len() - 1]
}

fn rail_offset(
    centerline: &[Point],
    sample: &crate::geometry::PolylineSample,
    half: f64,
) -> Result<Point, usize> {
    for index in 1..centerline.len() - 1 {
        if sample.point.distance(centerline[index]) > 1e-4 {
            continue;
        }
        let incoming = unit_left(
            centerline[index].x - centerline[index - 1].x,
            centerline[index].y - centerline[index - 1].y,
        );
        let outgoing = unit_left(
            centerline[index + 1].x - centerline[index].x,
            centerline[index + 1].y - centerline[index].y,
        );
        return corner_offset(incoming, outgoing, half).ok_or(index + 1);
    }
    Ok(Point {
        x: sample.normal.x * half,
        y: sample.normal.y * half,
    })
}

/// Miter toward the left bisector. A collapsed bisector is rejected.
/// The outgoing normal is not substituted, because that would reshape the column.
fn corner_offset(incoming: Point, outgoing: Point, half: f64) -> Option<Point> {
    let dot = (incoming.x * outgoing.x + incoming.y * outgoing.y).clamp(-1.0, 1.0);
    let bx = incoming.x + outgoing.x;
    let by = incoming.y + outgoing.y;
    let length = (bx * bx + by * by).sqrt();
    if length < MIN_BISECTOR {
        return None;
    }
    let sin_half = ((1.0 - dot) / 2.0).sqrt();
    let scale = if sin_half < 1e-3 {
        1.0
    } else {
        (1.0 / sin_half).min(2.0)
    };
    Some(Point {
        x: bx / length * half * scale,
        y: by / length * half * scale,
    })
}

fn unit_left(dx: f64, dy: f64) -> Point {
    let norm = (dx * dx + dy * dy).sqrt();
    if norm <= 1e-12 {
        return Point { x: 0.0, y: 0.0 };
    }
    Point {
        x: -dy / norm,
        y: dx / norm,
    }
}

fn place(point: Point, offset: Point) -> Point {
    Point {
        x: point.x + offset.x,
        y: point.y + offset.y,
    }
}

#[derive(Default)]
struct MoveStats {
    top: usize,
    centre: usize,
    edge: usize,
    connectors: usize,
    split_pieces: usize,
    omitted: usize,
}

struct Station {
    left: Point,
    right: Point,
    /// Direction from the centreline toward the left rail, before width.
    direction: Point,
}

fn satin_reject(object_id: u64, section: usize, problem: &str) -> PlanError {
    PlanError::new(format!(
        "Object {object_id}, section {section}: {problem} Move the centreline, reduce the width, or split this into separate columns. The plan was not changed."
    ))
}

fn admit_centreline(object_id: u64, centerline: &[Point]) -> Result<(), PlanError> {
    for (index, window) in centerline.windows(3).enumerate() {
        let incoming = unit_left(window[1].x - window[0].x, window[1].y - window[0].y);
        let outgoing = unit_left(window[2].x - window[1].x, window[2].y - window[1].y);
        let length = (incoming.x + outgoing.x).hypot(incoming.y + outgoing.y);
        if length < MIN_BISECTOR {
            return Err(satin_reject(
                object_id,
                index + 2,
                "the centreline reverses or folds too sharply to place a column.",
            ));
        }
    }
    for index in 0..centerline.len().saturating_sub(1) {
        let end = index + 1;
        for later in (index + 2)..centerline.len().saturating_sub(1) {
            if segments_conflict(
                centerline[index],
                centerline[end],
                centerline[later],
                centerline[later + 1],
            ) {
                return Err(satin_reject(
                    object_id,
                    index + 1,
                    "the centreline crosses itself.",
                ));
            }
        }
    }
    Ok(())
}

fn column_stations(
    object_id: u64,
    centerline: &[Point],
    samples: &[crate::geometry::PolylineSample],
    widths: &[f64],
    pull: f64,
) -> Result<Vec<Station>, PlanError> {
    samples
        .iter()
        .zip(widths.iter())
        .map(|(sample, width)| {
            let half = width / 2.0 + pull;
            let direction = rail_offset(centerline, sample, 1.0).map_err(|section| {
                satin_reject(
                    object_id,
                    section,
                    "the centreline reverses or folds too sharply to place a column.",
                )
            })?;
            Ok(Station {
                left: place(sample.point, scale(direction, half)),
                right: place(sample.point, scale(direction, -half)),
                direction,
            })
        })
        .collect()
}

fn admit_column(object_id: u64, stations: &[Station], label: &str) -> Result<(), PlanError> {
    for (index, station) in stations.iter().enumerate() {
        let across = Point {
            x: station.left.x - station.right.x,
            y: station.left.y - station.right.y,
        };
        let dot = across.x * station.direction.x + across.y * station.direction.y;
        if dot <= 1e-4 {
            return Err(satin_reject(
                object_id,
                index + 1,
                &format!("the {label} is inverted or pinched."),
            ));
        }
    }
    let mut segments = Vec::new();
    for index in 0..stations.len().saturating_sub(1) {
        segments.push((index + 1, stations[index].left, stations[index + 1].left));
        segments.push((index + 1, stations[index].right, stations[index + 1].right));
    }
    for (index, station) in stations.iter().enumerate() {
        segments.push((index + 1, station.left, station.right));
    }
    for index in 0..segments.len() {
        for later in index + 1..segments.len() {
            if segments_conflict(
                segments[index].1,
                segments[index].2,
                segments[later].1,
                segments[later].2,
            ) {
                return Err(satin_reject(
                    object_id,
                    segments[index].0,
                    &format!("the {label} rails cross or overlap."),
                ));
            }
        }
    }
    Ok(())
}

fn segments_conflict(a: Point, b: Point, c: Point, d: Point) -> bool {
    if a.distance(b) <= 1e-6 || c.distance(d) <= 1e-6 {
        return false;
    }
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    if o1.abs() < 1e-6 && o2.abs() < 1e-6 && o3.abs() < 1e-6 && o4.abs() < 1e-6 {
        return collinear_overlap(a, b, c, d);
    }
    interior_hit(a, b, c) || interior_hit(a, b, d) || interior_hit(c, d, a) || interior_hit(c, d, b)
}

fn orient(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn interior_hit(a: Point, b: Point, point: Point) -> bool {
    if point.distance(a) <= 1e-4 || point.distance(b) <= 1e-4 {
        return false;
    }
    orient(a, b, point).abs() <= 1e-6 && on_segment(a, b, point)
}

fn on_segment(a: Point, b: Point, point: Point) -> bool {
    point.x >= a.x.min(b.x) - 1e-6
        && point.x <= a.x.max(b.x) + 1e-6
        && point.y >= a.y.min(b.y) - 1e-6
        && point.y <= a.y.max(b.y) + 1e-6
}

fn collinear_overlap(a: Point, b: Point, c: Point, d: Point) -> bool {
    let use_x = (a.x - b.x).abs() >= (a.y - b.y).abs();
    let (a1, a2) = if use_x {
        (a.x.min(b.x), a.x.max(b.x))
    } else {
        (a.y.min(b.y), a.y.max(b.y))
    };
    let (b1, b2) = if use_x {
        (c.x.min(d.x), c.x.max(d.x))
    } else {
        (c.y.min(d.y), c.y.max(d.y))
    };
    a2.min(b2) - a1.max(b1) > 1e-4
}

fn validate_run_lengths(config: &SatinConfig) -> Result<(), PlanError> {
    for (label, length) in [
        ("centre-run", config.centre_run_mm),
        ("edge-run", config.edge_run_mm),
    ] {
        let Some(length) = length else {
            continue;
        };
        if !length.is_finite() || length <= 0.0 {
            return Err(PlanError::new(format!(
                "Satin {label} maximum stitch length must be positive. Leave that pass off to skip it, then generate again. The plan was not changed."
            )));
        }
        if length > DST_MAX_STITCH_MM {
            return Err(PlanError::new(format!(
                "Satin {label} maximum stitch length is longer than one DST record ({DST_MAX_STITCH_MM} mm per axis). Reduce that length and generate again. The plan was not changed."
            )));
        }
    }
    Ok(())
}

struct UnderlayPaths<'a> {
    centerline: &'a [Point],
    nominal_left: &'a [Point],
    nominal_right: &'a [Point],
}

fn sew_selected_underlay(
    batch: &mut Vec<Stitch>,
    stats: &mut MoveStats,
    plan_pen: Option<Point>,
    paths: UnderlayPaths<'_>,
    config: &SatinConfig,
    color_index: usize,
    object_id: u64,
) -> Result<(), PlanError> {
    if let Some(length) = config.centre_run_mm {
        let centre = resample_polyline(paths.centerline, length);
        let start = batch.len();
        append_run(batch, plan_pen, &centre, length, color_index, object_id)?;
        note_added(batch, start, "centre", stats);
    }
    if let Some(length) = config.edge_run_mm {
        let left_run = resample_polyline(paths.nominal_left, length);
        let mut right_run = resample_polyline(paths.nominal_right, length);
        right_run.reverse();
        let start = batch.len();
        append_run(batch, plan_pen, &left_run, length, color_index, object_id)?;
        append_run(batch, plan_pen, &right_run, length, color_index, object_id)?;
        note_added(batch, start, "edge", stats);
    }
    Ok(())
}

fn append_run(
    batch: &mut Vec<Stitch>,
    plan_pen: Option<Point>,
    points: &[Point],
    max_mm: f64,
    color_index: usize,
    object_id: u64,
) -> Result<(), PlanError> {
    let start = batch.len();
    push_run(
        batch,
        plan_pen,
        points,
        StitchKind::Running,
        color_index,
        object_id,
    );
    for stitch in &batch[start..] {
        if stitch.kind != StitchKind::Running {
            continue;
        }
        let length = stitch.from.distance(stitch.to);
        if length > max_mm + 1e-4 {
            return Err(satin_reject(
                object_id,
                start + 1,
                "an underlay movement is longer than its stitch length. Reduce that length and generate again.",
            ));
        }
        let dx = mm_to_units(stitch.to.x)? - mm_to_units(stitch.from.x)?;
        let dy = mm_to_units(stitch.to.y)? - mm_to_units(stitch.from.y)?;
        if dx.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
            || dy.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
        {
            return Err(satin_reject(
                object_id,
                start + 1,
                "an underlay movement is longer than one DST record. Reduce that length and generate again.",
            ));
        }
    }
    Ok(())
}

fn note_added(batch: &[Stitch], start: usize, role: &str, stats: &mut MoveStats) {
    for stitch in &batch[start..] {
        match stitch.kind {
            StitchKind::Jump => stats.connectors += 1,
            StitchKind::Running if role == "centre" => stats.centre += 1,
            StitchKind::Running if role == "edge" => stats.edge += 1,
            StitchKind::Satin if role == "top" => stats.top += 1,
            _ => {}
        }
    }
}

fn top_sequence(batch: &[Stitch]) -> String {
    batch
        .iter()
        .filter(|stitch| stitch.kind == StitchKind::Satin)
        .map(|stitch| {
            format!(
                "{:.6},{:.6}>{:.6},{:.6}",
                stitch.from.x, stitch.from.y, stitch.to.x, stitch.to.y
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn admit_movements(object_id: u64, batch: &[Stitch]) -> Result<(), PlanError> {
    for (index, stitch) in batch.iter().enumerate() {
        if index > 0 {
            let gap = batch[index - 1].to.distance(stitch.from);
            if gap > 1e-4 {
                return Err(satin_reject(
                    object_id,
                    index + 1,
                    "a filtered movement left a gap, so the next command would not start at the needle.",
                ));
            }
        }
        if !stitch.kind.is_sewing() {
            continue;
        }
        let length = stitch.from.distance(stitch.to);
        let axis = (stitch.to.x - stitch.from.x)
            .abs()
            .max((stitch.to.y - stitch.from.y).abs());
        let dx = mm_to_units(stitch.to.x)? - mm_to_units(stitch.from.x)?;
        let dy = mm_to_units(stitch.to.y)? - mm_to_units(stitch.from.y)?;
        let past_record = dx.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
            || dy.unsigned_abs() > DST_MAX_DELTA_UNITS as u32;
        if stitch.kind == StitchKind::Running {
            if past_record {
                return Err(satin_reject(
                    object_id,
                    index + 1,
                    "an underlay movement is longer than one DST record. Reduce that length and generate again.",
                ));
            }
            continue;
        }
        if axis > SPLIT_AXIS_MM + 1e-6 || past_record {
            return Err(satin_reject(
                object_id,
                index + 1,
                "a sewing movement exceeds the 12 mm split policy or one DST record.",
            ));
        }
        if stitch.kind == StitchKind::Satin && length < MIN_SEWING_MM - 1e-9 {
            return Err(satin_reject(
                object_id,
                index + 1,
                "a satin movement shorter than 0.2 mm was emitted.",
            ));
        }
    }
    Ok(())
}

fn append_meta(plan: &mut StitchPlan, key: &str, line: &str) {
    let entry = plan.metadata.entry(key.to_string()).or_default();
    if !entry.is_empty() {
        entry.push(' ');
    }
    entry.push_str(line);
}

fn record_phases(
    plan: &mut StitchPlan,
    object_id: u64,
    stats: &MoveStats,
    sequence: &str,
    variable: bool,
) {
    if variable {
        let line = format!(
            "Object {object_id} variable satin uses the 12 mm axis split, at most {MAX_SPLIT_PIECES} pieces, and omits satin sewing shorter than 0.2 mm. Top {}, underlay {}, travel {}, split pieces {}, omitted {}.",
            stats.top,
            stats.centre + stats.edge,
            stats.connectors,
            stats.split_pieces,
            stats.omitted
        );
        append_meta(plan, "satin_movement_summary", &line);
    }
    let phases = format!(
        "Object {object_id}: Centre-run underlay {}. Edge-run underlay {}. Top satin {}. Positioning and inter-phase connectors {}.",
        stats.centre, stats.edge, stats.top, stats.connectors
    );
    append_meta(plan, "satin_phase_summary", &phases);
    append_meta(
        plan,
        "satin_top_sequence",
        &format!("object {object_id} {sequence}"),
    );
}

fn scale(point: Point, factor: f64) -> Point {
    Point {
        x: point.x * factor,
        y: point.y * factor,
    }
}

fn push_limited(
    batch: &mut Vec<Stitch>,
    from: Point,
    to: Point,
    color_index: usize,
    object_id: u64,
    stats: &mut MoveStats,
) -> Result<Point, PlanError> {
    let span = (to.x - from.x).abs().max((to.y - from.y).abs());
    if from.distance(to) < MIN_SEWING_MM {
        stats.omitted += 1;
        return Ok(from);
    }
    let pieces = (span / SPLIT_AXIS_MM).ceil() as usize;
    if pieces > MAX_SPLIT_PIECES {
        return Err(satin_reject(
            object_id,
            1,
            "a sewing movement needs more than 16 pieces to stay within 12 mm on each axis.",
        ));
    }
    let pieces = pieces.max(1);
    if pieces > 1 {
        stats.split_pieces += pieces;
    }
    let mut pen = from;
    for piece in 1..=pieces {
        let next = from.lerp(to, piece as f64 / pieces as f64);
        if pen.distance(next) < MIN_SEWING_MM {
            stats.omitted += 1;
            continue;
        }
        push(batch, StitchKind::Satin, pen, next, color_index, object_id);
        pen = next;
    }
    Ok(pen)
}

fn shift(point: Point, normal: Point, distance: f64) -> Point {
    Point {
        x: point.x + normal.x * distance,
        y: point.y + normal.y * distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StitchPlan;

    #[test]
    fn straight_column_alternates_and_stays_inside_the_width() {
        let mut plan = StitchPlan::new("satin");
        let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let config = SatinConfig {
            max_column_width_mm: 4.0,
            density_mm: 5.0,
            pull_compensation_mm: 0.0,
            centre_run_mm: None,
            edge_run_mm: None,
        };
        generate_satin(&mut plan, 3, 1, &line, &config).unwrap();

        let satin: Vec<_> = plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Satin)
            .collect();
        // Three stations produce five column stitches: L0 R0 L1 R1 L2 R2.
        assert_eq!(satin.len(), 5);
        assert!(plan
            .stitches
            .windows(2)
            .all(|pair| pair[0].to == pair[1].from));
        assert!(satin.iter().all(|stitch| {
            let length = stitch.from.distance(stitch.to);
            length <= 4.0_f64.hypot(5.0) + 1e-6
        }));
        assert!(satin
            .iter()
            .all(|stitch| stitch.color_index == 1 && stitch.object_id == 3));
        assert!(satin
            .iter()
            .all(|stitch| stitch.from.y.abs() <= 2.0 + 1e-6 && stitch.to.y.abs() <= 2.0 + 1e-6));
    }

    #[test]
    fn underlay_is_sewn_before_the_column() {
        let mut plan = StitchPlan::new("satin");
        generate_satin(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            &SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 5.0,
                pull_compensation_mm: 0.2,
                centre_run_mm: Some(5.0),
                edge_run_mm: Some(5.0),
            },
        )
        .unwrap();
        assert_eq!(plan.stitches[0].kind, StitchKind::Running);
        assert!(plan
            .stitches
            .iter()
            .any(|stitch| stitch.kind == StitchKind::Satin));
        let first_satin = plan
            .stitches
            .iter()
            .position(|stitch| stitch.kind == StitchKind::Satin)
            .unwrap();
        assert!(first_satin > 0);
    }

    #[test]
    fn a_column_wider_than_one_dst_record_is_rejected_without_writing() {
        let mut plan = StitchPlan::new("wide");
        let error = generate_satin(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 0.0), Point::new(5.0, 0.0)],
            &SatinConfig {
                max_column_width_mm: 20.0,
                density_mm: 2.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: None,
                edge_run_mm: None,
            },
        )
        .unwrap_err();
        assert!(error.message().contains("12.1"));
        assert!(plan.stitches.is_empty());
    }

    fn variable_config(underlay: f64) -> SatinConfig {
        SatinConfig {
            max_column_width_mm: 2.0,
            density_mm: 10.0,
            pull_compensation_mm: 0.0,
            centre_run_mm: (underlay > 0.0).then_some(underlay),
            edge_run_mm: (underlay > 0.0).then_some(underlay),
        }
    }

    #[test]
    fn variable_width_tapers_and_miters_a_square_corner() {
        let mut plan = StitchPlan::new("taper");
        let line = [
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 20.0),
        ];
        generate_satin_variable(
            &mut plan,
            4,
            2,
            &line,
            &[4.0, 4.0, 4.0],
            &variable_config(0.0),
        )
        .unwrap();
        let satin: Vec<_> = plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Satin)
            .collect();
        assert!(satin.len() >= 3);
        assert!(satin.iter().all(|stitch| {
            stitch.from.distance(stitch.to) >= 0.2 - 1e-9
                && (stitch.to.x - stitch.from.x).abs() <= 12.0 + 1e-6
                && (stitch.to.y - stitch.from.y).abs() <= 12.0 + 1e-6
        }));
        let corner = satin.iter().any(|stitch| {
            let points = [stitch.from, stitch.to];
            points
                .iter()
                .any(|point| (point.x - 8.0).abs() < 1e-6 && (point.y - 12.0).abs() < 1e-6)
                || points
                    .iter()
                    .any(|point| (point.x - 12.0).abs() < 1e-6 && (point.y - 8.0).abs() < 1e-6)
        });
        assert!(corner, "expected the 90 degree miter points");
    }

    #[test]
    fn an_over_wide_variable_rung_is_split_instead_of_rejected() {
        let mut plan = StitchPlan::new("split");
        generate_satin_variable(
            &mut plan,
            1,
            0,
            &[Point::new(10.0, 30.0), Point::new(20.0, 30.0)],
            &[20.0, 20.0],
            &variable_config(0.0),
        )
        .unwrap();
        let satin: Vec<_> = plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Satin)
            .collect();
        assert!(satin.len() >= 4, "{}", satin.len());
        assert!(satin.iter().all(|stitch| {
            (stitch.to.x - stitch.from.x).abs() <= 12.0 + 1e-6
                && (stitch.to.y - stitch.from.y).abs() <= 12.0 + 1e-6
        }));
        let mut constant = StitchPlan::new("constant");
        let error = generate_satin(
            &mut constant,
            1,
            0,
            &[Point::new(0.0, 0.0), Point::new(5.0, 0.0)],
            &SatinConfig {
                max_column_width_mm: 20.0,
                density_mm: 2.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: None,
                edge_run_mm: None,
            },
        )
        .unwrap_err();
        assert!(error.message().contains("12.1"));
        assert!(constant.stitches.is_empty());
    }

    #[test]
    fn a_rung_shorter_than_the_minimum_is_omitted() {
        let mut plan = StitchPlan::new("narrow");
        let error = generate_satin_variable(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            &[0.05, 0.05],
            &variable_config(0.0),
        )
        .unwrap_err();
        assert!(error.message().contains("0.2"));
        assert!(plan.stitches.is_empty());

        generate_satin_variable(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 10.0), Point::new(10.0, 10.0)],
            &[4.0, 0.05],
            &variable_config(0.0),
        )
        .unwrap();
        let satin: Vec<_> = plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Satin)
            .collect();
        assert!(!satin.is_empty());
        assert!(satin
            .iter()
            .all(|stitch| stitch.from.distance(stitch.to) >= 0.2 - 1e-9));
    }

    #[test]
    fn variable_underlay_still_precedes_the_column() {
        let mut plan = StitchPlan::new("under");
        generate_satin_variable(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 10.0), Point::new(10.0, 10.0)],
            &[2.0, 6.0],
            &variable_config(5.0),
        )
        .unwrap();
        assert_eq!(plan.stitches[0].kind, StitchKind::Running);
        assert!(plan
            .stitches
            .iter()
            .any(|stitch| stitch.kind == StitchKind::Satin));
    }

    #[test]
    fn mismatched_widths_do_not_write_a_plan() {
        let mut plan = StitchPlan::new("bad");
        let error = generate_satin_variable(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            &[2.0],
            &variable_config(0.0),
        )
        .unwrap_err();
        assert!(error.message().contains("each centreline point"));
        assert!(plan.stitches.is_empty());
    }

    #[test]
    fn omitted_short_rungs_keep_the_needle_and_record_the_policy() {
        let mut plan = StitchPlan::new("filter");
        generate_satin_variable(
            &mut plan,
            7,
            0,
            &[Point::new(0.0, 10.0), Point::new(10.0, 10.0)],
            &[4.0, 0.05],
            &variable_config(0.0),
        )
        .unwrap();
        assert!(plan
            .stitches
            .windows(2)
            .all(|pair| pair[0].to.distance(pair[1].from) <= 1e-6));
        let summary = plan.metadata.get("satin_movement_summary").unwrap();
        assert!(summary.contains("Object 7"));
        assert!(summary.contains("12 mm"));
        assert!(summary.contains("omitted "));
        assert!(!summary.contains("omitted 0"));
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn declared_satin_domain_accepts_or_rejects_without_partial_output() {
        let accepted = [
            (
                "straight",
                vec![(0.0, 10.0), (20.0, 10.0)],
                vec![4.0, 4.0],
                0.0,
                0.0,
            ),
            (
                "taper",
                vec![(0.0, 10.0), (20.0, 10.0)],
                vec![2.0, 8.0],
                0.0,
                0.0,
            ),
            (
                "bend",
                vec![(0.0, 10.0), (12.0, 10.0), (20.0, 16.0)],
                vec![3.0, 3.0, 3.0],
                0.0,
                0.0,
            ),
            (
                "square",
                vec![(0.0, 10.0), (10.0, 10.0), (10.0, 20.0)],
                vec![4.0, 4.0, 4.0],
                0.0,
                0.0,
            ),
            (
                "obtuse",
                vec![(0.0, 10.0), (10.0, 10.0), (15.0, 18.66)],
                vec![3.0, 3.0, 3.0],
                0.0,
                0.0,
            ),
            (
                "compensated",
                vec![(0.0, 20.0), (24.0, 20.0)],
                vec![4.0, 4.0],
                0.2,
                0.0,
            ),
            (
                "underlay",
                vec![(0.0, 20.0), (16.0, 20.0)],
                vec![2.0, 6.0],
                0.0,
                4.0,
            ),
            (
                "split",
                vec![(10.0, 30.0), (22.0, 30.0)],
                vec![20.0, 20.0],
                0.0,
                0.0,
            ),
            (
                "filtered-tip",
                vec![(0.0, 12.0), (12.0, 12.0)],
                vec![4.0, 0.05],
                0.0,
                0.0,
            ),
            (
                "gentle-s",
                vec![(0.0, 10.0), (10.0, 14.0), (20.0, 10.0), (30.0, 14.0)],
                vec![2.5, 2.5, 2.5, 2.5],
                0.0,
                0.0,
            ),
        ];
        for (name, points, widths, pull, underlay) in accepted {
            let mut plan = StitchPlan::new(name);
            let result = generate_satin_variable(
                &mut plan,
                1,
                0,
                &points
                    .iter()
                    .map(|(x, y)| Point::new(*x, *y))
                    .collect::<Vec<_>>(),
                &widths,
                &SatinConfig {
                    max_column_width_mm: 2.0,
                    density_mm: 4.0,
                    pull_compensation_mm: pull,
                    centre_run_mm: (underlay > 0.0).then_some(underlay),
                    edge_run_mm: (underlay > 0.0).then_some(underlay),
                },
            );
            assert!(result.is_ok(), "{name}: {result:?}");
            assert!(
                plan.stitches
                    .windows(2)
                    .all(|pair| pair[0].to.distance(pair[1].from) <= 1e-4),
                "{name} gap"
            );
            assert!(
                plan.metadata.contains_key("satin_movement_summary"),
                "{name} policy"
            );
        }

        let rejected: &[(&str, Vec<(f64, f64)>, Vec<f64>, f64, &str)] = &[
            (
                "repeat",
                vec![(0.0, 10.0), (0.0, 10.0)],
                vec![4.0, 4.0],
                0.0,
                "repeats a point",
            ),
            (
                "mismatch",
                vec![(0.0, 10.0), (10.0, 10.0)],
                vec![4.0],
                0.0,
                "each centreline point",
            ),
            (
                "zero",
                vec![(0.0, 10.0), (10.0, 10.0)],
                vec![0.0, 4.0],
                0.0,
                "greater than 0",
            ),
            (
                "wide",
                vec![(0.0, 10.0), (10.0, 10.0)],
                vec![49.0, 4.0],
                0.0,
                "at most 48",
            ),
            (
                "negative",
                vec![(0.0, 10.0), (10.0, 10.0)],
                vec![-1.0, 4.0],
                0.0,
                "greater than 0",
            ),
            (
                "reversal",
                vec![(0.0, 10.0), (12.0, 10.0), (0.0, 10.0)],
                vec![4.0, 4.0, 4.0],
                0.0,
                "reverses",
            ),
            (
                "hairpin",
                vec![(0.0, 10.0), (12.0, 10.0), (0.2, 10.4)],
                vec![3.0, 3.0, 3.0],
                0.0,
                "reverses",
            ),
            (
                "cross",
                vec![(0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)],
                vec![2.0, 2.0, 2.0, 2.0],
                0.0,
                "crosses itself",
            ),
            (
                "folded-corner",
                vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)],
                vec![12.0, 12.0, 12.0],
                0.0,
                "rails cross",
            ),
            (
                "overlap",
                vec![(0.0, 10.0), (20.0, 10.0), (20.0, 14.0), (0.0, 14.0)],
                vec![6.0, 6.0, 6.0, 6.0],
                0.0,
                "rails cross",
            ),
            (
                "compensation-fold",
                vec![(0.0, 10.0), (20.0, 10.0), (20.0, 14.0), (0.0, 14.0)],
                vec![3.0, 3.0, 3.0, 3.0],
                0.6,
                "pull compensation",
            ),
        ];
        let modes = [
            ("none", None, None),
            ("centre", Some(2.5), None),
            ("edge", None, Some(2.5)),
            ("both", Some(2.5), Some(2.5)),
        ];
        for (name, points, widths, pull, needle) in rejected {
            for (mode, centre, edge) in modes {
                let mut plan = StitchPlan::new(*name);
                let error = generate_satin_variable(
                    &mut plan,
                    9,
                    0,
                    &points
                        .iter()
                        .map(|(x, y)| Point::new(*x, *y))
                        .collect::<Vec<_>>(),
                    widths,
                    &SatinConfig {
                        max_column_width_mm: 2.0,
                        density_mm: 4.0,
                        pull_compensation_mm: *pull,
                        centre_run_mm: centre,
                        edge_run_mm: edge,
                    },
                )
                .unwrap_err();
                assert!(plan.stitches.is_empty(), "{name}/{mode} wrote stitches");
                assert!(
                    error.message().contains(needle),
                    "{name}/{mode}: {} does not contain {needle}",
                    error.message()
                );
                if error.message().contains("section") {
                    assert!(error.message().contains("Object 9"), "{name}/{mode}");
                    assert!(
                        error.message().contains("separate columns"),
                        "{name}/{mode}"
                    );
                }
            }
        }
    }

    fn phase_line(plan: &StitchPlan) -> String {
        plan.metadata
            .get("satin_phase_summary")
            .cloned()
            .unwrap_or_default()
    }

    fn top_line(plan: &StitchPlan) -> String {
        plan.metadata
            .get("satin_top_sequence")
            .cloned()
            .unwrap_or_default()
    }

    #[test]
    fn four_underlay_selections_keep_the_top_satin_sequence() {
        let modes = [
            (None, None),
            (Some(2.5), None),
            (None, Some(2.5)),
            (Some(2.5), Some(2.5)),
        ];
        let straight = [Point::new(0.0, 10.0), Point::new(20.0, 10.0)];
        let bend = [
            Point::new(0.0, 10.0),
            Point::new(12.0, 10.0),
            Point::new(20.0, 16.0),
        ];
        for (centre, edge) in modes {
            let config = SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: centre,
                edge_run_mm: edge,
            };
            let mut first = StitchPlan::new("constant");
            generate_satin(&mut first, 1, 0, &straight, &config).unwrap();
            let mut second = StitchPlan::new("constant");
            generate_satin(&mut second, 1, 0, &straight, &config).unwrap();
            assert_eq!(first.stitches, second.stitches);
            let mut tapered = StitchPlan::new("taper");
            generate_satin_variable(&mut tapered, 2, 0, &straight, &[2.0, 8.0], &config).unwrap();
            let mut tapered_again = StitchPlan::new("taper");
            generate_satin_variable(&mut tapered_again, 2, 0, &straight, &[2.0, 8.0], &config)
                .unwrap();
            assert_eq!(tapered.stitches, tapered_again.stitches);
            let mut bent = StitchPlan::new("bend");
            generate_satin_variable(&mut bent, 3, 0, &bend, &[3.0, 3.0, 3.0], &config).unwrap();
            let mut bent_again = StitchPlan::new("bend");
            generate_satin_variable(&mut bent_again, 3, 0, &bend, &[3.0, 3.0, 3.0], &config)
                .unwrap();
            assert_eq!(bent.stitches, bent_again.stitches);
            for plan in [&first, &tapered, &bent] {
                let phases = phase_line(plan);
                assert!(phases.contains("Centre-run underlay"));
                assert!(phases.contains("Edge-run underlay"));
                assert!(phases.contains("Top satin"));
                assert!(phases.contains("Positioning and inter-phase connectors"));
            }
        }
        let mut reference_tops = Vec::new();
        for (centre, edge) in modes {
            let config = SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: centre,
                edge_run_mm: edge,
            };
            let mut constant = StitchPlan::new("constant");
            generate_satin(&mut constant, 1, 0, &straight, &config).unwrap();
            let mut tapered = StitchPlan::new("taper");
            generate_satin_variable(&mut tapered, 2, 0, &straight, &[2.0, 8.0], &config).unwrap();
            let mut bent = StitchPlan::new("bend");
            generate_satin_variable(&mut bent, 3, 0, &bend, &[3.0, 3.0, 3.0], &config).unwrap();
            reference_tops.push((top_line(&constant), top_line(&tapered), top_line(&bent)));
            let phases = phase_line(&constant);
            if centre.is_some() {
                assert!(!phases.contains("Centre-run underlay 0"));
            } else {
                assert!(phases.contains("Centre-run underlay 0"));
            }
            if edge.is_some() {
                assert!(!phases.contains("Edge-run underlay 0"));
            } else {
                assert!(phases.contains("Edge-run underlay 0"));
            }
        }
        assert!(reference_tops.iter().all(|item| item == &reference_tops[0]));
        let mut bare = StitchPlan::new("bare");
        generate_satin(
            &mut bare,
            1,
            0,
            &straight,
            &SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: None,
                edge_run_mm: None,
            },
        )
        .unwrap();
        let mut covered = StitchPlan::new("covered");
        generate_satin(
            &mut covered,
            1,
            0,
            &straight,
            &SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: Some(2.5),
                edge_run_mm: Some(2.5),
            },
        )
        .unwrap();
        assert_ne!(bare.stitches, covered.stitches);
        assert_eq!(top_line(&bare), top_line(&covered));
    }

    #[test]
    fn rejection_preserves_a_prepopulated_plan_for_every_underlay() {
        let mut plan = StitchPlan::new("existing");
        crate::generate_running(
            &mut plan,
            4,
            0,
            &[Point::new(1.0, 1.0), Point::new(6.0, 1.0)],
            &crate::RunningConfig {
                max_stitch_length_mm: 2.5,
            },
        )
        .unwrap();
        let before = plan.stitches.clone();
        let metadata = plan.metadata.clone();
        let modes = [
            (None, None),
            (Some(2.5), None),
            (None, Some(2.5)),
            (Some(2.5), Some(2.5)),
        ];
        for (centre, edge) in modes {
            let error = generate_satin_variable(
                &mut plan,
                9,
                0,
                &[
                    Point::new(0.0, 10.0),
                    Point::new(12.0, 10.0),
                    Point::new(0.0, 10.0),
                ],
                &[4.0, 4.0, 4.0],
                &SatinConfig {
                    max_column_width_mm: 4.0,
                    density_mm: 4.0,
                    pull_compensation_mm: 0.0,
                    centre_run_mm: centre,
                    edge_run_mm: edge,
                },
            )
            .unwrap_err();
            assert!(error.message().contains("reverses"), "{}", error.message());
            assert_eq!(plan.stitches, before);
            assert_eq!(plan.metadata, metadata);
        }
    }

    #[test]
    fn a_short_underlay_stitch_is_kept_and_each_pass_uses_its_own_length() {
        let mut plan = StitchPlan::new("short-run");
        generate_satin_variable(
            &mut plan,
            5,
            0,
            &[Point::new(0.0, 10.0), Point::new(0.15, 10.0)],
            &[4.0, 4.0],
            &SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: Some(2.5),
                edge_run_mm: None,
            },
        )
        .unwrap();
        assert!(plan.stitches.iter().any(|stitch| {
            stitch.kind == StitchKind::Running && stitch.from.distance(stitch.to) < 0.2
        }));
        assert!(plan
            .stitches
            .windows(2)
            .all(|pair| pair[0].to.distance(pair[1].from) <= 1e-4));

        let mut split_lengths = StitchPlan::new("lengths");
        generate_satin_variable(
            &mut split_lengths,
            6,
            0,
            &[Point::new(0.0, 10.0), Point::new(20.0, 10.0)],
            &[4.0, 4.0],
            &SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: Some(5.0),
                edge_run_mm: Some(2.0),
            },
        )
        .unwrap();
        let running: Vec<_> = split_lengths
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Running)
            .collect();
        assert!(running
            .iter()
            .any(|stitch| stitch.from.distance(stitch.to) > 2.0 + 1e-4));
        assert!(running
            .iter()
            .all(|stitch| stitch.from.distance(stitch.to) <= 5.0 + 1e-4));

        let mut kept = StitchPlan::new("too-long");
        crate::generate_running(
            &mut kept,
            1,
            0,
            &[Point::new(2.0, 2.0), Point::new(4.0, 2.0)],
            &crate::RunningConfig {
                max_stitch_length_mm: 3.0,
            },
        )
        .unwrap();
        let before = kept.stitches.clone();
        let error = generate_satin(
            &mut kept,
            8,
            0,
            &[Point::new(0.0, 10.0), Point::new(20.0, 10.0)],
            &SatinConfig {
                max_column_width_mm: 4.0,
                density_mm: 4.0,
                pull_compensation_mm: 0.0,
                centre_run_mm: Some(15.0),
                edge_run_mm: None,
            },
        )
        .unwrap_err();
        assert!(
            error.message().contains("Reduce that length"),
            "{}",
            error.message()
        );
        assert_eq!(kept.stitches, before);
    }
}
