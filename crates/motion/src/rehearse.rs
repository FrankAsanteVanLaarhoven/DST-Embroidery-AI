//! Build a rehearsal document from a native project.
//!
//! Stitches come from the Rust generators. The trace is decoded from the DST
//! bytes those generators were compiled into.

use serde::Serialize;
use serde_json::json;

use embroidery_core::{generate_project, parse_project};
use embroidery_dst::compile_to_vec_with_links;

use crate::frame::{sample_pose, Phase, SampledPose};
use crate::trace::{decode_execution, ExecutionTrace};
use crate::{GENERIC_PROFILE_ID, NEEDLE_UP_MM};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectionRequest {
    pub at_event: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Injection {
    kind: &'static str,
    at_event: u32,
    origin: &'static str,
    observed: bool,
    message: &'static str,
}

pub fn rehearse_project_json(body: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| {
        "The rehearsal body is not JSON. Send the native project and try again.".to_string()
    })?;
    let (project_json, injection) = if value.get("format").and_then(|item| item.as_str())
        == Some("embroidery-project")
    {
        (body.to_string(), None)
    } else if let Some(project) = value.get("project") {
        let scenario = value.get("scenario").and_then(|item| {
            if item.is_null() {
                None
            } else {
                Some(InjectionRequest {
                    at_event: item
                        .get("at_event")
                        .and_then(|event| event.as_u64())
                        .map(|event| event as u32),
                })
            }
        });
        if value.get("scenario").is_some()
            && !value["scenario"].is_null()
            && value["scenario"].get("kind").and_then(|kind| kind.as_str()) != Some("thread_break")
        {
            return Err(
                "The only rehearsal scenario is thread_break. Remove the scenario or inject a thread break."
                    .to_string(),
            );
        }
        (
            serde_json::to_string(project).map_err(|_| {
                "The project could not be read. Reload the design and rehearse it again."
                    .to_string()
            })?,
            scenario,
        )
    } else {
        return Err(
            "Send an embroidery-project document. The reference EXF format is not accepted."
                .to_string(),
        );
    };
    let rehearsal = rehearse_project(&project_json, injection)?;
    serde_json::to_string(&rehearsal).map_err(|_| {
        "The rehearsal result could not be encoded. Export the design and rehearse it again."
            .to_string()
    })
}

pub fn rehearse_project(
    project_json: &str,
    injection: Option<InjectionRequest>,
) -> Result<Rehearsal, String> {
    let project = parse_project(project_json).map_err(|error| error.to_string())?;
    let generation = generate_project(&project).map_err(|error| error.to_string())?;
    if !generation.readiness.engineering_export_allowed {
        let details = if generation.validation.errors.is_empty() {
            "The design is not ready for an engineering export.".to_string()
        } else {
            generation.validation.errors.join(" ")
        };
        return Err(format!(
            "{details} Fix the design and rehearse the export again."
        ));
    }
    let (bytes, links) = compile_to_vec_with_links(&generation.plan, 12.1, 1000.0)
        .map_err(|error| error.to_string())?;
    let trace = decode_execution(
        &bytes,
        &links,
        project.hoop_width_mm,
        project.hoop_height_mm,
    )?;
    let injected = match injection {
        None => None,
        Some(request) => {
            let at_event = request.at_event.unwrap_or_else(|| {
                trace
                    .events
                    .iter()
                    .find(|event| event.kind == "sew")
                    .map(|event| event.index)
                    .unwrap_or(0)
            });
            if trace.events.get(at_event as usize).is_none() {
                return Err(
                    "The thread-break scenario points past the end of the trace. Choose an event in the timeline and inject it again."
                        .to_string(),
                );
            }
            Some(Injection {
                kind: "thread_break",
                at_event,
                origin: "injected_scenario",
                observed: false,
                message: "Injected thread-break scenario. This was not reported by a machine.",
            })
        }
    };
    let samples = pose_samples(&trace)?;
    Ok(Rehearsal {
        ok: true,
        qualification: "Virtual demo · Generic machine · No hardware connected · Physical quality not validated",
        machine_release_allowed: false,
        hardware_connected: false,
        physical_quality_validated: false,
        mode: "kinematic_demo",
        revision: generation.revision,
        provenance: generation.provenance,
        hoop_width_mm: project.hoop_width_mm,
        hoop_height_mm: project.hoop_height_mm,
        profile: profile(),
        transform: TransformNotes::declared(),
        trace,
        pose_samples: samples,
        injection: injected,
        needle_up_mm: NEEDLE_UP_MM,
    })
}

