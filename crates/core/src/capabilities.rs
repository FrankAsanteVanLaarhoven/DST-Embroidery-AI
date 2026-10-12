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
            stitch_types: &["satin", "text"],
            detail: "Selectable on satin or text by itself or with edge_run.",
        },
        UnderlayRule {
            id: "edge_run",
            status: CapabilityStatus::Available,
            stitch_types: &["satin", "text"],
            detail: "Selectable on satin or text by itself or with centre_run.",
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
            detail: "Column satin. An empty width list uses one column width and still rejects a rung past one DST record. A width on every centreline point varies the column, miters a sharp corner up to twice the half-width, splits a sewing movement wider than 12 mm on an axis, and omits a satin segment shorter than 0.2 mm. A reversed, folded, or crossing column is rejected. Centre-run and edge-run underlay can be selected alone or together.",
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
            id: "lettering",
            status: CapabilityStatus::Available,
            detail: "One text object in EmbroideryOS Block 01. Version 1 is the default repertoire: A–Z, 0–9, space, hyphen, and a straight apostrophe. Version 2 is a separate pinned recipe, chosen explicitly, adding lowercase, U+2019, and a bounded set of accented letters. Cap height is a software size, not the full line height or a measured sewing limit. One to four horizontal lines are an explicit layout policy, embroideryos-line-layout/1, chosen separately from the font. Curves, font import, and a font catalogue are not implemented.",
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
        "running" | "satin" | "fill" | "text" => {}
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
        "satin" | "text" => names
            .iter()
            .all(|name| name == "centre_run" || name == "edge_run"),
        "fill" => names.is_empty() || same(&names, &["perpendicular_run"]),
        _ => false,
    };
    if allowed {
        return Ok(());
    }

    let supported = match kind {
        "running" => "no underlay",
        "satin" | "text" => "no underlay, centre_run, edge_run, or both",
        "fill" => "no underlay, or perpendicular_run",
        _ => "a supported stitch type",
    };
    Err(PlanError::new(format!(
        "{name} requests underlay [{}], which this {kind} generator does not sew. Use {supported}, then generate again.",
        underlay.join(", ")
    )))
}
