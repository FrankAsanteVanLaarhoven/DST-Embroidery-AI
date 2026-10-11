use crate::error::PlanError;
use crate::geometry::{polyline_length, resample_polyline};
use crate::model::{Point, StitchPlan};

use super::{commit, ensure_finite, push_run};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunningConfig {
    pub max_stitch_length_mm: f64,
}

impl Default for RunningConfig {
    fn default() -> Self {
        Self {
            max_stitch_length_mm: 2.5,
        }
    }
}

/// Append a running stitch along `path`.
///
/// Long spans are split so each sewing record stays within
/// `config.max_stitch_length_mm` and within one DST record. On failure the
/// plan is left as it was.
pub fn generate_running(
    plan: &mut StitchPlan,
    object_id: u64,
    color_index: usize,
    path: &[Point],
    config: &RunningConfig,
) -> Result<(), PlanError> {
    if path.len() < 2 {
        return Err(PlanError::new(
            "A running stitch needs at least two points. Add another point to the path.",
        ));
    }
    if !config.max_stitch_length_mm.is_finite() || config.max_stitch_length_mm <= 0.0 {
        return Err(PlanError::new(
            "Running max_stitch_length_mm must be a positive length in millimetres.",
        ));
    }
    ensure_finite(path, "Running path")?;
    let length = polyline_length(path);
    if length / config.max_stitch_length_mm > 1_000_000.0 {
        return Err(PlanError::new(
            "Running path is too long for this stitch length. Raise max_stitch_length_mm or split the path.",
        ));
    }

    let sampled = resample_polyline(path, config.max_stitch_length_mm);
    let mut batch = Vec::new();
    let plan_pen = plan.stitches.last().map(|stitch| stitch.to);
    push_run(
        &mut batch,
        plan_pen,
        &sampled,
        crate::model::StitchKind::Running,
        color_index,
        object_id,
    );
    if batch.iter().all(|stitch| !stitch.kind.is_sewing()) {
        return Err(PlanError::new(
            "Running path produced no sewing stitches. The points are coincident; separate them and try again.",
        ));
    }
    commit(plan, batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Point, StitchKind, StitchPlan};

    #[test]
    fn splits_a_long_span_and_keeps_the_bend() {
        let mut plan = StitchPlan::new("run");
        let path = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 5.0),
        ];
        generate_running(
            &mut plan,
            7,
            0,
            &path,
            &RunningConfig {
                max_stitch_length_mm: 4.0,
            },
        )
        .unwrap();
        assert!(plan.stitches.len() >= 4);
        assert!(plan
            .stitches
            .iter()
            .all(|stitch| stitch.kind == StitchKind::Running));
        assert!(plan
            .stitches
            .iter()
            .all(|stitch| stitch.from.distance(stitch.to) <= 4.0 + 1e-6));
        assert!(plan
            .stitches
            .windows(2)
            .all(|pair| pair[0].to == pair[1].from));
        assert_eq!(plan.stitches.last().unwrap().to, Point::new(10.0, 5.0));
    }

    #[test]
    fn failure_does_not_append() {
        let mut plan = StitchPlan::new("run");
        let error = generate_running(
            &mut plan,
            1,
            0,
            &[Point::new(0.0, 0.0)],
            &RunningConfig::default(),
        )
        .unwrap_err();
        assert!(error.message().contains("two points"));
        assert!(plan.stitches.is_empty());
    }
}
