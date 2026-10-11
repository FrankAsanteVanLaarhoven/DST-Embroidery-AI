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
/// first and last points differ. Holes, self-intersections, and curved
/// angle fields are not handled: an odd number of crossings on a row is an
/// error and the plan is not changed. Rows alternate direction to shorten travel.
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
}
