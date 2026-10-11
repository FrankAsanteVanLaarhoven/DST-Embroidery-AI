//! Export rehearsal for a generic fixed-needle machine.
//!
//! The trace is decoded from compiled DST bytes. The rig moves the hoop and
//! holds it while the needle descends. It does not claim a measured machine,
//! a fabric prediction, or a hardware connection.

mod frame;
mod rehearse;
mod trace;

pub use frame::{design_mm_to_scene_m, sample_pose, scene_m_to_design_mm, Phase, SampledPose};
pub use rehearse::{rehearse_project, rehearse_project_json, InjectionRequest};
pub use trace::{decode_execution, ExecutionTrace, TraceEvent};

pub const GENERIC_PROFILE_ID: &str = "generic-flat-xy-frame";
pub const DEMO_STITCHES_PER_MINUTE: f64 = 600.0;
pub const DEMO_JUMP_MM_PER_SEC: f64 = 200.0;
pub const DEMO_COLOR_CHANGE_SECONDS: f64 = 2.0;
pub const DEMO_TRIM_SECONDS: f64 = 0.4;
pub const NEEDLE_UP_MM: f64 = 14.0;
