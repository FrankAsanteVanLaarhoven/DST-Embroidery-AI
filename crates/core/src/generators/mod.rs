mod fill;
mod running;
mod satin;

pub use fill::{generate_fill, generate_fill_with_holes, validate_fill_region, FillConfig};
pub use running::{generate_running, RunningConfig};
pub use satin::{generate_satin, SatinConfig};

use crate::error::PlanError;
use crate::model::{mm_to_units, Point, Stitch, StitchKind, StitchPlan, DST_MAX_DELTA_UNITS};

/// Gaps at or below this length are already the same DST coordinate.
const CONNECT_MM: f64 = 0.05;

pub(crate) fn ensure_finite(points: &[Point], label: &str) -> Result<(), PlanError> {
    if points.iter().any(|point| !point.is_finite()) {
        return Err(PlanError::new(format!(
            "{label} contains a non-finite coordinate. Regenerate that outline and try again."
        )));
    }
    Ok(())
}

pub(crate) fn push(
    batch: &mut Vec<Stitch>,
    kind: StitchKind,
    from: Point,
    to: Point,
    color_index: usize,
    object_id: u64,
) {
    batch.push(Stitch {
        kind,
        from,
        to,
        color_index,
        object_id,
    });
}

pub(crate) fn pen(batch: &[Stitch], plan_pen: Option<Point>) -> Option<Point> {
    batch.last().map(|stitch| stitch.to).or(plan_pen)
}

pub(crate) fn travel(
    batch: &mut Vec<Stitch>,
    plan_pen: Option<Point>,
    to: Point,
    color_index: usize,
    object_id: u64,
) {
    let Some(from) = pen(batch, plan_pen) else {
        return;
    };
    if from.distance(to) > CONNECT_MM {
        push(batch, StitchKind::Jump, from, to, color_index, object_id);
    }
}

pub(crate) fn push_run(
    batch: &mut Vec<Stitch>,
    plan_pen: Option<Point>,
    points: &[Point],
    kind: StitchKind,
    color_index: usize,
    object_id: u64,
) {
    if points.len() < 2 {
        return;
    }
    travel(batch, plan_pen, points[0], color_index, object_id);
    for window in points.windows(2) {
        if window[0].distance(window[1]) <= 1e-9 {
            continue;
        }
        push(batch, kind, window[0], window[1], color_index, object_id);
    }
}

/// Keep `plan` unchanged when a generated sewing stitch cannot fit in one DST record.
pub(crate) fn commit(plan: &mut StitchPlan, batch: Vec<Stitch>) -> Result<(), PlanError> {
    for (index, stitch) in batch.iter().enumerate() {
        if !stitch.kind.is_sewing() {
            continue;
        }
        let dx = mm_to_units(stitch.to.x)? - mm_to_units(stitch.from.x)?;
        let dy = mm_to_units(stitch.to.y)? - mm_to_units(stitch.from.y)?;
        if dx.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
            || dy.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
        {
            return Err(PlanError::new(format!(
                "Generated stitch {index} is {dx} by {dy} DST counts, past the ±{DST_MAX_DELTA_UNITS} record limit (12.1 mm per axis). Reduce the stitch length, column width, or row spacing and generate again. The plan was not changed."
            )));
        }
    }
    plan.stitches.extend(batch);
    Ok(())
}