fn pose_samples(trace: &ExecutionTrace) -> Result<Vec<SampledPose>, String> {
    let Some(event) = trace.events.iter().find(|event| event.penetrates) else {
        return Ok(Vec::new());
    };
    Ok(vec![
        sample_pose(trace, event.index, Phase::Travel, 0.0)?,
        sample_pose(trace, event.index, Phase::Travel, 1.0)?,
        sample_pose(trace, event.index, Phase::Penetrate, 0.5)?,
    ])
}

fn profile() -> serde_json::Value {
    json!({
        "id": GENERIC_PROFILE_ID,
        "manufacturer": "generic",
        "model": "fixed-needle-xy-frame",
        "evidence": "declared_demo",
        "mechanism": "The needle stays on the machine axis. The hoop and fabric move by the negated design point and hold still during penetration.",
        "timing_note": "600 stitches per minute, 200 mm/s jumps, 2 s colour changes, and 0.4 s trims are development assumptions. They are not a manufacturer rating.",
        "capabilities": {
            "geometry": "declared",
            "timing": "assumed_demo",
            "collision": "unknown",
            "file_acceptance": "unknown",
            "fabric_prediction": "unknown",
            "telemetry": "unavailable",
            "cap_attachment": "unavailable",
            "machine_release": "unavailable"
        },
        "unavailable_modes": [
            {
                "id": "calibrated_process_model",
                "reason": "No measured observations are loaded. This demo cannot issue a calibrated prediction."
            },
            {
                "id": "connected_readonly_twin",
                "reason": "No machine telemetry is connected."
            }
        ]
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TransformNotes {
    design: &'static str,
    scene: &'static str,
    mechanism: &'static str,
}

impl TransformNotes {
    fn declared() -> Self {
        Self {
            design: "millimetres, Y down",
            scene: "metres, Y up; design Y maps to scene Z; 1 mm = 0.001 m",
            mechanism:
                "fixed needle at scene XZ origin; hoop translation is the negated design point",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rehearsal {
    pub ok: bool,
    pub qualification: &'static str,
    pub machine_release_allowed: bool,
    pub hardware_connected: bool,
    pub physical_quality_validated: bool,
    pub mode: &'static str,
    pub revision: String,
    pub provenance: embroidery_core::Provenance,
    pub hoop_width_mm: f64,
    pub hoop_height_mm: f64,
    pub profile: serde_json::Value,
    pub transform: TransformNotes,
    pub trace: ExecutionTrace,
    pub pose_samples: Vec<SampledPose>,
    pub injection: Option<Injection>,
    pub needle_up_mm: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use embroidery_dst::compile_to_vec;

    #[test]
    fn club_mark_replay_follows_the_exported_bytes() {
        let text = include_str!("../../../fixtures/projects/08_club_mark.json");
        let rehearsal = rehearse_project(text, None).unwrap();
        assert!(!rehearsal.machine_release_allowed);
        assert!(!rehearsal.hardware_connected);
        assert!(!rehearsal.physical_quality_validated);
        assert_eq!(rehearsal.mode, "kinematic_demo");
        let project = parse_project(text).unwrap();
        let generation = generate_project(&project).unwrap();
        let bytes = compile_to_vec(&generation.plan, 12.1, 1000.0).unwrap();
        assert_eq!(
            rehearsal.trace.dst_sha256,
            embroidery_core::sha256_hex(&bytes)
        );
        let sew_objects: Vec<_> = rehearsal
            .trace
            .events
            .iter()
            .filter(|event| event.kind == "sew")
            .map(|event| event.object_id)
            .collect();
        assert!(sew_objects.contains(&Some(1)));
        assert!(sew_objects.contains(&Some(2)));
        assert!(rehearsal
            .trace
            .events
            .iter()
            .any(|event| event.kind == "color_change"));
        assert!(rehearsal.pose_samples.len() == 3);
        assert!(rehearsal.injection.is_none());
    }

    #[test]
    fn an_injected_thread_break_is_not_an_observation() {
        let text = include_str!("../../../fixtures/projects/01_running_line.json");
        let rehearsal =
            rehearse_project(text, Some(InjectionRequest { at_event: Some(0) })).unwrap();
        let injection = rehearsal.injection.unwrap();
        assert_eq!(injection.origin, "injected_scenario");
        assert!(!injection.observed);
        assert_eq!(injection.at_event, 0);
    }

    #[test]
    fn motif_is_rejected_before_a_trace_exists() {
        let text = include_str!("../../../fixtures/projects/09_invalid_motif.json");
        let error = rehearse_project(text, None).unwrap_err();
        assert!(error.contains("unavailable"), "{error}");
    }
}
