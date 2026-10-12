//! Versioned editable project.
//!
//! Geometry and parameters live here. [`generate_project`] calls the Rust
//! generators and returns a derived stitch plan. Callers do not supply stitch
//! coordinates. A geometry review is bound to the revision hash and is not a
//! machine release.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::capabilities::check_request;
use crate::error::PlanError;
use crate::generators::{
    generate_fill, generate_fill_with_holes, generate_running, generate_satin,
    generate_satin_variable, FillConfig, RunningConfig, SatinConfig,
};
use crate::lettering::{generate_text, LineLayout, TextRequest};
use crate::model::{ColorStop, Point, Stitch, StitchKind, StitchPlan};
use crate::sha256::sha256_hex;
use crate::validate::{validate, ValidationReport};

const FORMAT: &str = "embroidery-project";
const FORMAT_VERSION: &str = "1";
/// A document that contains a text object. Version 1 documents stay version 1.
const TEXT_FORMAT_VERSION: &str = "2";
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_OBJECTS: usize = 500;
const MAX_POINTS: usize = 10_000;
const MAX_HOOP_MM: f64 = 10_000.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDocument {
    pub format: String,
    pub format_version: String,
    pub name: String,
    pub hoop_width_mm: f64,
    pub hoop_height_mm: f64,
    pub objects: Vec<DesignObject>,
    pub review: Option<GeometryReview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignObject {
    pub object_id: u64,
    pub label: String,
    pub kind: String,
    pub points: Vec<Point>,
    /// Closed rings subtracted from a fill. Absent on older projects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<Point>>,
    pub angle_degrees: f64,
    pub density_mm: f64,
    pub column_width_mm: f64,
    /// One full column width for each centreline point. Empty keeps `column_width_mm`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_widths_mm: Vec<f64>,
    pub max_stitch_mm: f64,
    pub underlay_spacing_mm: f64,
    pub underlay: Vec<String>,
    /// Centre-run maximum stitch length. Absent inherits `max_stitch_mm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre_run_max_stitch_mm: Option<f64>,
    /// Edge-run maximum stitch length. Absent inherits `max_stitch_mm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_run_max_stitch_mm: Option<f64>,
    pub pull_compensation_mm: f64,
    pub color_index: usize,
    /// Editable lettering. Omitted on geometry objects so version 1 bytes stay stable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextIntent>,
}

/// Entered text and the font recipe it was pinned to.
///
/// Glyph outlines and stitches are derived. They are not stored here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextIntent {
    pub content: String,
    pub font_id: String,
    pub font_version: String,
    pub font_sha256: String,
    pub origin_x_mm: f64,
    pub origin_y_mm: f64,
    pub cap_height_mm: f64,
    pub letter_spacing_mm: f64,
    pub word_spacing_mm: f64,
    /// Explicit horizontal lines. Absent keeps the historical single-line path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<LineLayout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryReview {
    pub revision: String,
    pub reviewer: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Provenance {
    pub engine: &'static str,
    pub engine_version: &'static str,
    pub generators: Vec<String>,
    pub project_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Readiness {
    pub state: String,
    pub label: String,
    pub engineering_export_allowed: bool,
    pub reviewed_export_allowed: bool,
    pub machine_release_allowed: bool,
    pub review_matches_revision: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Generation {
    pub revision: String,
    pub plan: StitchPlan,
    pub validation: ValidationReport,
    pub provenance: Provenance,
    pub readiness: Readiness,
}

pub fn parse_project(text: &str) -> Result<ProjectDocument, PlanError> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| {
        PlanError::new(format!(
            "Could not read the project JSON ({error}). Save an embroidery-project version 1 or version 2 document and try again."
        ))
    })?;
    let format = value
        .get("format")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    if format != FORMAT {
        return Err(PlanError::new(
            "This file is not an embroidery-project document. The reference EXF format is not accepted until a tested adapter exists. Create a native project and try again.",
        ));
    }
    let project: ProjectDocument = serde_json::from_value(value).map_err(|error| {
        PlanError::new(format!(
            "The project document does not match embroidery-project version 1 or 2 ({error}). Remove unsupported fields and try again."
        ))
    })?;
    if project.format_version != FORMAT_VERSION && project.format_version != TEXT_FORMAT_VERSION {
        return Err(PlanError::new(format!(
            "Project format version {} is not supported. This engine reads embroidery-project version 1, or version 2 when the design contains a text object.",
            project.format_version
        )));
    }
    if project.format_version == FORMAT_VERSION
        && project
            .objects
            .iter()
            .any(|object| object.kind == "text" || object.text.is_some())
    {
        return Err(PlanError::new(
            "Project format version 1 cannot store a text object. Save the design as embroidery-project version 2 and open it again. The text was not dropped.",
        ));
    }
    Ok(project)
}

pub fn revision_of(project: &ProjectDocument) -> String {
    let mut value = serde_json::to_value(project).expect("project serialization");
    if let Some(map) = value.as_object_mut() {
        map.remove("review");
    }
    let canonical = serde_json::to_string(&value).expect("canonical project JSON");
    sha256_hex(canonical.as_bytes())
}

pub fn stamp_geometry_review(
    project: &mut ProjectDocument,
    reviewer: &str,
) -> Result<String, PlanError> {
    let reviewer = reviewer.trim();
    let length = reviewer.chars().count();
    if reviewer.is_empty()
        || length > 80
        || reviewer.chars().any(|character| character.is_control())
    {
        return Err(PlanError::new(
            "Reviewer name must be 1 to 80 visible characters. Enter a name and record the geometry review again.",
        ));
    }
    let revision = revision_of(project);
    project.review = Some(GeometryReview {
        revision: revision.clone(),
        reviewer: reviewer.to_string(),
        scope: "geometry".to_string(),
    });
    Ok(revision)
}

pub fn readiness_for(project: &ProjectDocument, checks_ok: bool) -> Readiness {
    let revision = revision_of(project);
    let review_matches_revision = project
        .review
        .as_ref()
        .is_some_and(|review| review.scope == "geometry" && review.revision == revision);
    let mut notes = vec![
        "Production release is not implemented. An engineering DST file is not permission to transfer or run a machine.".to_string(),
    ];
    if let Some(review) = &project.review {
        if !review_matches_revision {
            notes.push(format!(
                "The geometry review by {} is for revision {}, not {revision}. Record a new geometry review after generating this revision.",
                review.reviewer, review.revision
            ));
        }
    }

    let (state, label) = if !checks_ok {
        ("draft", "Draft")
    } else if review_matches_revision {
        ("reviewed", "Reviewed")
    } else {
        ("geometry_checked", "Geometry checked")
    };

    Readiness {
        state: state.to_string(),
        label: label.to_string(),
        engineering_export_allowed: checks_ok,
        reviewed_export_allowed: checks_ok && review_matches_revision,
        machine_release_allowed: false,
        review_matches_revision,
        notes,
    }
}

pub fn generate_project(project: &ProjectDocument) -> Result<Generation, PlanError> {
    validate_document(project)?;
    let revision = revision_of(project);
    let mut plan = StitchPlan::new(project.name.trim());
    if plan.name.is_empty() {
        plan.name = "Untitled".to_string();
    }
    let mut generators = Vec::new();
    let mut colors = BTreeSet::new();

    for object in &project.objects {
        check_request(
            object.object_id,
            &object.label,
            &object.kind,
            &object.underlay,
        )?;
        if !object.holes.is_empty() && object.kind != "fill" {
            return Err(PlanError::new(format!(
                "Object {} is a {} and carries holes. Holes are supported only on a fill region. Remove the holes or change the object to fill, then generate again.",
                object.object_id, object.kind
            )));
        }
        check_numbers(object)?;
        check_text(object)?;
        ensure_inside_source(project, object)?;
        ensure_color(&mut plan, object.color_index, object.object_id);
        colors.insert(object.color_index);
        match object.kind.as_str() {
            "running" => {
                generate_running(
                    &mut plan,
                    object.object_id,
                    object.color_index,
                    &object.points,
                    &RunningConfig {
                        max_stitch_length_mm: object.max_stitch_mm,
                    },
                )?;
                generators.push("running".to_string());
            }
            "satin" => {
                let config = SatinConfig {
                    max_column_width_mm: object.column_width_mm,
                    density_mm: object.density_mm,
                    pull_compensation_mm: object.pull_compensation_mm,
                    centre_run_mm: satin_pass_length(
                        object,
                        "centre_run",
                        object.centre_run_max_stitch_mm,
                    )?,
                    edge_run_mm: satin_pass_length(
                        object,
                        "edge_run",
                        object.edge_run_max_stitch_mm,
                    )?,
                };
                if object.column_widths_mm.is_empty() {
                    generate_satin(
                        &mut plan,
                        object.object_id,
                        object.color_index,
                        &object.points,
                        &config,
                    )?;
                } else {
                    generate_satin_variable(
                        &mut plan,
                        object.object_id,
                        object.color_index,
                        &object.points,
                        &object.column_widths_mm,
                        &config,
                    )?;
                }
                generators.push("satin".to_string());
            }
            "fill" => {
                let underlay = if object.underlay.is_empty() {
                    0.0
                } else {
                    object.underlay_spacing_mm
                };
                let config = FillConfig {
                    angle_degrees: object.angle_degrees,
                    row_spacing_mm: object.density_mm,
                    max_stitch_length_mm: object.max_stitch_mm,
                    underlay_spacing_mm: underlay,
                };
                if object.holes.is_empty() {
                    generate_fill(
                        &mut plan,
                        object.object_id,
                        object.color_index,
                        &object.points,
                        &config,
                    )?;
                } else {
                    generate_fill_with_holes(
                        &mut plan,
                        object.object_id,
                        object.color_index,
                        &object.points,
                        &object.holes,
                        &config,
                    )?;
                }
                generators.push("fill".to_string());
            }
            "text" => {
                let text = object.text.as_ref().expect("text object checked");
                generate_text(
                    &mut plan,
                    &TextRequest {
                        object_id: object.object_id,
                        content: &text.content,
                        font_id: &text.font_id,
                        font_version: &text.font_version,
                        font_sha256: &text.font_sha256,
                        origin_x_mm: text.origin_x_mm,
                        origin_y_mm: text.origin_y_mm,
                        cap_height_mm: text.cap_height_mm,
                        letter_spacing_mm: text.letter_spacing_mm,
                        word_spacing_mm: text.word_spacing_mm,
                        density_mm: object.density_mm,
                        column_width_mm: object.column_width_mm,
                        pull_compensation_mm: object.pull_compensation_mm,
                        max_stitch_mm: object.max_stitch_mm,
                        centre_run_mm: satin_pass_length(
                            object,
                            "centre_run",
                            object.centre_run_max_stitch_mm,
                        )?,
                        edge_run_mm: satin_pass_length(
                            object,
                            "edge_run",
                            object.edge_run_max_stitch_mm,
                        )?,
                        color_index: object.color_index,
                        min_x: 0.0,
                        min_y: 0.0,
                        max_x: project.hoop_width_mm,
                        max_y: project.hoop_height_mm,
                        layout: text.layout.as_ref(),
                    },
                )?;
                generators.push("lettering".to_string());
            }
            _ => {
                return Err(PlanError::new(format!(
                    "Object {} requests {}, which is not an implemented generator. Choose running, satin, fill, or text.",
                    object.object_id, object.kind
                )));
            }
        }
    }

    for color_index in colors {
        plan.colors.push(ColorStop {
            color_index,
            thread: None,
        });
    }
    plan.metadata
        .insert("project_revision".into(), revision.clone());
    plan.metadata
        .insert("machine_release".into(), "unavailable".into());

    let mut validation = validate(&plan, 12.1, 1000.0);
    append_hoop_errors(project, &plan, &mut validation);
    let mut readiness = readiness_for(project, validation.ok());
    if let Some(summary) = plan.metadata.get("satin_movement_summary") {
        readiness.notes.push(summary.clone());
    }
    if let Some(summary) = plan.metadata.get("satin_phase_summary") {
        readiness.notes.push(summary.clone());
    }
    if let Some(summary) = plan.metadata.get("lettering_summary") {
        readiness.notes.push(summary.clone());
    }
    Ok(Generation {
        provenance: Provenance {
            engine: "embroidery-core",
            engine_version: ENGINE_VERSION,
            generators,
            project_revision: revision.clone(),
        },
        revision,
        plan,
        validation,
        readiness,
    })
}

fn validate_document(project: &ProjectDocument) -> Result<(), PlanError> {
    if project.name.trim().is_empty() || project.name.chars().count() > 80 {
        return Err(PlanError::new(
            "Project name must be 1 to 80 characters. Name the project and generate again.",
        ));
    }
    if !project.hoop_width_mm.is_finite()
        || !project.hoop_height_mm.is_finite()
        || project.hoop_width_mm <= 0.0
        || project.hoop_height_mm <= 0.0
        || project.hoop_width_mm > MAX_HOOP_MM
        || project.hoop_height_mm > MAX_HOOP_MM
    {
        return Err(PlanError::new(
            "Hoop width and height must be positive and at most 10000 mm. Set the hoop size and generate again.",
        ));
    }
    if project.objects.is_empty() {
        return Err(PlanError::new(
            "The project has no objects. Add a running, satin, or fill region and generate again.",
        ));
    }
    if project.objects.len() > MAX_OBJECTS {
        return Err(PlanError::new(format!(
            "This project has {} objects. Split it into projects of at most {MAX_OBJECTS} objects and generate again.",
            project.objects.len()
        )));
    }
    let mut seen = BTreeSet::new();
    for object in &project.objects {
        if !seen.insert(object.object_id) {
            return Err(PlanError::new(format!(
                "Object id {} is used more than once. Give every region its own id and generate again.",
                object.object_id
            )));
        }
        if object.label.trim().is_empty() || object.label.chars().count() > 80 {
            return Err(PlanError::new(format!(
                "Object {} needs a label of 1 to 80 characters. Name the region and generate again.",
                object.object_id
            )));
        }
        if object.points.len() > MAX_POINTS {
            return Err(PlanError::new(format!(
                "Object {} has too many points. Simplify the outline below {MAX_POINTS} points and generate again.",
                object.object_id
            )));
        }
    }
    Ok(())
}

fn check_numbers(object: &DesignObject) -> Result<(), PlanError> {
    let numbers = [
        ("angle_degrees", object.angle_degrees),
        ("density_mm", object.density_mm),
        ("column_width_mm", object.column_width_mm),
        ("max_stitch_mm", object.max_stitch_mm),
        ("underlay_spacing_mm", object.underlay_spacing_mm),
        ("pull_compensation_mm", object.pull_compensation_mm),
    ];
    for (name, value) in numbers {
        if !value.is_finite() {
            return Err(PlanError::new(format!(
                "Object {} has a non-finite {name}. Enter a finite number and generate again.",
                object.object_id
            )));
        }
    }
    if object.pull_compensation_mm < 0.0 {
        return Err(PlanError::new(format!(
            "Object {} has negative pull compensation. Use 0 or a positive allowance and generate again.",
            object.object_id
        )));
    }
    if !object.column_widths_mm.is_empty() {
        if object.kind != "satin" {
            return Err(PlanError::new(format!(
                "Object {} carries column widths, which are used only by satin. Remove the widths or change the object to satin, then generate again.",
                object.object_id
            )));
        }
        if object.column_widths_mm.len() != object.points.len() {
            return Err(PlanError::new(format!(
                "Object {} needs one column width for each centreline point. Match the widths to the points and generate again.",
                object.object_id
            )));
        }
        if object
            .column_widths_mm
            .iter()
            .any(|width| !width.is_finite() || *width <= 0.0 || *width > 48.0)
        {
            return Err(PlanError::new(format!(
                "Object {} has a column width outside 0 to 48 mm. Enter a width in that range and generate again.",
                object.object_id
            )));
        }
    }
    for (name, length) in [
        ("centre_run_max_stitch_mm", object.centre_run_max_stitch_mm),
        ("edge_run_max_stitch_mm", object.edge_run_max_stitch_mm),
    ] {
        if let Some(length) = length {
            if object.kind != "satin" && object.kind != "text" {
                return Err(PlanError::new(format!(
                    "Object {} carries {name}, which belongs only on satin or text. Remove it or change the object, then generate again.",
                    object.object_id
                )));
            }
            if !length.is_finite() || length <= 0.0 || length > crate::DST_MAX_STITCH_MM {
                return Err(PlanError::new(format!(
                    "Object {} has a {name} outside 0 to {:.1} mm. Reduce that length or remove it, then generate again.",
                    object.object_id,
                    crate::DST_MAX_STITCH_MM
                )));
            }
        }
    }
    if object.kind == "fill" && !object.underlay.is_empty() && object.underlay_spacing_mm <= 0.0 {
        return Err(PlanError::new(format!(
            "Object {} requests fill underlay but underlay_spacing_mm is not positive. Set the underlay spacing or clear the underlay, then generate again.",
            object.object_id
        )));
    }
    Ok(())
}

fn check_text(object: &DesignObject) -> Result<(), PlanError> {
    if object.kind == "text" {
        let Some(text) = object.text.as_ref() else {
            return Err(PlanError::new(format!(
                "Object {} is text and has no lettering intent. Enter the text and pin the font. An outline was not inferred.",
                object.object_id
            )));
        };
        if !object.points.is_empty() || !object.column_widths_mm.is_empty() {
            return Err(PlanError::new(format!(
                "Object {} is text and carries geometry. Lettering stores the entered string, not an outline. Remove the points and column widths, then generate again.",
                object.object_id
            )));
        }
        if object.angle_degrees.abs() > 1e-9 {
            let message = if text.layout.is_some() {
                format!(
                    "Object {} sets a text angle. Horizontal lines are not rotated. Set the angle to 0 and generate again.",
                    object.object_id
                )
            } else {
                format!(
                    "Object {} sets a text angle. This lettering step sews one horizontal line. Set the angle to 0 and generate again.",
                    object.object_id
                )
            };
            return Err(PlanError::new(message));
        }
        if let Some(layout) = text.layout.as_ref() {
            crate::lettering_layout::validate_layout(object.object_id, layout)?;
        }
        if object.underlay_spacing_mm != 0.0 {
            return Err(PlanError::new(format!(
                "Object {} sets underlay spacing. Lettering uses centre-run and edge-run lengths. Set underlay spacing to 0 and generate again.",
                object.object_id
            )));
        }
        for (name, value) in [
            ("origin_x_mm", text.origin_x_mm),
            ("origin_y_mm", text.origin_y_mm),
            ("cap_height_mm", text.cap_height_mm),
            ("letter_spacing_mm", text.letter_spacing_mm),
            ("word_spacing_mm", text.word_spacing_mm),
        ] {
            if !value.is_finite() {
                return Err(PlanError::new(format!(
                    "Object {} has a non-finite {name}. Enter a finite number and generate again.",
                    object.object_id
                )));
            }
        }
        return Ok(());
    }
    if object.text.is_some() {
        return Err(PlanError::new(format!(
            "Object {} carries text. Text belongs on a text object. Remove it or change the object to text, then generate again. The text was not dropped.",
            object.object_id
        )));
    }
    Ok(())
}

fn satin_pass_length(
    object: &DesignObject,
    name: &str,
    specific: Option<f64>,
) -> Result<Option<f64>, PlanError> {
    if !object.underlay.iter().any(|item| item == name) {
        return Ok(None);
    }
    let length = specific.unwrap_or(object.max_stitch_mm);
    if !length.is_finite() || length <= 0.0 {
        return Err(PlanError::new(format!(
            "Object {} requests {name} but that pass has no positive maximum stitch length. Set the length or clear the pass, then generate again. The plan was not changed.",
            object.object_id
        )));
    }
    Ok(Some(length))
}

fn ensure_inside_source(project: &ProjectDocument, object: &DesignObject) -> Result<(), PlanError> {
    let mut points = object.points.iter();
    let holes = object.holes.iter().flatten();
    for point in points.by_ref().chain(holes) {
        if !inside(*point, project.hoop_width_mm, project.hoop_height_mm) {
            return Err(PlanError::new(format!(
                "Object {} ({}) has a point at ({:.2}, {:.2}) mm outside the {:.1} by {:.1} mm hoop. Move the object inside the hoop and generate again.",
                object.object_id,
                object.label,
                point.x,
                point.y,
                project.hoop_width_mm,
                project.hoop_height_mm
            )));
        }
    }
    Ok(())
}

fn append_hoop_errors(project: &ProjectDocument, plan: &StitchPlan, report: &mut ValidationReport) {
    let mut extra = 0usize;
    let mut reported = 0usize;
    for stitch in &plan.stitches {
        for point in [stitch.from, stitch.to] {
            if inside(point, project.hoop_width_mm, project.hoop_height_mm) {
                continue;
            }
            if reported < 8 {
                report.errors.push(format!(
                    "A stitch on object {} ends at ({:.2}, {:.2}) mm, outside the {:.1} by {:.1} mm hoop. Move the object, or reduce width and pull compensation, then generate again.",
                    stitch.object_id, point.x, point.y, project.hoop_width_mm, project.hoop_height_mm
                ));
                reported += 1;
            } else {
                extra += 1;
            }
        }
    }
    if extra > 0 {
        report.errors.push(format!(
            "{extra} more stitch endpoints are outside the hoop. Move the object inside the hoop and generate again."
        ));
    }
}

fn inside(point: Point, width: f64, height: f64) -> bool {
    point.is_finite()
        && point.x >= -1e-6
        && point.y >= -1e-6
        && point.x <= width + 1e-6
        && point.y <= height + 1e-6
}

fn ensure_color(plan: &mut StitchPlan, color_index: usize, object_id: u64) {
    let current = plan
        .stitches
        .last()
        .map(|stitch| stitch.color_index)
        .unwrap_or(0);
    if current == color_index {
        return;
    }
    let at = plan
        .stitches
        .last()
        .map(|stitch| stitch.to)
        .unwrap_or_else(|| Point::new(0.0, 0.0));
    plan.stitches.push(Stitch {
        kind: StitchKind::ColorChange,
        from: at,
        to: at,
        color_index,
        object_id,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_project() -> ProjectDocument {
        ProjectDocument {
            format: FORMAT.to_string(),
            format_version: FORMAT_VERSION.to_string(),
            name: "Running line".to_string(),
            hoop_width_mm: 120.0,
            hoop_height_mm: 100.0,
            objects: vec![DesignObject {
                object_id: 1,
                label: "Outline".to_string(),
                kind: "running".to_string(),
                points: vec![Point::new(10.0, 20.0), Point::new(80.0, 20.0)],
                holes: Vec::new(),
                angle_degrees: 0.0,
                density_mm: 2.5,
                column_width_mm: 2.0,
                column_widths_mm: Vec::new(),
                max_stitch_mm: 2.5,
                underlay_spacing_mm: 0.0,
                underlay: Vec::new(),
                centre_run_max_stitch_mm: None,
                edge_run_max_stitch_mm: None,
                pull_compensation_mm: 0.0,
                color_index: 0,
                text: None,
            }],
            review: None,
        }
    }

    #[test]
    fn running_project_generates_and_does_not_release_a_machine() {
        let project = line_project();
        let generation = generate_project(&project).unwrap();
        assert!(generation.plan.sewing_stitch_count() > 0);
        assert!(generation.validation.ok());
        assert_eq!(generation.readiness.state, "geometry_checked");
        assert!(generation.readiness.engineering_export_allowed);
        assert!(!generation.readiness.reviewed_export_allowed);
        assert!(!generation.readiness.machine_release_allowed);
        assert_eq!(
            generation.provenance.generators,
            vec!["running".to_string()]
        );
    }

    #[test]
    fn a_parameter_edit_invalidates_the_geometry_review() {
        let mut project = line_project();
        let before = revision_of(&project);
        stamp_geometry_review(&mut project, "Ada").unwrap();
        assert_eq!(revision_of(&project), before);
        let reviewed = generate_project(&project).unwrap();
        assert_eq!(reviewed.readiness.state, "reviewed");
        assert!(reviewed.readiness.reviewed_export_allowed);
        assert!(!reviewed.readiness.machine_release_allowed);

        project.objects[0].density_mm = 3.0;
        let after = generate_project(&project).unwrap();
        assert_ne!(after.revision, before);
        assert!(!after.readiness.review_matches_revision);
        assert!(!after.readiness.reviewed_export_allowed);
        assert_eq!(after.readiness.state, "geometry_checked");
        assert!(after
            .readiness
            .notes
            .iter()
            .any(|note| note.contains("not")));
    }

    #[test]
    fn motif_and_foreign_formats_are_rejected() {
        let mut project = line_project();
        project.objects[0].kind = "motif".to_string();
        let error = generate_project(&project).unwrap_err();
        assert!(error.message().contains("planned"));
        assert!(error.message().contains("unavailable"));

        let error = parse_project(r#"{"format":"exf","name":"Alpine"}"#).unwrap_err();
        assert!(error.message().contains("EXF"));
    }

    #[test]
    fn review_text_does_not_change_the_revision() {
        let mut project = line_project();
        let bare = revision_of(&project);
        stamp_geometry_review(&mut project, "Ada").unwrap();
        assert_eq!(revision_of(&project), bare);
    }

    #[test]
    fn projects_without_holes_keep_the_locked_revision() {
        assert_eq!(
            revision_of(&line_project()),
            "daaf1d0fed513d354da2cc305c3eafeffeeb7a6028f379b5e18074d128542580"
        );
        let mut value = serde_json::to_value(line_project()).unwrap();
        value.as_object_mut().unwrap().remove("review");
        let canonical = serde_json::to_string(&value).unwrap();
        assert!(!canonical.contains("holes"));
        assert!(!canonical.contains("column_widths_mm"));
    }

    #[test]
    fn variable_satin_widths_round_trip_and_keep_release_disabled() {
        let text = r#"{"format":"embroidery-project","format_version":"1","name":"Taper","hoop_width_mm":80.0,"hoop_height_mm":80.0,"objects":[{"object_id":1,"label":"Column","kind":"satin","points":[{"x":10.0,"y":20.0},{"x":40.0,"y":20.0}],"angle_degrees":0.0,"density_mm":5.0,"column_width_mm":2.0,"column_widths_mm":[2.0,8.0],"max_stitch_mm":3.0,"underlay_spacing_mm":0.0,"underlay":["centre_run","edge_run"],"pull_compensation_mm":0.0,"color_index":0}],"review":null}"#;
        let project = parse_project(text).unwrap();
        assert_eq!(project.objects[0].column_widths_mm, vec![2.0, 8.0]);
        let generation = generate_project(&project).unwrap();
        assert!(
            generation.validation.ok(),
            "{}",
            generation.validation.errors.join("; ")
        );
        assert!(generation.readiness.engineering_export_allowed);
        assert!(!generation.readiness.machine_release_allowed);
        assert!(generation.plan.sewing_stitch_count() > 0);
        let stored = serde_json::to_string(&project).unwrap();
        assert!(stored.contains("column_widths_mm"));
        let mut cleared = project.clone();
        cleared.objects[0].column_widths_mm.clear();
        assert!(!serde_json::to_string(&cleared)
            .unwrap()
            .contains("column_widths_mm"));
    }

    #[test]
    fn a_missing_holes_field_still_parses_and_a_hole_round_trips() {
        let bare = parse_project(
            r#"{"format":"embroidery-project","format_version":"1","name":"Square","hoop_width_mm":40.0,"hoop_height_mm":40.0,"objects":[{"object_id":1,"label":"Field","kind":"fill","points":[{"x":2.0,"y":2.0},{"x":30.0,"y":2.0},{"x":30.0,"y":30.0},{"x":2.0,"y":30.0}],"angle_degrees":0.0,"density_mm":4.0,"column_width_mm":2.0,"max_stitch_mm":8.0,"underlay_spacing_mm":0.0,"underlay":[],"pull_compensation_mm":0.0,"color_index":0}],"review":null}"#,
        )
        .unwrap();
        assert!(bare.objects[0].holes.is_empty());
        let mut with_hole = bare.clone();
        with_hole.objects[0].holes.push(vec![
            Point::new(10.0, 10.0),
            Point::new(18.0, 10.0),
            Point::new(18.0, 18.0),
            Point::new(10.0, 18.0),
        ]);
        let text = serde_json::to_string(&with_hole).unwrap();
        assert!(text.contains("holes"));
        let parsed = parse_project(&text).unwrap();
        assert_eq!(parsed.objects[0].holes.len(), 1);
        let generation = generate_project(&parsed).unwrap();
        assert!(generation.plan.sewing_stitch_count() > 0);
        assert!(!generation.readiness.machine_release_allowed);
        let before = revision_of(&parsed);
        let mut moved = parsed.clone();
        moved.objects[0].holes[0][0].x += 1.0;
        assert_ne!(revision_of(&moved), before);

        let mut running = with_hole.clone();
        running.objects[0].kind = "running".to_string();
        let error = generate_project(&running).unwrap_err();
        assert!(error
            .message()
            .contains("Holes are supported only on a fill"));
    }

    fn text_object(content: &str) -> DesignObject {
        let hash = crate::font_content_sha256();
        DesignObject {
            object_id: 4,
            label: "Lettering".to_string(),
            kind: "text".to_string(),
            points: Vec::new(),
            holes: Vec::new(),
            angle_degrees: 0.0,
            density_mm: 1.5,
            column_width_mm: 1.92,
            column_widths_mm: Vec::new(),
            max_stitch_mm: 3.0,
            underlay_spacing_mm: 0.0,
            underlay: Vec::new(),
            centre_run_max_stitch_mm: None,
            edge_run_max_stitch_mm: None,
            pull_compensation_mm: 0.0,
            color_index: 1,
            text: Some(TextIntent {
                content: content.to_string(),
                font_id: crate::FONT_ID.to_string(),
                font_version: crate::FONT_VERSION.to_string(),
                font_sha256: hash,
                origin_x_mm: 18.0,
                origin_y_mm: 30.0,
                cap_height_mm: 12.0,
                letter_spacing_mm: 0.8,
                word_spacing_mm: 3.6,
                layout: None,
            }),
        }
    }

    #[test]
    fn version_two_text_round_trips_and_version_one_rejects_it() {
        assert!(!serde_json::to_string(&line_project())
            .unwrap()
            .contains("\"text\""));
        let mut project = line_project();
        project.format_version = TEXT_FORMAT_VERSION.to_string();
        project.hoop_width_mm = 160.0;
        project.hoop_height_mm = 160.0;
        project.objects.push(text_object("ALPINE 08"));
        let stored = serde_json::to_string(&project).unwrap();
        let parsed = parse_project(&stored).unwrap();
        assert_eq!(
            parsed.objects[1].text.as_ref().unwrap().content,
            "ALPINE 08"
        );
        assert_eq!(
            parsed.objects[1].text.as_ref().unwrap().font_sha256,
            crate::font_content_sha256()
        );
        let generation = generate_project(&parsed).unwrap();
        assert!(generation.plan.metadata["lettering_text"].contains("ALPINE 08"));
        assert!(generation.plan.metadata["lettering_trace"].contains("U+0041"));
        assert!(generation.plan.metadata["lettering_trace"].contains("U+0020 space"));
        assert!(generation.plan.metadata["lettering_trace"].contains("U+0038"));
        assert_eq!(
            generation.provenance.generators,
            vec!["running".to_string(), "lettering".to_string()]
        );
        assert!(!generation.readiness.machine_release_allowed);
        assert!(generation.readiness.engineering_export_allowed);
        let again = generate_project(&parsed).unwrap();
        assert_eq!(generation.plan.stitches, again.plan.stitches);
        assert_eq!(generation.revision, again.revision);

        let mut changed = parsed.clone();
        changed.objects[1].text.as_mut().unwrap().content = "ALPINE 09".to_string();
        assert_ne!(revision_of(&changed), generation.revision);
        let next = generate_project(&changed).unwrap();
        assert!(next.plan.metadata["lettering_text"].contains("ALPINE 09"));
        assert_ne!(next.plan.stitches, generation.plan.stitches);

        let mut version_one = project.clone();
        version_one.format_version = FORMAT_VERSION.to_string();
        let rejected = parse_project(&serde_json::to_string(&version_one).unwrap()).unwrap_err();
        assert!(rejected.message().contains("version 1"));
        assert!(rejected.message().contains("was not dropped"));

        let mut lowercase = parsed.clone();
        lowercase.objects[1].text.as_mut().unwrap().content = "Alpine".to_string();
        let before = serde_json::to_string(&lowercase).unwrap();
        let error = generate_project(&lowercase).unwrap_err();
        assert_eq!(serde_json::to_string(&lowercase).unwrap(), before);
        assert!(error.message().contains("U+006C"));
        assert!(error.message().contains("left unchanged"));
    }
}
