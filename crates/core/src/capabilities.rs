//! What the kernel can generate today.
//!
//! A schema may still name a planned stitch type so an agent can propose it.
//! Generation must consult this manifest and reject anything that is not
//! `available`. Unsupported requests do not fall back to another stitch type.

use serde::Serialize;

use crate::error::PlanError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Available,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Capability {
    pub id: &'static str,
    pub status: CapabilityStatus,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UnderlayRule {
    pub id: &'static str,
    pub status: CapabilityStatus,
    pub stitch_types: &'static [&'static str],
    pub detail: &'static str,
}

pub fn underlay_rules() -> &'static [UnderlayRule] {
    &[
        UnderlayRule {
            id: "centre_run",
            status: CapabilityStatus::Available,
            stitch_types: &["satin"],
            detail: "Available only together with edge_run.",
        },
        UnderlayRule {
            id: "edge_run",
            status: CapabilityStatus::Available,
            stitch_types: &["satin"],
            detail: "Available only together with centre_run.",
        },
        UnderlayRule {
            id: "perpendicular_run",
            status: CapabilityStatus::Available,
            stitch_types: &["fill"],
            detail: "Perpendicular running underlay for a simple fill.",
        },
        UnderlayRule {
            id: "zigzag",
            status: CapabilityStatus::Planned,
            stitch_types: &["satin", "fill"],
            detail: "Planned and unavailable.",
        },
        UnderlayRule {
            id: "lattice",
            status: CapabilityStatus::Planned,
            stitch_types: &["fill"],
            detail: "Planned and unavailable.",
        },
    ]
}

pub fn capability_manifest() -> &'static [Capability] {
    &[
        Capability {
            id: "running",
            status: CapabilityStatus::Available,
            detail: "Polyline running stitch. No underlay.",
        },
        Capability {
            id: "satin",
            status: CapabilityStatus::Available,
            detail: "Constant-width column. Underlay is available only as centre_run and edge_run together.",
        },
        Capability {
            id: "fill",
            status: CapabilityStatus::Available,
            detail: "Scanline fill of one simple outer ring and at most 32 simple holes. Holes must sit strictly inside, without touching, overlapping, or nesting. Perpendicular underlay uses the same region. This is not general Boolean clipping or an offset.",
        },
        Capability {
            id: "motif",
            status: CapabilityStatus::Planned,
            detail: "No motif generator is implemented.",
        },
        Capability {
            id: "applique",
            status: CapabilityStatus::Planned,
            detail: "No applique generator is implemented.",
        },
        Capability {
            id: "underlay.zigzag",
            status: CapabilityStatus::Planned,
            detail: "Zigzag underlay is not implemented.",
        },
        Capability {
            id: "underlay.lattice",
            status: CapabilityStatus::Planned,
            detail: "Lattice underlay is not implemented.",
        },
        Capability {
            id: "machine_release",
            status: CapabilityStatus::Planned,
            detail: "Local engineering export is not permission to transfer or run a machine file.",
        },
    ]
}

fn same(names: &[String], expected: &[&str]) -> bool {
    names.len() == expected.len()
        && names
            .iter()
            .zip(expected)
            .all(|(name, expected)| name == expected)
}

pub(crate) fn check_request(
    object_id: u64,
    label: &str,
    kind: &str,
    underlay: &[String],
) -> Result<(), PlanError> {
    let name = if label.is_empty() {
        format!("object {object_id}")
    } else {
        format!("object {object_id} ({label})")
    };

    match kind {
        "running" | "satin" | "fill" => {}
        "motif" | "applique" => {
            return Err(PlanError::new(format!(
                "{name} requests {kind}, which is planned and unavailable. Choose running, satin, or fill, then generate again."
            )));
        }
        other => {
            return Err(PlanError::new(format!(
                "{name} uses stitch type {other}, which this engine does not recognise. Choose running, satin, or fill, then generate again."
            )));
        }
    }

    let mut seen = Vec::new();
    for item in underlay {
        if seen.iter().any(|existing: &String| existing == item) {
            return Err(PlanError::new(format!(
                "{name} lists underlay {item} more than once. Keep each underlay once and generate again."
            )));
        }
        seen.push(item.clone());
        if matches!(item.as_str(), "zigzag" | "lattice") {
            return Err(PlanError::new(format!(
                "{name} requests underlay {item}, which is planned and unavailable. Remove it and generate again."
            )));
        }
    }

    let mut names = seen;
    names.sort();
    let allowed = match kind {
        "running" => names.is_empty(),
        "satin" => names.is_empty() || same(&names, &["centre_run", "edge_run"]),
        "fill" => names.is_empty() || same(&names, &["perpendicular_run"]),
        _ => false,
    };
    if allowed {
        return Ok(());
    }

    let supported = match kind {
        "running" => "no underlay",
        "satin" => "no underlay, or centre_run together with edge_run",
        "fill" => "no underlay, or perpendicular_run",
        _ => "a supported stitch type",
    };
    Err(PlanError::new(format!(
        "{name} requests underlay [{}], which this {kind} generator does not sew. Use {supported}, then generate again.",
        underlay.join(", ")
    )))
}
