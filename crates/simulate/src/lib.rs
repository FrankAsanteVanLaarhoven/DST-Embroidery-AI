//! Production-risk scoring.
//!
//! This is not a fabric simulator. It measures stitch length per area, flags
//! sewing moves past one DST record, and flags jumps long enough to deserve a
//! trim. Physical stitch-out is still required before any quality claim.

use std::collections::HashMap;

use embroidery_core::{Point, StitchKind, StitchPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskFlag {
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskReport {
    pub flags: Vec<RiskFlag>,
    /// Highest millimetres of sewing stitch observed in one cell, per mm².
    pub max_length_density: f64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulateConfig {
    pub cell_mm: f64,
    /// Sewing length per square millimetre that counts as a dense pile-up.
    /// A single 0.4 mm fill is about 2.5. The default of 12 is several layers.
    pub max_length_density: f64,
    pub max_stitch_mm: f64,
    pub max_jump_mm: f64,
}

impl Default for SimulateConfig {
    fn default() -> Self {
        Self {
            cell_mm: 5.0,
            max_length_density: 12.0,
            max_stitch_mm: 12.1,
            max_jump_mm: 500.0,
        }
    }
}

const MAX_FLAGS: usize = 24;

pub fn score(plan: &StitchPlan) -> RiskReport {
    score_with(plan, &SimulateConfig::default())
}

pub fn score_with(plan: &StitchPlan, config: &SimulateConfig) -> RiskReport {
    let mut flags = Vec::new();
    if !config.cell_mm.is_finite() || config.cell_mm <= 0.0 {
        flags.push(flag(
            Severity::Error,
            "bad_cell",
            "cell_mm must be a positive cell size in millimetres.",
        ));
        return RiskReport {
            flags,
            max_length_density: 0.0,
            truncated: false,
        };
    }

    if plan.stitches.is_empty() {
        flags.push(flag(
            Severity::Error,
            "empty_plan",
            "Stitch plan contains no stitches. Generate an object before scoring production risk.",
        ));
        return RiskReport {
            flags,
            max_length_density: 0.0,
            truncated: false,
        };
    }

    let mut grid: HashMap<(i32, i32), f64> = HashMap::new();
    for (index, stitch) in plan.stitches.iter().enumerate() {
        if !stitch.from.is_finite() || !stitch.to.is_finite() {
            push_flag(
                &mut flags,
                flag(
                    Severity::Error,
                    "non_finite",
                    format!("Stitch {index} has a non-finite coordinate. Regenerate that object."),
                ),
            );
            continue;
        }
        let length = stitch.from.distance(stitch.to);
        if stitch.kind.is_sewing() {
            if length > config.max_stitch_mm {
                push_flag(
                    &mut flags,
                    flag(
                        Severity::Error,
                        "long_stitch",
                        format!(
                            "Stitch {index} is {length:.2} mm, past the {max:.2} mm sewing limit. Resample it before sewing.",
                            max = config.max_stitch_mm
                        ),
                    ),
                );
            }
            accumulate(&mut grid, stitch.from, stitch.to, length, config.cell_mm);
        } else if stitch.kind == StitchKind::Jump && length > config.max_jump_mm {
            push_flag(
                &mut flags,
                flag(
                    Severity::Warning,
                    "long_jump",
                    format!(
                        "Jump {index} is {length:.1} mm. Add a trim before this travel or split the object."
                    ),
                ),
            );
        }
    }

    let area = config.cell_mm * config.cell_mm;
    let mut ranked: Vec<((i32, i32), f64)> = grid.into_iter().collect();
    ranked.sort_by(|left, right| right.1.total_cmp(&left.1));
    let max_length_density = ranked
        .first()
        .map(|(_, length)| length / area)
        .unwrap_or(0.0);
    let mut truncated = false;
    for ((ix, iy), length) in &ranked {
        let density = length / area;
        if density <= config.max_length_density {
            break;
        }
        if flags.len() >= MAX_FLAGS {
            truncated = true;
            break;
        }
        let x0 = *ix as f64 * config.cell_mm;
        let y0 = *iy as f64 * config.cell_mm;
        flags.push(flag(
            Severity::Warning,
            "high_density",
            format!(
                "Cell at ({x0:.1}, {y0:.1}) mm holds {density:.1} mm of stitch per mm². Loosen the fill or satin density in that area."
            ),
        ));
    }
    if truncated {
        flags.push(flag(
            Severity::Warning,
            "truncated",
            "Density findings were truncated at 24 flags. Inspect the design in smaller regions.",
        ));
    }

    RiskReport {
        flags,
        max_length_density,
        truncated,
    }
}

fn accumulate(
    grid: &mut HashMap<(i32, i32), f64>,
    from: Point,
    to: Point,
    length: f64,
    cell_mm: f64,
) {
    if length == 0.0 {
        *grid.entry(cell_of(from, cell_mm)).or_insert(0.0) += 0.0;
        return;
    }
    let steps = ((length / (cell_mm * 0.5)).ceil() as usize).clamp(1, 10_000);
    let share = length / steps as f64;
    for step in 0..steps {
        let t = (step as f64 + 0.5) / steps as f64;
        let point = from.lerp(to, t);
        *grid.entry(cell_of(point, cell_mm)).or_insert(0.0) += share;
    }
}

fn cell_of(point: Point, cell_mm: f64) -> (i32, i32) {
    (
        (point.x / cell_mm).floor() as i32,
        (point.y / cell_mm).floor() as i32,
    )
}

fn flag(severity: Severity, code: &str, message: impl Into<String>) -> RiskFlag {
    RiskFlag {
        severity,
        code: code.to_string(),
        message: message.into(),
    }
}

fn push_flag(flags: &mut Vec<RiskFlag>, flag: RiskFlag) {
    if flags.len() < MAX_FLAGS {
        flags.push(flag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embroidery_core::{Point, Stitch, StitchKind, StitchPlan};

    fn sew(from: (f64, f64), to: (f64, f64)) -> Stitch {
        Stitch {
            kind: StitchKind::Running,
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            color_index: 0,
            object_id: 1,
        }
    }

    #[test]
    fn a_normal_stitch_is_not_dense() {
        let mut plan = StitchPlan::new("one");
        plan.stitches.push(sew((0.0, 0.0), (10.0, 0.0)));
        let report = score(&plan);
        assert!(report.flags.is_empty());
        assert!(report.max_length_density < 1.0);
    }

    #[test]
    fn piled_stitches_are_flagged() {
        let mut plan = StitchPlan::new("pile");
        for _ in 0..400 {
            plan.stitches.push(sew((1.0, 1.0), (2.0, 1.0)));
        }
        let report = score(&plan);
        assert!(report.flags.iter().any(|flag| flag.code == "high_density"));
        assert!(report.max_length_density > 12.0);
    }

    #[test]
    fn a_stitch_past_one_dst_record_is_an_error() {
        let mut plan = StitchPlan::new("long");
        plan.stitches.push(sew((0.0, 0.0), (20.0, 0.0)));
        let report = score(&plan);
        assert!(report
            .flags
            .iter()
            .any(|flag| { flag.code == "long_stitch" && flag.severity == Severity::Error }));
    }
}
