use crate::error::PlanError;
use crate::geometry::{polyline_length, resample_polyline, sample_polyline};
use crate::model::{Point, StitchKind, StitchPlan};

use super::{commit, ensure_finite, push, push_run, travel};

/// Column satin parameters.
///
/// `density_mm` is the distance along the centreline between needle stations.
/// `pull_compensation_mm` pushes each rail outward from the nominal half-width.
/// A non-positive `underlay_running_stitch_mm` skips centre and edge underlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SatinConfig {
    pub max_column_width_mm: f64,
    pub density_mm: f64,
    pub pull_compensation_mm: f64,
    pub underlay_running_stitch_mm: f64,
}

impl Default for SatinConfig {
    fn default() -> Self {
        Self {
            max_column_width_mm: 8.0,
            density_mm: 0.4,
            pull_compensation_mm: 0.15,
            underlay_running_stitch_mm: 2.5,
        }
    }
}

/// Append a centre-run underlay, an edge-run underlay, and a zigzag column.
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
    if !config.underlay_running_stitch_mm.is_finite() || config.underlay_running_stitch_mm < 0.0 {
        return Err(PlanError::new(
            "Satin underlay_running_stitch_mm cannot be negative. Use 0 to skip underlay.",
        ));
    }
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
    let plan_pen = plan.stitches.last().map(|stitch| stitch.to);

    if config.underlay_running_stitch_mm > 0.0 {
        let centre = resample_polyline(centerline, config.underlay_running_stitch_mm);
        push_run(
            &mut batch,
            plan_pen,
            &centre,
            StitchKind::Running,
            color_index,
            object_id,
        );

        let nominal_left: Vec<Point> = samples
            .iter()
            .map(|sample| shift(sample.point, sample.normal, half))
            .collect();
        let nominal_right: Vec<Point> = samples
            .iter()
            .map(|sample| shift(sample.point, sample.normal, -half))
            .collect();
        let left_run = resample_polyline(&nominal_left, config.underlay_running_stitch_mm);
        let mut right_run = resample_polyline(&nominal_right, config.underlay_running_stitch_mm);
        right_run.reverse();
        push_run(
            &mut batch,
            plan_pen,
            &left_run,
            StitchKind::Running,
            color_index,
            object_id,
        );
        push_run(
            &mut batch,
            plan_pen,
            &right_run,
            StitchKind::Running,
            color_index,
            object_id,
        );
    }

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

    if !batch.iter().any(|stitch| stitch.kind == StitchKind::Satin) {
        return Err(PlanError::new(
            "Satin generation produced no column stitches. Check the centreline and try again.",
        ));
    }
    commit(plan, batch)
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
            underlay_running_stitch_mm: 0.0,
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
                underlay_running_stitch_mm: 5.0,
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
                underlay_running_stitch_mm: 0.0,
            },
        )
        .unwrap_err();
        assert!(error.message().contains("12.1"));
        assert!(plan.stitches.is_empty());
    }
}
