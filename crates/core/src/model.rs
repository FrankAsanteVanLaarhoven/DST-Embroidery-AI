use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::PlanError;

/// One DST count is 0.1 mm.
pub const DST_UNIT_MM: f64 = 0.1;

/// Multiply millimetres by this to get DST counts.
///
/// This is `10` exactly. Dividing by [`DST_UNIT_MM`] is not the same binary
/// operation, so conversion uses this constant.
pub const DST_UNITS_PER_MM: f64 = 10.0;

/// Largest absolute component of one DST record.
pub const DST_MAX_DELTA_UNITS: i32 = 121;

/// Header extent fields are five digits, so a coordinate must fit in this many counts.
pub const DST_MAX_ABS_UNITS: i32 = 99_999;

/// Longest single-record sewing move, 12.1 mm, when it lies on one axis.
pub const DST_MAX_STITCH_MM: f64 = DST_MAX_DELTA_UNITS as f64 * DST_UNIT_MM;

/// Design intent for one needle command.
///
/// DST itself only stores needle-down, jump, colour change, and end. The
/// extra kinds exist so generators can keep satin, fill, and trim distinct
/// until a compiler maps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StitchKind {
    Running,
    Satin,
    Fill,
    Motif,
    Applique,
    Jump,
    Trim,
    ColorChange,
}

impl StitchKind {
    /// Needle-down work. Jumps, trims, and colour changes are travel or control.
    pub fn is_sewing(self) -> bool {
        matches!(
            self,
            StitchKind::Running
                | StitchKind::Satin
                | StitchKind::Fill
                | StitchKind::Motif
                | StitchKind::Applique
        )
    }
}

/// A point in millimetres. Y grows downward, matching the pyembroidery pattern
/// space and typical design canvases. The DST file flips Y as it is written.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn lerp(self, other: Point, t: f64) -> Point {
        Point {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// One commanded move. `from` and `to` are absolute design coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stitch {
    pub kind: StitchKind,
    pub from: Point,
    pub to: Point,
    pub color_index: usize,
    pub object_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadSpec {
    pub brand: String,
    pub code: String,
    pub name: String,
    pub rgb: [u8; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    pub color_index: usize,
    pub thread: Option<ThreadSpec>,
}

/// Editable design result. Generators append stitches. Compilers read them.
///
/// An external model may propose regions and parameters. It must not invent
/// the records in `stitches`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StitchPlan {
    pub name: String,
    pub stitches: Vec<Stitch>,
    pub colors: Vec<ColorStop>,
    pub metadata: HashMap<String, String>,
}

impl StitchPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stitches: Vec::new(),
            colors: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn bounds(&self) -> Option<(Point, Point)> {
        let mut iter = self.stitches.iter();
        let first = iter.next()?;
        let mut min = first.from;
        let mut max = first.from;

        for stitch in &self.stitches {
            for point in [stitch.from, stitch.to] {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
            }
        }

        Some((min, max))
    }

    pub fn sewing_stitch_count(&self) -> usize {
        self.stitches
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .count()
    }
}

/// Convert millimetres to DST counts.
///
/// Ties at `n + 0.5` use half-to-even, the same rule as Python 3 `round`,
/// which is what pyembroidery uses when it quantises a coordinate.
pub fn mm_to_units(mm: f64) -> Result<i32, PlanError> {
    if !mm.is_finite() {
        return Err(PlanError::new(
            "A coordinate is not a finite number. Regenerate the geometry that produced it.",
        ));
    }
    let scaled = mm * DST_UNITS_PER_MM;
    if scaled.abs() > f64::from(DST_MAX_ABS_UNITS) + 0.5 {
        return Err(PlanError::new(format!(
            "Coordinate {mm:.3} mm is outside the DST header range of ±9999.9 mm. Move the design closer to the origin."
        )));
    }
    let units = round_half_even(scaled);
    if units.unsigned_abs() > DST_MAX_ABS_UNITS as u32 {
        return Err(PlanError::new(format!(
            "Coordinate {mm:.3} mm is outside the DST header range of ±9999.9 mm. Move the design closer to the origin."
        )));
    }
    Ok(units)
}

pub fn units_to_mm(units: i32) -> f64 {
    f64::from(units) * DST_UNIT_MM
}

/// Half-to-even rounding. Values within `1e-8` of a `*.5` boundary count as a tie
/// so that decimal millimetre inputs survive the binary multiply by 10.
pub(crate) fn round_half_even(value: f64) -> i32 {
    let trunc = value.trunc();
    let frac = (value - trunc).abs();
    if (frac - 0.5).abs() <= 1e-8 {
        let base = trunc as i64;
        let even = if base % 2 == 0 {
            base
        } else {
            base + value.signum() as i64
        };
        return even as i32;
    }
    value.round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_even_matches_python_round() {
        assert_eq!(mm_to_units(0.05).unwrap(), 0);
        assert_eq!(mm_to_units(0.15).unwrap(), 2);
        assert_eq!(mm_to_units(0.25).unwrap(), 2);
        assert_eq!(mm_to_units(0.35).unwrap(), 4);
        assert_eq!(mm_to_units(-0.15).unwrap(), -2);
        assert_eq!(mm_to_units(-0.25).unwrap(), -2);
        assert_eq!(mm_to_units(10.0).unwrap(), 100);
        assert_eq!(mm_to_units(-5.0).unwrap(), -50);
    }

    #[test]
    fn bounds_cover_both_ends() {
        let mut plan = StitchPlan::new("box");
        plan.stitches.push(Stitch {
            kind: StitchKind::Running,
            from: Point::new(1.0, 2.0),
            to: Point::new(4.0, -3.0),
            color_index: 0,
            object_id: 1,
        });
        let (min, max) = plan.bounds().unwrap();
        assert_eq!(min.x, 1.0);
        assert_eq!(min.y, -3.0);
        assert_eq!(max.x, 4.0);
        assert_eq!(max.y, 2.0);
        assert_eq!(plan.sewing_stitch_count(), 1);
    }

    #[test]
    fn empty_plan_has_no_bounds() {
        assert!(StitchPlan::new("empty").bounds().is_none());
    }

    #[test]
    fn out_of_range_coordinate_explains_the_limit() {
        let error = mm_to_units(20_000.0).unwrap_err();
        assert!(error.message().contains("9999.9"));
    }
}
