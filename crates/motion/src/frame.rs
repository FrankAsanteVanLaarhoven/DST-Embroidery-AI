//! Named transforms between design millimetres and the rendered scene.
//!
//! Design space is millimetres, Y positive downward. The scene is metres and
//! Y-up, matching glTF. Design Y maps to scene Z. There is no extra scale.

use serde::Serialize;

use crate::trace::ExecutionTrace;
use crate::NEEDLE_UP_MM;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Travel,
    Penetrate,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Phase::Travel => "travel",
            Phase::Penetrate => "penetrate",
        }
    }
}

/// One pose of the generic fixed-needle rig.
///
/// `needle_machine_x_mm` and `needle_machine_y_mm` stay at zero. The hoop
/// translation is what places `design_x_mm, design_y_mm` under the needle.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SampledPose {
    pub event_index: u32,
    pub phase: String,
    pub t: f64,
    pub design_x_mm: f64,
    pub design_y_mm: f64,
    pub hoop_x_mm: f64,
    pub hoop_y_mm: f64,
    pub needle_tip_above_fabric_mm: f64,
    pub needle_machine_x_mm: f64,
    pub needle_machine_y_mm: f64,
    pub scene_needle_m: [f64; 3],
    pub scene_hoop_m: [f64; 3],
}

pub fn design_mm_to_scene_m(x_mm: f64, y_down_mm: f64, height_mm: f64) -> [f64; 3] {
    [x_mm / 1000.0, height_mm / 1000.0, y_down_mm / 1000.0]
}

pub fn scene_m_to_design_mm(scene: [f64; 3]) -> (f64, f64) {
    (scene[0] * 1000.0, scene[2] * 1000.0)
}

pub fn sample_pose(
    trace: &ExecutionTrace,
    event_index: u32,
    phase: Phase,
    t: f64,
) -> Result<SampledPose, String> {
    let event = trace.events.get(event_index as usize).ok_or_else(|| {
        format!(
            "Event {event_index} is outside this trace of {} events. Choose an event in the timeline.",
            trace.events.len()
        )
    })?;
    if !t.is_finite() {
        return Err(
            "Playback position is not a finite number. Rewind the trace and play it again."
                .to_string(),
        );
    }
    let t = t.clamp(0.0, 1.0);
    if phase == Phase::Penetrate && !event.penetrates {
        return Err(
            "This event has no needle penetration. Step to a sewing event or stay in the travel phase."
                .to_string(),
        );
    }

    let (design_x, design_y, needle) = match phase {
        Phase::Travel => {
            let x = lerp(event.from_x_mm, event.design_x_mm, t);
            let y = lerp(event.from_y_mm, event.design_y_mm, t);
            (x, y, NEEDLE_UP_MM)
        }
        Phase::Penetrate => (event.design_x_mm, event.design_y_mm, needle_tip(t)),
    };
    let hoop_x = -design_x;
    let hoop_y = -design_y;
    Ok(SampledPose {
        event_index,
        phase: phase.label().to_string(),
        t,
        design_x_mm: design_x,
        design_y_mm: design_y,
        hoop_x_mm: hoop_x,
        hoop_y_mm: hoop_y,
        needle_tip_above_fabric_mm: needle,
        needle_machine_x_mm: 0.0,
        needle_machine_y_mm: 0.0,
        scene_needle_m: design_mm_to_scene_m(0.0, 0.0, needle),
        scene_hoop_m: design_mm_to_scene_m(hoop_x, hoop_y, 0.0),
    })
}

fn needle_tip(t: f64) -> f64 {
    if t <= 0.5 {
        NEEDLE_UP_MM * (1.0 - t / 0.5)
    } else {
        NEEDLE_UP_MM * ((t - 0.5) / 0.5)
    }
}

fn lerp(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::decode_execution;
    use embroidery_core::{generate_running, Point, RunningConfig, StitchPlan};
    use embroidery_dst::compile_to_vec_with_links;

    fn line_trace() -> ExecutionTrace {
        let mut plan = StitchPlan::new("Line");
        generate_running(
            &mut plan,
            7,
            0,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            &RunningConfig {
                max_stitch_length_mm: 5.0,
            },
        )
        .unwrap();
        let (bytes, links) = compile_to_vec_with_links(&plan, 12.1, 1000.0).unwrap();
        decode_execution(&bytes, &links, 120.0, 100.0).unwrap()
    }

    #[test]
    fn design_millimetres_map_onto_scene_metres() {
        let scene = design_mm_to_scene_m(10.0, 20.0, 14.0);
        assert_eq!(scene, [0.01, 0.014, 0.02]);
        assert_eq!(scene_m_to_design_mm(scene), (10.0, 20.0));
    }

    #[test]
    fn penetration_holds_the_hoop_and_leaves_the_needle_on_axis() {
        let trace = line_trace();
        let sew = trace
            .events
            .iter()
            .find(|event| event.kind == "sew")
            .unwrap();
        let pose = sample_pose(&trace, sew.index, Phase::Penetrate, 0.5).unwrap();
        assert_eq!(pose.needle_machine_x_mm, 0.0);
        assert_eq!(pose.needle_machine_y_mm, 0.0);
        assert!((pose.needle_tip_above_fabric_mm - 0.0).abs() < 1e-9);
        assert!((pose.design_x_mm - sew.design_x_mm).abs() < 1e-9);
        assert!((pose.hoop_x_mm + sew.design_x_mm).abs() < 1e-9);
        assert!((pose.hoop_y_mm + sew.design_y_mm).abs() < 1e-9);
        assert_eq!(pose.scene_needle_m[0], 0.0);
        assert_eq!(pose.scene_needle_m[2], 0.0);
        assert_eq!(
            pose.scene_hoop_m,
            design_mm_to_scene_m(pose.hoop_x_mm, pose.hoop_y_mm, 0.0)
        );
    }

    #[test]
    fn travel_moves_the_hoop_with_the_needle_up() {
        let trace = line_trace();
        let sew = trace
            .events
            .iter()
            .find(|event| event.kind == "sew")
            .unwrap();
        let start = sample_pose(&trace, sew.index, Phase::Travel, 0.0).unwrap();
        let middle = sample_pose(&trace, sew.index, Phase::Travel, 0.5).unwrap();
        assert!((start.needle_tip_above_fabric_mm - NEEDLE_UP_MM).abs() < 1e-9);
        assert!((middle.needle_tip_above_fabric_mm - NEEDLE_UP_MM).abs() < 1e-9);
        assert!((start.design_x_mm - sew.from_x_mm).abs() < 1e-9);
        let expected = (sew.from_x_mm + sew.design_x_mm) / 2.0;
        assert!((middle.design_x_mm - expected).abs() < 1e-9);
        assert!((middle.hoop_x_mm + middle.design_x_mm).abs() < 1e-9);
    }
}
