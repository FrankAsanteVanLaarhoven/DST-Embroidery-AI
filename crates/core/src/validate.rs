use crate::model::{mm_to_units, StitchKind, StitchPlan, DST_MAX_DELTA_UNITS};

/// Machine and quality findings for one plan.
///
/// `errors` block a strict DST compile. `warnings` describe travel the
/// compiler will insert and stitches that will collapse to one count.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Check a plan against sewing and jump limits.
///
/// Sewing stitches are also checked against the DST per-axis limit of ±121
/// counts, because that limit is a property of the file format. Jumps may be
/// longer: the compiler splits them without adding needle penetrations.
pub fn validate(plan: &StitchPlan, max_stitch_mm: f64, max_jump_mm: f64) -> ValidationReport {
    let mut report = ValidationReport {
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    if !max_stitch_mm.is_finite() || max_stitch_mm <= 0.0 {
        report.errors.push(
            "max_stitch_mm must be a positive length. Pass the sewing limit in millimetres.".into(),
        );
        return report;
    }
    if !max_jump_mm.is_finite() || max_jump_mm <= 0.0 {
        report.errors.push(
            "max_jump_mm must be a positive length. Pass the jump limit in millimetres.".into(),
        );
        return report;
    }

    if plan.stitches.is_empty() {
        report
            .errors
            .push("Stitch plan contains no stitches. Generate a running, satin, or fill object before compiling.".into());
        return report;
    }

    if let Some(first) = plan.stitches.first() {
        if first.from.distance(crate::model::Point::new(0.0, 0.0)) > 0.05 {
            report.warnings.push(format!(
                "Plan starts at ({:.2}, {:.2}) mm rather than the origin. The DST compiler will insert a jump from 0,0.",
                first.from.x, first.from.y
            ));
        }
    }

    for (index, stitch) in plan.stitches.iter().enumerate() {
        if !stitch.from.is_finite() || !stitch.to.is_finite() {
            report.errors.push(format!(
                "Stitch {index} has a non-finite coordinate. Remove it or regenerate that object."
            ));
            continue;
        }

        let length = stitch.from.distance(stitch.to);
        match stitch.kind {
            StitchKind::Running
            | StitchKind::Satin
            | StitchKind::Fill
            | StitchKind::Motif
            | StitchKind::Applique => {
                if length > max_stitch_mm {
                    report.errors.push(format!(
                        "Stitch {index}: sewing length {length:.2} mm exceeds {max_stitch_mm:.2} mm. Shorten it with the running, satin, or fill generator."
                    ));
                }
                match axis_delta(stitch.from, stitch.to) {
                    Ok((dx, dy)) => {
                        if dx.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
                            || dy.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
                        {
                            report.errors.push(format!(
                                "Stitch {index} compiles to DST delta ({dx}, {dy}) counts, past the ±{DST_MAX_DELTA_UNITS} record limit. Resample the stitch below 12.1 mm on each axis."
                            ));
                        }
                    }
                    Err(message) => report.errors.push(format!("Stitch {index}: {message}")),
                }
                if length > 0.0 && length < 0.1 {
                    report.warnings.push(format!(
                        "Stitch {index} is {length:.3} mm, shorter than one DST count (0.1 mm). Rounding may turn it into a stay stitch."
                    ));
                }
            }
            StitchKind::Jump => {
                if length > max_jump_mm {
                    report.errors.push(format!(
                        "Jump {index}: length {length:.2} mm exceeds {max_jump_mm:.2} mm. Split the travel or raise the jump limit."
                    ));
                }
                if length <= 0.05 {
                    report.warnings.push(format!(
                        "Jump {index} has no movement. Remove it or keep the previous stitch."
                    ));
                }
            }
            StitchKind::Trim | StitchKind::ColorChange => {}
        }

        if index > 0 {
            let previous = plan.stitches[index - 1].to;
            let gap = previous.distance(stitch.from);
            if gap > 0.05 {
                report.warnings.push(format!(
                    "Stitch {index} starts {gap:.2} mm away from the previous end. The DST compiler will insert a jump."
                ));
            }
        }
    }

    if plan.sewing_stitch_count() == 0 {
        report.errors.push(
            "No sewing stitches found. Add a running, satin, or fill object before sending this plan to a machine.".into(),
        );
    }

    report
}

fn axis_delta(from: crate::model::Point, to: crate::model::Point) -> Result<(i32, i32), String> {
    let x0 = mm_to_units(from.x).map_err(|error| error.message().to_string())?;
    let y0 = mm_to_units(from.y).map_err(|error| error.message().to_string())?;
    let x1 = mm_to_units(to.x).map_err(|error| error.message().to_string())?;
    let y1 = mm_to_units(to.y).map_err(|error| error.message().to_string())?;
    Ok((x1 - x0, y1 - y0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Point, Stitch, StitchKind, StitchPlan};

    fn running(from: (f64, f64), to: (f64, f64)) -> Stitch {
        Stitch {
            kind: StitchKind::Running,
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            color_index: 0,
            object_id: 1,
        }
    }

    #[test]
    fn rejects_empty_long_and_non_finite_stitches() {
        let empty = validate(&StitchPlan::new("empty"), 12.1, 100.0);
        assert!(!empty.ok());

        let mut plan = StitchPlan::new("bad");
        plan.stitches.push(running((0.0, 0.0), (20.0, 0.0)));
        plan.stitches.push(Stitch {
            kind: StitchKind::Running,
            from: Point::new(f64::NAN, 0.0),
            to: Point::new(1.0, 0.0),
            color_index: 0,
            object_id: 1,
        });
        let report = validate(&plan, 12.1, 100.0);
        assert!(report.errors.iter().any(|error| error.contains("20.00")));
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("non-finite")));
    }

    #[test]
    fn warns_when_the_pen_would_have_to_jump() {
        let mut plan = StitchPlan::new("gap");
        plan.stitches.push(running((0.0, 0.0), (5.0, 0.0)));
        plan.stitches.push(running((8.0, 0.0), (12.0, 0.0)));
        let report = validate(&plan, 12.1, 100.0);
        assert!(report.ok());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("insert a jump")));
    }
}
