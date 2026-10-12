//! Bounded name-drop over one native text object.
//!
//! The template is cloned. Only the selected text content changes. Stitches
//! still come from [`crate::generate_project`]. This module does not shrink
//! type, import fonts, or certify a machine file.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lettering::{font_content_sha256, FONT_ID, FONT_VERSION};
use crate::lettering_v2::{font_v2_content_sha256, layout_text, FONT_VERSION_2};
use crate::model::{Point, StitchKind, StitchPlan};
use crate::project::{generate_project, parse_project, revision_of, Generation, ProjectDocument};
use crate::sha256::sha256_hex;

pub const PROFILE_NATIVE_HOOP: &str = "native-hoop-v1";
pub const MAX_BATCH_ROWS: usize = 100;

const ALLOWED_FIELDS: &[&str] = &[
    "profile_id",
    "object_id",
    "placement",
    "template",
    "items",
    "csv",
    "hoop_width_mm",
    "hoop_height_mm",
    "cancel_from",
    "item_id",
    "expected_snapshot_id",
    "presented_dst_sha256",
    "include_dst",
    "scale_confirmed",
    "source_width_mm",
    "source_x_mm",
    "source_y_mm",
    "source_page",
];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlacementMm {
    pub min_x_mm: f64,
    pub min_y_mm: f64,
    pub max_x_mm: f64,
    pub max_y_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchRow {
    pub item_id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct BatchInput {
    pub profile_id: String,
    pub object_id: u64,
    pub placement: PlacementMm,
    pub template: ProjectDocument,
    pub rows: Vec<BatchRow>,
    pub hoop_width_mm: Option<f64>,
    pub hoop_height_mm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchCounts {
    pub queued: usize,
    /// Synchronous generation finishes a row before it returns. The count is
    /// still reported so a caller can show queued, running, completed, failed,
    /// and cancelled separately.
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemOutcome {
    pub item_id: String,
    pub text: String,
    pub status: String,
    pub error: Option<String>,
    pub project: Option<Value>,
    pub project_revision: Option<String>,
    pub lettering_text: Option<String>,
    pub glyph_codes: Vec<String>,
    pub font_id: Option<String>,
    pub font_version: Option<String>,
    pub font_sha256: Option<String>,
    pub text_sewing_count: u32,
    pub other_sewing_sha256: Option<String>,
    pub travel_count: u32,
    pub machine_release_allowed: bool,
    #[serde(skip)]
    pub plan: Option<StitchPlan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub snapshot_id: String,
    pub template_revision: String,
    pub table_sha256: String,
    pub profile_id: String,
    pub object_id: u64,
    pub font_id: String,
    pub font_version: String,
    pub font_sha256: String,
    pub placement: PlacementMm,
    pub status: String,
    pub certifies_batch: bool,
    pub counts: BatchCounts,
    pub items: Vec<ItemOutcome>,
    pub machine_release_allowed: bool,
    pub message: String,
}

pub fn parse_batch_input(value: &Value) -> Result<BatchInput, String> {
    let Some(map) = value.as_object() else {
        return Err("The batch request must be a JSON object. Nothing was generated.".to_string());
    };
    for key in map.keys() {
        if !ALLOWED_FIELDS.contains(&key.as_str()) {
            return Err(format!(
                "The batch request field {key} is not part of this name drop. Nothing was generated."
            ));
        }
    }
    let profile_id = required_string(map, "profile_id")?;
    let object_id = required_u64(map, "object_id")?;
    let placement = required_placement(map.get("placement"))?;
    let template_value = map.get("template").ok_or_else(|| {
        "The batch needs the open project as its template. Nothing was generated.".to_string()
    })?;
    let template_json = serde_json::to_string(template_value).map_err(|_| {
        "The template could not be read. Nothing was generated. The template was not changed."
            .to_string()
    })?;
    let template = parse_project(&template_json).map_err(|error| {
        format!(
            "{} Nothing was generated. The template was not changed.",
            error.message()
        )
    })?;
    let rows = rows_from(map)?;
    let hoop_width_mm = optional_finite(map, "hoop_width_mm")?;
    let hoop_height_mm = optional_finite(map, "hoop_height_mm")?;
    Ok(BatchInput {
        profile_id,
        object_id,
        placement,
        template,
        rows,
        hoop_width_mm,
        hoop_height_mm,
    })
}

pub fn parse_name_csv(text: &str) -> Result<Vec<BatchRow>, String> {
    if text.starts_with('\u{feff}') {
        return Err(
            "The CSV must be UTF-8 without a byte order mark. Nothing was generated.".to_string(),
        );
    }
    if text.contains('\0') {
        return Err("The CSV contains a null character. Nothing was generated.".to_string());
    }
    let records = csv_records(text)?;
    if records.first().map(String::as_str) != Some("item_id,text") {
        return Err("The CSV header must be item_id,text. Nothing was generated.".to_string());
    }
    if records.len() == 1 {
        return Err(
            "The name table is empty. Add a row and submit it again. Nothing was generated."
                .to_string(),
        );
    }
    let mut rows = Vec::with_capacity(records.len() - 1);
    for (index, line) in records.iter().skip(1).enumerate() {
        if line.is_empty() {
            return Err(format!(
                "CSV row {} is empty. Remove the blank line and submit the table again. Nothing was generated.",
                index + 2
            ));
        }
        let (item_id, text) =
            parse_csv_row(line).map_err(|message| format!("{message} Nothing was generated."))?;
        reject_item_id(&item_id)?;
        rows.push(BatchRow { item_id, text });
    }
    reject_row_count(&rows)?;
    reject_duplicate_ids(&rows)?;
    Ok(rows)
}

pub fn prepare_batch(input: &BatchInput) -> Result<BatchReport, String> {
    let ready = inspect(input)?;
    let items = ready
        .rows
        .iter()
        .map(|row| empty_item(&row.item_id, &row.text, "queued", None))
        .collect();
    Ok(report(
        &ready,
        "prepared",
        false,
        items,
        "The table is queued. Nothing has been generated. Machine release remains unavailable."
            .to_string(),
    ))
}

pub fn generate_one(input: &BatchInput, item_id: &str) -> Result<BatchReport, String> {
    let ready = inspect(input)?;
    let Some(index) = ready.rows.iter().position(|row| row.item_id == item_id) else {
        return Err(format!(
            "{item_id} is not in this table. Nothing was generated for it. The template was not changed."
        ));
    };
    let item = generate_row(&ready, index);
    Ok(report(
        &ready,
        "item",
        false,
        vec![item],
        "One row was generated. This response is not a completion manifest. Machine release remains unavailable.".to_string(),
    ))
}

pub fn run_batch(input: &BatchInput, cancel_from: Option<usize>) -> Result<BatchReport, String> {
    let ready = inspect(input)?;
    if let Some(index) = cancel_from {
        if index > ready.rows.len() {
            return Err(
                "The cancellation point is past the end of the table. Nothing was generated. The template was not changed.".to_string(),
            );
        }
    }
    let mut items = Vec::with_capacity(ready.rows.len());
    for index in 0..ready.rows.len() {
        if cancel_from.is_some_and(|stop| index >= stop) {
            let row = &ready.rows[index];
            items.push(empty_item(
                &row.item_id,
                &row.text,
                "cancelled",
                Some(
                    "This row was cancelled before generation. It is not part of a completion manifest."
                        .to_string(),
                ),
            ));
        } else {
            items.push(generate_row(&ready, index));
        }
    }
    let certifies = cancel_from.is_none() && items.iter().all(|item| item.status == "completed");
    let message = if certifies {
        "Every row has an engineering result. Machine release remains unavailable.".to_string()
    } else if cancel_from.is_some() {
        "The batch was cancelled or stopped early. No completion manifest was issued. Machine release remains unavailable.".to_string()
    } else {
        "The batch is incomplete. A failed row blocks the completion manifest. Successful rows can still be inspected. Machine release remains unavailable.".to_string()
    };
    let status = if certifies { "complete" } else { "incomplete" };
    Ok(report(&ready, status, certifies, items, message))
}

fn generate_row(ready: &Ready<'_>, index: usize) -> ItemOutcome {
    let row = &ready.rows[index];
    let mut derived = ready.template.clone();
    {
        let Some(object) = derived
            .objects
            .iter_mut()
            .find(|object| object.object_id == ready.object_id)
        else {
            return failed(
                row,
                "The selected text object disappeared. The template was not changed.",
            );
        };
        let Some(text) = object.text.as_mut() else {
            return failed(
                row,
                "The selected object has no text. The template was not changed.",
            );
        };
        text.content = row.text.clone();
    }
    let generation = match generate_project(&derived) {
        Ok(generation) => generation,
        Err(error) => {
            return failed(
                row,
                &format!(
                    "Item {} kept the entered text. {} The template was not changed.",
                    row.item_id,
                    error.message()
                ),
            );
        }
    };
    if !generation.readiness.engineering_export_allowed || !generation.validation.ok() {
        let detail = if generation.validation.errors.is_empty() {
            "The design is not an engineering export.".to_string()
        } else {
            generation.validation.errors.join(" ")
        };
        return failed(
            row,
            &format!("{detail} Edit the design and generate this row again. The template was not changed."),
        );
    }
    if let Err(message) = sewing_fits(&generation, ready.object_id, ready.placement) {
        return failed(row, &message);
    }
    let lettering_text = generation
        .plan
        .metadata
        .get("lettering_text")
        .cloned()
        .unwrap_or_default();
    let trace = generation
        .plan
        .metadata
        .get("lettering_trace")
        .cloned()
        .unwrap_or_default();
    let expected = format!("object {} {}", ready.object_id, row.text);
    if lettering_text != expected && !lettering_text.lines().any(|line| line == expected) {
        return failed(
            row,
            "The generated lettering text does not match the entered text. The row was not substituted. The template was not changed.",
        );
    }
    let glyph_codes = match glyph_codes(&trace, ready.object_id, &row.text, &ready.font_version) {
        Ok(codes) => codes,
        Err(message) => return failed(row, &message),
    };
    let project = match serde_json::to_value(&derived) {
        Ok(project) => project,
        Err(_) => {
            return failed(
                row,
                "The derived project could not be stored. The template was not changed.",
            );
        }
    };
    let text = derived
        .objects
        .iter()
        .find(|object| object.object_id == ready.object_id)
        .and_then(|object| object.text.as_ref())
        .expect("selected text");
    ItemOutcome {
        item_id: row.item_id.clone(),
        text: row.text.clone(),
        status: "completed".to_string(),
        error: None,
        project: Some(project),
        project_revision: Some(generation.revision),
        lettering_text: Some(expected),
        glyph_codes,
        font_id: Some(text.font_id.clone()),
        font_version: Some(text.font_version.clone()),
        font_sha256: Some(text.font_sha256.clone()),
        text_sewing_count: sewing_count(&generation.plan, ready.object_id),
        other_sewing_sha256: Some(other_sewing_sha256(&generation.plan, ready.object_id)),
        travel_count: travel_count(&generation.plan),
        machine_release_allowed: false,
        plan: Some(generation.plan),
    }
}

fn sewing_fits(
    generation: &Generation,
    object_id: u64,
    placement: PlacementMm,
) -> Result<(), String> {
    let mut saw = false;
    for stitch in &generation.plan.stitches {
        if stitch.object_id != object_id || !stitch.kind.is_sewing() {
            continue;
        }
        saw = true;
        for point in [stitch.from, stitch.to] {
            if !point_inside(point, placement) {
                return Err(format!(
                    "The name does not fit the placement area. A sewing stitch is at ({:.2}, {:.2}) mm, outside {:.2}–{:.2} mm by {:.2}–{:.2} mm. Edit the text or the placement area. The lettering size was not changed. The template was not changed.",
                    point.x,
                    point.y,
                    placement.min_x_mm,
                    placement.max_x_mm,
                    placement.min_y_mm,
                    placement.max_y_mm
                ));
            }
        }
    }
    if !saw {
        return Err(
            "The text produced no sewing stitches inside the placement area. Edit the text and generate it again. The template was not changed."
                .to_string(),
        );
    }
    Ok(())
}

fn glyph_codes(
    trace: &str,
    object_id: u64,
    text: &str,
    font_version: &str,
) -> Result<Vec<String>, String> {
    if text.contains('\n') || text.contains('\r') {
        return glyph_codes_by_line(trace, object_id, text, font_version);
    }
    let layout;
    let source: &str = if font_version == FONT_VERSION_2 {
        layout = layout_text(text);
        &layout
    } else {
        text
    };
    Ok(collect_glyph_codes(trace, object_id, source)?.0)
}

/// A line break is not a glyph. Each derived line restarts its glyph index at 0.
fn glyph_codes_by_line(
    trace: &str,
    object_id: u64,
    text: &str,
    font_version: &str,
) -> Result<Vec<String>, String> {
    let lines = crate::lettering_layout::derive_lines(text)
        .map_err(|error| format!("{} The template was not changed.", error.message()))?;
    let mut codes = Vec::new();
    let mut from = 0usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let normalized;
        let source: &str = if font_version == FONT_VERSION_2 {
            normalized = layout_text(&line);
            &normalized
        } else {
            &line
        };
        let (found, consumed) = collect_glyph_codes(&trace[from..], object_id, source)?;
        from += consumed;
        codes.extend(found);
    }
    Ok(codes)
}

fn collect_glyph_codes(
    trace: &str,
    object_id: u64,
    source: &str,
) -> Result<(Vec<String>, usize), String> {
    let mut codes = Vec::new();
    let mut from = 0usize;
    for (index, character) in source.chars().enumerate() {
        let code = format!("U+{:04X}", character as u32);
        let marker = format!("object {object_id} glyph {index} {code}");
        let Some(found) = trace[from..].find(&marker) else {
            return Err(format!(
                "The glyph check did not find {code} at position {}. The entered text was not substituted. The template was not changed.",
                index + 1
            ));
        };
        from += found + marker.len();
        codes.push(code);
    }
    Ok((codes, from))
}

fn point_inside(point: Point, placement: PlacementMm) -> bool {
    point.x >= placement.min_x_mm - 1e-6
        && point.x <= placement.max_x_mm + 1e-6
        && point.y >= placement.min_y_mm - 1e-6
        && point.y <= placement.max_y_mm + 1e-6
}

fn sewing_count(plan: &StitchPlan, object_id: u64) -> u32 {
    plan.stitches
        .iter()
        .filter(|stitch| stitch.object_id == object_id && stitch.kind.is_sewing())
        .count() as u32
}

fn travel_count(plan: &StitchPlan) -> u32 {
    plan.stitches
        .iter()
        .filter(|stitch| !stitch.kind.is_sewing())
        .count() as u32
}

fn other_sewing_sha256(plan: &StitchPlan, object_id: u64) -> String {
    let mut body = String::new();
    for stitch in &plan.stitches {
        if stitch.object_id == object_id || !stitch.kind.is_sewing() {
            continue;
        }
        body.push_str(&format!(
            "{} {} {:.6} {:.6} {:.6} {:.6}\n",
            stitch.object_id,
            kind_name(stitch.kind),
            stitch.from.x,
            stitch.from.y,
            stitch.to.x,
            stitch.to.y
        ));
    }
    sha256_hex(body.as_bytes())
}

fn kind_name(kind: StitchKind) -> &'static str {
    match kind {
        StitchKind::Running => "running",
        StitchKind::Satin => "satin",
        StitchKind::Fill => "fill",
        StitchKind::Motif => "motif",
        StitchKind::Applique => "applique",
        StitchKind::Jump => "jump",
        StitchKind::Trim => "trim",
        StitchKind::ColorChange => "color_change",
    }
}

struct Ready<'a> {
    template: &'a ProjectDocument,
    rows: &'a [BatchRow],
    object_id: u64,
    placement: PlacementMm,
    snapshot_id: String,
    template_revision: String,
    table_sha256: String,
    font_version: String,
    font_sha256: String,
}

fn inspect(input: &BatchInput) -> Result<Ready<'_>, String> {
    if input.profile_id != PROFILE_NATIVE_HOOP {
        let message = if input.profile_id.is_empty() {
            "Choose the native-hoop-v1 profile. Nothing was generated. The template was not changed.".to_string()
        } else {
            format!(
                "This name drop uses the native-hoop-v1 profile. {} is a different batch. Nothing was generated. The template was not changed.",
                input.profile_id
            )
        };
        return Err(message);
    }
    if let Some(width) = input.hoop_width_mm {
        if (width - input.template.hoop_width_mm).abs() > 1e-6 {
            return Err(
                "The profile hoop width does not match the template. Nothing was generated. The template was not changed.".to_string(),
            );
        }
    }
    if let Some(height) = input.hoop_height_mm {
        if (height - input.template.hoop_height_mm).abs() > 1e-6 {
            return Err(
                "The profile hoop height does not match the template. Nothing was generated. The template was not changed.".to_string(),
            );
        }
    }
    let Some(object) = input
        .template
        .objects
        .iter()
        .find(|object| object.object_id == input.object_id)
    else {
        return Err(
            "The selected text object is not in the template. Nothing was generated. The template was not changed.".to_string(),
        );
    };
    if object.kind != "text" {
        return Err(format!(
            "Object {} is {}, not text. Select the lettering object. Nothing was generated. The template was not changed.",
            input.object_id, object.kind
        ));
    }
    let Some(text) = object.text.as_ref() else {
        return Err(
            "The selected object has no text intent. Nothing was generated. The template was not changed.".to_string(),
        );
    };
    let (font_version, pinned) = accepted_font(text)?;
    if !placement_inside(
        input.placement,
        input.template.hoop_width_mm,
        input.template.hoop_height_mm,
    ) {
        return Err(
            "The placement area must sit inside the template hoop and have a finite width and height. Nothing was generated. The template was not changed.".to_string(),
        );
    }
    reject_row_count(&input.rows)?;
    reject_duplicate_ids(&input.rows)?;
    for row in &input.rows {
        reject_item_id(&row.item_id)?;
    }
    let template_revision = revision_of(&input.template);
    let table_json = serde_json::to_string(&input.rows).expect("row JSON");
    let table_sha256 = sha256_hex(table_json.as_bytes());
    let snapshot = SnapshotBody {
        profile_id: PROFILE_NATIVE_HOOP,
        template_revision: &template_revision,
        object_id: input.object_id,
        font_id: FONT_ID,
        font_version: &font_version,
        font_sha256: &pinned,
        placement: &input.placement,
        items: &input.rows,
    };
    let snapshot_json = serde_json::to_string(&snapshot).expect("snapshot JSON");
    Ok(Ready {
        template: &input.template,
        rows: &input.rows,
        object_id: input.object_id,
        placement: input.placement,
        snapshot_id: sha256_hex(snapshot_json.as_bytes()),
        template_revision,
        table_sha256,
        font_version,
        font_sha256: pinned,
    })
}

fn accepted_font(text: &crate::project::TextIntent) -> Result<(String, String), String> {
    if text.font_id == FONT_ID && text.font_version == FONT_VERSION_2 {
        let expected = font_v2_content_sha256();
        if text.font_sha256 != expected {
            return Err(
                "The selected text is not pinned to EmbroideryOS Block 01 version 2. The recipe was not replaced. Nothing was generated. The template was not changed.".to_string(),
            );
        }
        return Ok((FONT_VERSION_2.to_string(), expected));
    }
    let pinned = font_content_sha256();
    if text.font_id != FONT_ID || text.font_version != FONT_VERSION || text.font_sha256 != pinned {
        return Err(
            "The selected text is not pinned to EmbroideryOS Block 01 version 1. The recipe was not replaced. Nothing was generated. The template was not changed.".to_string(),
        );
    }
    Ok((FONT_VERSION.to_string(), pinned))
}

#[derive(Serialize)]
struct SnapshotBody<'a> {
    profile_id: &'a str,
    template_revision: &'a str,
    object_id: u64,
    font_id: &'a str,
    font_version: &'a str,
    font_sha256: &'a str,
    placement: &'a PlacementMm,
    items: &'a [BatchRow],
}

fn placement_inside(placement: PlacementMm, width: f64, height: f64) -> bool {
    let values = [
        placement.min_x_mm,
        placement.min_y_mm,
        placement.max_x_mm,
        placement.max_y_mm,
        width,
        height,
    ];
    values.iter().all(|value| value.is_finite())
        && placement.min_x_mm >= -1e-6
        && placement.min_y_mm >= -1e-6
        && placement.max_x_mm <= width + 1e-6
        && placement.max_y_mm <= height + 1e-6
        && placement.max_x_mm - placement.min_x_mm >= 0.2
        && placement.max_y_mm - placement.min_y_mm >= 0.2
}

fn report(
    ready: &Ready<'_>,
    status: &str,
    certifies: bool,
    items: Vec<ItemOutcome>,
    message: String,
) -> BatchReport {
    let counts = BatchCounts {
        queued: items.iter().filter(|item| item.status == "queued").count(),
        running: 0,
        completed: items
            .iter()
            .filter(|item| item.status == "completed")
            .count(),
        failed: items.iter().filter(|item| item.status == "failed").count(),
        cancelled: items
            .iter()
            .filter(|item| item.status == "cancelled")
            .count(),
    };
    debug_assert_eq!(
        counts.queued + counts.running + counts.completed + counts.failed + counts.cancelled,
        items.len()
    );
    BatchReport {
        snapshot_id: ready.snapshot_id.clone(),
        template_revision: ready.template_revision.clone(),
        table_sha256: ready.table_sha256.clone(),
        profile_id: PROFILE_NATIVE_HOOP.to_string(),
        object_id: ready.object_id,
        font_id: FONT_ID.to_string(),
        font_version: ready.font_version.clone(),
        font_sha256: ready.font_sha256.clone(),
        placement: ready.placement,
        status: status.to_string(),
        certifies_batch: certifies,
        counts,
        items,
        machine_release_allowed: false,
        message,
    }
}

fn empty_item(item_id: &str, text: &str, status: &str, error: Option<String>) -> ItemOutcome {
    ItemOutcome {
        item_id: item_id.to_string(),
        text: text.to_string(),
        status: status.to_string(),
        error,
        project: None,
        project_revision: None,
        lettering_text: None,
        glyph_codes: Vec::new(),
        font_id: None,
        font_version: None,
        font_sha256: None,
        text_sewing_count: 0,
        other_sewing_sha256: None,
        travel_count: 0,
        machine_release_allowed: false,
        plan: None,
    }
}

fn failed(row: &BatchRow, message: &str) -> ItemOutcome {
    empty_item(&row.item_id, &row.text, "failed", Some(message.to_string()))
}

fn rows_from(map: &serde_json::Map<String, Value>) -> Result<Vec<BatchRow>, String> {
    let csv = match map.get("csv") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.as_str()),
        Some(_) => {
            return Err("The CSV must be text. Nothing was generated.".to_string());
        }
    };
    let items = map.get("items");
    match (items, csv) {
        (Some(_), Some(_)) => Err(
            "Send either the name table or the CSV, not both. Nothing was generated.".to_string(),
        ),
        (None, None) => Err(
            "The name table is empty. Add a row and submit it again. Nothing was generated."
                .to_string(),
        ),
        (None, Some(text)) => parse_name_csv(text),
        (Some(value), None) => parse_items(value),
    }
}

fn parse_items(value: &Value) -> Result<Vec<BatchRow>, String> {
    let Some(rows) = value.as_array() else {
        return Err("The name table must be a list of rows. Nothing was generated.".to_string());
    };
    if rows.is_empty() {
        return Err(
            "The name table is empty. Add a row and submit it again. Nothing was generated."
                .to_string(),
        );
    }
    let mut parsed = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(object) = row.as_object() else {
            return Err(
                "Each name-drop row must be an item id and text. Nothing was generated."
                    .to_string(),
            );
        };
        for key in object.keys() {
            if key != "item_id" && key != "text" {
                return Err(format!(
                    "The name-drop row field {key} is not accepted. Nothing was generated."
                ));
            }
        }
        let Some(item_id) = object.get("item_id").and_then(Value::as_str) else {
            return Err("Each row needs an item id string. Nothing was generated.".to_string());
        };
        let Some(text) = object.get("text").and_then(Value::as_str) else {
            return Err(
                "Each row needs the entered text as a string. Nothing was generated.".to_string(),
            );
        };
        reject_item_id(item_id)?;
        parsed.push(BatchRow {
            item_id: item_id.to_string(),
            text: text.to_string(),
        });
    }
    reject_row_count(&parsed)?;
    reject_duplicate_ids(&parsed)?;
    Ok(parsed)
}

fn reject_row_count(rows: &[BatchRow]) -> Result<(), String> {
    if rows.is_empty() {
        return Err(
            "The name table is empty. Add a row and submit it again. Nothing was generated."
                .to_string(),
        );
    }
    if rows.len() > MAX_BATCH_ROWS {
        return Err(format!(
            "This name drop accepts {MAX_BATCH_ROWS} rows. The table has {}. Remove rows and submit it again. Nothing was generated.",
            rows.len()
        ));
    }
    Ok(())
}

fn reject_duplicate_ids(rows: &[BatchRow]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for row in rows {
        if !seen.insert(row.item_id.as_str()) {
            return Err(format!(
                "{} is repeated. Item ids must be unique. Duplicate text is allowed under different ids. Nothing was generated.",
                row.item_id
            ));
        }
    }
    Ok(())
}

fn reject_item_id(item_id: &str) -> Result<(), String> {
    let mut chars = item_id.chars();
    let Some(first) = chars.next() else {
        return Err("An item id is empty. Use letters, digits, hyphens, and underscores. Nothing was generated.".to_string());
    };
    let ok = first.is_ascii_alphanumeric()
        && item_id.chars().count() <= 32
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        });
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{item_id} is not a usable item id. Use letters, digits, hyphens, and underscores. Nothing was generated."
        ))
    }
}

/// Record separators are LF and CRLF outside quotes. A break inside quotes stays in the field.
fn csv_records(text: &str) -> Result<Vec<String>, String> {
    let mut records = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;
    let mut saw_comma = false;
    while let Some(character) = chars.next() {
        if in_quotes {
            if character == '"' {
                current.push('"');
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
                continue;
            }
            if character == '\r' {
                if chars.peek() != Some(&'\n') {
                    return Err(
                        "The CSV must use LF or CRLF line breaks. Nothing was generated."
                            .to_string(),
                    );
                }
                current.push('\r');
                current.push('\n');
                chars.next();
                continue;
            }
            current.push(character);
            continue;
        }
        if character == '"' {
            if !saw_comma {
                return Err("The item id must not be quoted. Nothing was generated.".to_string());
            }
            in_quotes = true;
            current.push('"');
            continue;
        }
        if character == '\n' || character == '\r' {
            if character == '\r' {
                if chars.peek() != Some(&'\n') {
                    return Err(
                        "The CSV must use LF or CRLF line breaks. Nothing was generated."
                            .to_string(),
                    );
                }
                chars.next();
            }
            records.push(std::mem::take(&mut current));
            saw_comma = false;
            continue;
        }
        if character == ',' {
            saw_comma = true;
        }
        current.push(character);
    }
    if in_quotes {
        return Err(
            "The quoted text is missing its closing quote. Nothing was generated.".to_string(),
        );
    }
    if !current.is_empty() {
        records.push(current);
    }
    Ok(records)
}

fn parse_csv_row(line: &str) -> Result<(String, String), String> {
    let mut chars = line.chars().peekable();
    let mut item_id = String::new();
    while let Some(character) = chars.peek().copied() {
        if character == ',' {
            break;
        }
        if character == '"' {
            return Err("The item id must not be quoted.".to_string());
        }
        item_id.push(character);
        chars.next();
    }
    if chars.next() != Some(',') {
        return Err("The CSV row needs item_id and text.".to_string());
    }
    let text = if chars.peek() == Some(&'"') {
        chars.next();
        let mut text = String::new();
        loop {
            match chars.next() {
                Some('"') if chars.peek() == Some(&'"') => {
                    chars.next();
                    text.push('"');
                }
                Some('"') if chars.peek().is_none() => break,
                Some('"') => {
                    return Err("The quoted text has trailing characters.".to_string());
                }
                Some(character) => text.push(character),
                None => return Err("The quoted text is missing its closing quote.".to_string()),
            }
        }
        text
    } else {
        let text: String = chars.collect();
        if text.contains('"') {
            return Err("Quote the text when it contains a quotation mark.".to_string());
        }
        text
    };
    Ok((item_id, text))
}

fn required_string(map: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("The batch needs {key}. Nothing was generated."))
}

fn required_u64(map: &serde_json::Map<String, Value>, key: &str) -> Result<u64, String> {
    let Some(value) = map.get(key) else {
        return Err(format!("The batch needs {key}. Nothing was generated."));
    };
    value.as_u64().ok_or_else(|| {
        format!("The batch field {key} must be a whole number. Nothing was generated.")
    })
}

fn optional_finite(map: &serde_json::Map<String, Value>, key: &str) -> Result<Option<f64>, String> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => {
            let Some(value) = number.as_f64() else {
                return Err(format!(
                    "The batch field {key} must be a finite number. Nothing was generated."
                ));
            };
            if value.is_finite() {
                Ok(Some(value))
            } else {
                Err(format!(
                    "The batch field {key} must be a finite number. Nothing was generated."
                ))
            }
        }
        Some(_) => Err(format!(
            "The batch field {key} must be a finite number. Nothing was generated."
        )),
    }
}

fn required_placement(value: Option<&Value>) -> Result<PlacementMm, String> {
    let Some(value) = value else {
        return Err(
            "The batch needs a placement area. Nothing was generated. The template was not changed."
                .to_string(),
        );
    };
    let Some(map) = value.as_object() else {
        return Err(
            "The placement area must be four millimetre bounds. Nothing was generated.".to_string(),
        );
    };
    for key in map.keys() {
        if !matches!(
            key.as_str(),
            "min_x_mm" | "min_y_mm" | "max_x_mm" | "max_y_mm"
        ) {
            return Err(format!(
                "The placement field {key} is not accepted. Nothing was generated."
            ));
        }
    }
    Ok(PlacementMm {
        min_x_mm: placement_number(map, "min_x_mm")?,
        min_y_mm: placement_number(map, "min_y_mm")?,
        max_x_mm: placement_number(map, "max_x_mm")?,
        max_y_mm: placement_number(map, "max_y_mm")?,
    })
}

fn placement_number(map: &serde_json::Map<String, Value>, key: &str) -> Result<f64, String> {
    let Some(value) = map.get(key).and_then(Value::as_f64) else {
        return Err(format!(
            "The placement {key} must be a finite number of millimetres. Nothing was generated."
        ));
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "The placement {key} must be a finite number of millimetres. Nothing was generated."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;
    use crate::project::{DesignObject, TextIntent};
    use std::time::Instant;

    fn text_object(id: u64, content: &str) -> DesignObject {
        DesignObject {
            object_id: id,
            label: "Lettering".to_string(),
            kind: "text".to_string(),
            points: Vec::new(),
            holes: Vec::new(),
            angle_degrees: 0.0,
            density_mm: 4.0,
            column_width_mm: 1.6,
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
                font_id: FONT_ID.to_string(),
                font_version: FONT_VERSION.to_string(),
                font_sha256: font_content_sha256(),
                origin_x_mm: 18.0,
                origin_y_mm: 28.0,
                cap_height_mm: 8.0,
                letter_spacing_mm: 0.8,
                word_spacing_mm: 3.6,
                layout: None,
            }),
        }
    }

    fn badge(content: &str) -> ProjectDocument {
        ProjectDocument {
            format: "embroidery-project".to_string(),
            format_version: "2".to_string(),
            name: "Name drop badge".to_string(),
            hoop_width_mm: 120.0,
            hoop_height_mm: 80.0,
            objects: vec![
                DesignObject {
                    object_id: 1,
                    label: "Outline".to_string(),
                    kind: "running".to_string(),
                    points: vec![
                        Point::new(8.0, 8.0),
                        Point::new(112.0, 8.0),
                        Point::new(112.0, 72.0),
                        Point::new(8.0, 72.0),
                    ],
                    holes: Vec::new(),
                    angle_degrees: 0.0,
                    density_mm: 4.0,
                    column_width_mm: 1.0,
                    column_widths_mm: Vec::new(),
                    max_stitch_mm: 4.0,
                    underlay_spacing_mm: 0.0,
                    underlay: Vec::new(),
                    centre_run_max_stitch_mm: None,
                    edge_run_max_stitch_mm: None,
                    pull_compensation_mm: 0.0,
                    color_index: 0,
                    text: None,
                },
                text_object(2, content),
            ],
            review: None,
        }
    }

    fn placement() -> PlacementMm {
        PlacementMm {
            min_x_mm: 14.0,
            min_y_mm: 22.0,
            max_x_mm: 110.0,
            max_y_mm: 48.0,
        }
    }

    fn input_with(project: ProjectDocument, rows: Vec<BatchRow>) -> BatchInput {
        BatchInput {
            profile_id: PROFILE_NATIVE_HOOP.to_string(),
            object_id: 2,
            placement: placement(),
            template: project,
            rows,
            hoop_width_mm: Some(120.0),
            hoop_height_mm: Some(80.0),
        }
    }

    fn row(id: &str, text: &str) -> BatchRow {
        BatchRow {
            item_id: id.to_string(),
            text: text.to_string(),
        }
    }

    fn stored_text(item: &ItemOutcome) -> String {
        item.project.as_ref().unwrap()["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| object["kind"] == "text")
            .unwrap()["text"]["content"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn twenty_names_and_duplicate_text_are_all_kept() {
        let project = badge("ALPINE 08");
        let before = serde_json::to_string(&project).unwrap();
        let rows: Vec<_> = (1..=20)
            .map(|number| row(&format!("NAME-{number:02}"), &format!("ALPINE {number:02}")))
            .collect();
        let started = Instant::now();
        let report = run_batch(&input_with(project.clone(), rows), None).unwrap();
        let elapsed_ms = started.elapsed().as_millis();
        assert!(elapsed_ms < 60_000, "20-name run took {elapsed_ms} ms");
        assert_eq!(report.items.len(), 20);
        assert!(report.certifies_batch);
        assert_eq!(report.status, "complete");
        assert!(!report.machine_release_allowed);
        assert_eq!(report.counts.completed, 20);
        assert_eq!(report.counts.failed, 0);
        assert_eq!(report.counts.cancelled, 0);
        assert_eq!(report.counts.queued, 0);
        assert_eq!(report.counts.running, 0);
        assert_eq!(report.font_sha256, font_content_sha256());
        for (index, item) in report.items.iter().enumerate() {
            let number = index + 1;
            assert_eq!(item.item_id, format!("NAME-{number:02}"));
            assert_eq!(item.text, format!("ALPINE {number:02}"));
            assert_eq!(item.status, "completed");
            assert_eq!(stored_text(item), item.text);
            assert_eq!(
                item.font_sha256.as_deref(),
                Some(font_content_sha256().as_str())
            );
            assert!(item.glyph_codes.iter().any(|code| code == "U+0020"));
            assert!(item
                .plan
                .as_ref()
                .unwrap()
                .stitches
                .iter()
                .any(|stitch| { stitch.object_id == 1 && stitch.kind.is_sewing() }));
            let outline = item.project.as_ref().unwrap()["objects"][0].clone();
            assert_eq!(outline["kind"], "running");
            assert_eq!(outline["points"][0]["y"], 8.0);
        }
        assert_eq!(serde_json::to_string(&project).unwrap(), before);

        let duplicate = run_batch(
            &input_with(
                badge("ALPINE 08"),
                vec![
                    row("ITEM-001", "ALPINE 08"),
                    row("ITEM-002", "ALPINE 09"),
                    row("ITEM-003", "O'NEIL-08"),
                    row("ITEM-004", "ALPINE 08"),
                ],
            ),
            None,
        )
        .unwrap();
        assert_eq!(duplicate.items.len(), 4);
        assert!(duplicate.certifies_batch);
        assert_eq!(duplicate.items[0].text, "ALPINE 08");
        assert_eq!(duplicate.items[3].text, "ALPINE 08");
        assert_ne!(duplicate.items[0].item_id, duplicate.items[3].item_id);
        assert_eq!(
            duplicate.items[0].project_revision,
            duplicate.items[3].project_revision
        );
        assert_eq!(
            duplicate.items[0].plan.as_ref().unwrap().stitches,
            duplicate.items[3].plan.as_ref().unwrap().stitches
        );
        assert_ne!(
            duplicate.items[0].project_revision,
            duplicate.items[1].project_revision
        );
        assert!(duplicate.items[2]
            .glyph_codes
            .iter()
            .any(|code| code == "U+0027"));
        assert!(duplicate.items[2]
            .glyph_codes
            .iter()
            .any(|code| code == "U+002D"));
        assert_eq!(stored_text(&duplicate.items[2]), "O'NEIL-08");
    }

    #[test]
    fn repeated_generation_matches_and_other_objects_stay_put() {
        let input = input_with(badge("ALPINE 08"), vec![row("ITEM-001", "ALPINE 08")]);
        let first = generate_one(&input, "ITEM-001").unwrap();
        let second = generate_one(&input, "ITEM-001").unwrap();
        assert_eq!(
            first.items[0].project_revision,
            second.items[0].project_revision
        );
        assert_eq!(
            first.items[0].plan.as_ref().unwrap().stitches,
            second.items[0].plan.as_ref().unwrap().stitches
        );
        let changed = run_batch(
            &input_with(
                badge("ALPINE 08"),
                vec![row("ITEM-001", "ALPINE 08"), row("ITEM-002", "ALPINE 09")],
            ),
            None,
        )
        .unwrap();
        assert_eq!(
            changed.items[0].other_sewing_sha256,
            changed.items[1].other_sewing_sha256
        );
        assert_eq!(
            changed.items[0].project.as_ref().unwrap()["objects"][0],
            changed.items[1].project.as_ref().unwrap()["objects"][0]
        );
        assert_ne!(changed.items[0].text_sewing_count, 0);
        let travel_note = format!(
            "connector travel counts are {} and {}",
            changed.items[0].travel_count, changed.items[1].travel_count
        );
        assert!(
            changed.items[0].travel_count > 0 && changed.items[1].travel_count > 0,
            "{travel_note}"
        );
    }

    #[test]
    fn negative_tables_preserve_the_template() {
        let project = badge("ALPINE 08");
        let before = serde_json::to_string(&project).unwrap();
        let mut input = input_with(
            project.clone(),
            vec![
                row("ITEM-001", "Alpine"),
                row("ITEM-002", "CAFÉ"),
                row("ITEM-003", "O\u{2019}NEIL"),
                row("ITEM-004", ""),
                row("ITEM-005", "   "),
                row("ITEM-006", "A,B"),
            ],
        );
        input.placement.max_x_mm = 110.0;
        let failed = run_batch(&input, None).unwrap();
        assert!(!failed.certifies_batch);
        assert_eq!(failed.items.len(), 6);
        assert!(failed.items.iter().all(|item| item.status == "failed"));
        assert!(failed.items.iter().all(|item| item.project.is_none()));
        assert_eq!(failed.items[0].text, "Alpine");
        assert!(failed.items[0].error.as_deref().unwrap().contains("U+006C"));
        assert_eq!(failed.items[1].text, "CAFÉ");
        assert!(failed.items[1].error.as_deref().unwrap().contains("U+00C9"));
        assert!(failed.items[2].error.as_deref().unwrap().contains("U+2019"));
        assert_eq!(failed.items[2].text, "O\u{2019}NEIL");
        assert_eq!(failed.items[5].text, "A,B");
        assert!(failed.items[5].error.as_deref().unwrap().contains("U+002C"));
        assert_eq!(serde_json::to_string(&project).unwrap(), before);

        input.rows = vec![row("ITEM-001", "ALPINE 08")];
        input.placement.max_x_mm = 20.0;
        let overflow = run_batch(&input, None).unwrap();
        assert_eq!(overflow.items[0].status, "failed");
        assert!(overflow.items[0]
            .error
            .as_deref()
            .unwrap()
            .contains("was not changed"));
        assert!(overflow.items[0].project.is_none());

        let mut wrong_font = badge("ALPINE 08");
        wrong_font.objects[1].text.as_mut().unwrap().font_sha256 = "abc".to_string();
        let font_error = run_batch(
            &input_with(wrong_font, vec![row("ITEM-001", "ALPINE 08")]),
            None,
        );
        assert!(font_error.unwrap_err().contains("not pinned"));

        let mut wrong_profile = input_with(badge("ALPINE 08"), vec![row("ITEM-001", "ALPINE 08")]);
        wrong_profile.profile_id = "generic-flat-120-v1".to_string();
        assert!(run_batch(&wrong_profile, None)
            .unwrap_err()
            .contains("different batch"));

        wrong_profile.profile_id.clear();
        assert!(run_batch(&wrong_profile, None)
            .unwrap_err()
            .contains("native-hoop-v1"));

        let mut missing = input_with(badge("ALPINE 08"), vec![row("ITEM-001", "ALPINE 08")]);
        missing.object_id = 9;
        assert!(run_batch(&missing, None)
            .unwrap_err()
            .contains("not in the template"));

        let duplicate = run_batch(
            &input_with(
                badge("ALPINE 08"),
                vec![row("ITEM-001", "ALPINE 08"), row("ITEM-001", "ALPINE 09")],
            ),
            None,
        );
        assert!(duplicate.unwrap_err().contains("repeated"));

        let slash = parse_name_csv("item_id,text\nITEM/001,ALPINE 08");
        assert!(slash.unwrap_err().contains("not a usable item id"));

        let header = parse_name_csv("name,text\nITEM-001,ALPINE 08");
        assert!(header.unwrap_err().contains("item_id,text"));

        let mut too_many = Vec::new();
        for number in 1..=101 {
            too_many.push(row(&format!("R{number:03}"), "A1"));
        }
        let limit = run_batch(&input_with(badge("A1"), too_many), None).unwrap_err();
        assert!(limit.contains("100"), "{limit}");
        assert!(limit.contains("101"), "{limit}");

        let quoted =
            parse_name_csv("item_id,text\nITEM-005,\"A,B\"\nITEM-006,\"O'NEIL-08\"\n").unwrap();
        assert_eq!(quoted[0].text, "A,B");
        assert_eq!(quoted[1].text, "O'NEIL-08");

        let both = parse_batch_input(&serde_json::json!({
            "profile_id": PROFILE_NATIVE_HOOP,
            "object_id": 2,
            "placement": {"min_x_mm": 14.0, "min_y_mm": 22.0, "max_x_mm": 110.0, "max_y_mm": 48.0},
            "template": badge("ALPINE 08"),
            "items": [{"item_id": "ITEM-001", "text": "ALPINE 08"}],
            "csv": "item_id,text\nITEM-001,ALPINE 08\n"
        }));
        assert!(both.unwrap_err().contains("not both"));

        let mut version_one = badge("ALPINE 08");
        version_one.format_version = "1".to_string();
        let rejected = parse_batch_input(&serde_json::json!({
            "profile_id": PROFILE_NATIVE_HOOP,
            "object_id": 2,
            "placement": {"min_x_mm": 14.0, "min_y_mm": 22.0, "max_x_mm": 110.0, "max_y_mm": 48.0},
            "template": version_one,
            "items": [{"item_id": "ITEM-001", "text": "ALPINE 08"}]
        }))
        .unwrap_err();
        assert!(rejected.contains("version 1"), "{rejected}");
        assert!(rejected.contains("was not changed"), "{rejected}");
        assert_eq!(serde_json::to_string(&project).unwrap(), before);
    }

    #[test]
    fn the_row_limit_generates_and_cancellation_does_not_certify() {
        let rows: Vec<_> = (1..=100)
            .map(|number| row(&format!("R{number:03}"), "A1"))
            .collect();
        let report = run_batch(&input_with(badge("A1"), rows), None).unwrap();
        assert_eq!(report.items.len(), 100);
        assert!(report.certifies_batch);
        assert_eq!(report.counts.completed, 100);
        assert_eq!(
            report.items[0].project_revision,
            report.items[99].project_revision
        );
        assert_ne!(report.items[0].item_id, report.items[99].item_id);

        let stopped = run_batch(
            &input_with(
                badge("ALPINE 08"),
                vec![
                    row("ITEM-001", "ALPINE 08"),
                    row("ITEM-002", "ALPINE 09"),
                    row("ITEM-003", "O'NEIL-08"),
                ],
            ),
            Some(1),
        )
        .unwrap();
        assert!(!stopped.certifies_batch);
        assert_eq!(stopped.status, "incomplete");
        assert_eq!(stopped.counts.completed, 1);
        assert_eq!(stopped.counts.cancelled, 2);
        assert_eq!(stopped.counts.running, 0);
        assert!(stopped.items[0].project.is_some());
        assert!(stopped.items[1].project.is_none());
        assert_eq!(stopped.items[1].status, "cancelled");
        assert_eq!(stopped.items[1].text, "ALPINE 09");

        let cleared = run_batch(
            &input_with(badge("ALPINE 08"), vec![row("ITEM-001", "ALPINE 08")]),
            Some(0),
        )
        .unwrap();
        assert!(!cleared.certifies_batch);
        assert_eq!(cleared.counts.cancelled, 1);
        assert!(cleared.items[0].project.is_none());
    }

    #[test]
    fn the_same_table_has_one_snapshot_and_a_text_edit_changes_it() {
        let csv = "item_id,text\nITEM-001,ALPINE 08\nITEM-004,ALPINE 08\n";
        let from_csv = parse_batch_input(&serde_json::json!({
            "profile_id": PROFILE_NATIVE_HOOP,
            "object_id": 2,
            "placement": {"min_x_mm": 14.0, "min_y_mm": 22.0, "max_x_mm": 110.0, "max_y_mm": 48.0},
            "template": badge("ALPINE 08"),
            "csv": csv
        }))
        .unwrap();
        let from_items = input_with(
            badge("ALPINE 08"),
            vec![row("ITEM-001", "ALPINE 08"), row("ITEM-004", "ALPINE 08")],
        );
        let csv_report = prepare_batch(&from_csv).unwrap();
        let item_report = prepare_batch(&from_items).unwrap();
        assert_eq!(csv_report.snapshot_id, item_report.snapshot_id);
        assert_eq!(csv_report.counts.queued, 2);
        assert!(!csv_report.certifies_batch);
        let mut edited = from_items.clone();
        edited.rows[1].text = "ALPINE 09".to_string();
        let next = prepare_batch(&edited).unwrap();
        assert_ne!(next.snapshot_id, item_report.snapshot_id);
        assert_ne!(next.table_sha256, item_report.table_sha256);
        assert_eq!(next.template_revision, item_report.template_revision);
    }

    #[test]
    fn centre_run_stitches_must_fit_the_same_area() {
        let mut project = badge("HI");
        project.objects[1].underlay = vec!["centre_run".to_string()];
        let report = run_batch(&input_with(project, vec![row("ITEM-001", "HI")]), None).unwrap();
        assert!(
            report.certifies_batch,
            "{}",
            report.items[0].error.as_deref().unwrap_or("")
        );
        assert!(report.items[0].text_sewing_count > 0);
        let trace = &report.items[0].plan.as_ref().unwrap().metadata["lettering_trace"];
        assert!(trace.contains("U+0048"));
        assert!(trace.contains("U+0049"));
    }

    fn badge_v2(content: &str) -> ProjectDocument {
        let mut project = badge(content);
        let text = project.objects[1].text.as_mut().unwrap();
        text.font_version = FONT_VERSION_2.to_string();
        text.font_sha256 = font_v2_content_sha256();
        project
    }

    #[test]
    fn version_two_batch_keeps_mixed_case_accents_and_distinct_sources() {
        let names = [
            ("N-01", "Alpine 08"),
            ("N-02", "Alpine 09"),
            ("N-03", "Café"),
            ("N-04", "Cafe\u{0301}"),
            ("N-05", "François"),
            ("N-06", "Franc\u{0327}ois"),
            ("N-07", "José"),
            ("N-08", "Jose\u{0301}"),
            ("N-09", "Zoë"),
            ("N-10", "Zoe\u{0308}"),
            ("N-11", "Jürgen"),
            ("N-12", "Ju\u{0308}rgen"),
            ("N-13", "García"),
            ("N-14", "Garci\u{0301}a"),
            ("N-15", "Muñoz"),
            ("N-16", "Mun\u{0303}oz"),
            ("N-17", "O\u{2019}Neil"),
            ("N-18", "O'Neil"),
            ("N-19", "Van Laarhoven"),
            ("N-20", "Café"),
        ];
        assert_eq!(names.len(), 20);
        let untouched = badge("ALPINE 08");
        let before_untouched = serde_json::to_string(&untouched).unwrap();
        let template = badge_v2("Alpine 08");
        let before = serde_json::to_string(&template).unwrap();
        let rows: Vec<_> = names.iter().map(|(id, text)| row(id, text)).collect();
        let report = run_batch(&input_with(template.clone(), rows), None).unwrap();
        assert!(
            report.certifies_batch,
            "{}",
            report
                .items
                .iter()
                .filter_map(|item| item.error.clone())
                .collect::<Vec<_>>()
                .join(" | ")
        );
        assert_eq!(report.items.len(), 20);
        assert_eq!(report.font_version, FONT_VERSION_2);
        assert_eq!(report.font_sha256, font_v2_content_sha256());
        assert!(!report.machine_release_allowed);
        assert_eq!(report.status, "complete");
        for (item, (id, text)) in report.items.iter().zip(names) {
            assert_eq!(item.item_id, id);
            assert_eq!(item.text, text);
            assert_eq!(item.status, "completed");
            assert_eq!(stored_text(item), text);
            assert_eq!(item.font_version.as_deref(), Some(FONT_VERSION_2));
            assert_eq!(
                item.font_sha256.as_deref(),
                Some(font_v2_content_sha256().as_str())
            );
            assert!(item.project.is_some());
            assert!(item.plan.is_some());
        }
        let cafe = &report.items[2];
        let cafe_nfd = &report.items[3];
        let cafe_again = &report.items[19];
        assert_ne!(cafe.text, cafe_nfd.text);
        assert_eq!(cafe.glyph_codes, cafe_nfd.glyph_codes);
        assert!(cafe.glyph_codes.iter().any(|code| code == "U+00E9"));
        assert!(cafe_nfd.glyph_codes.iter().all(|code| code != "U+0301"));
        assert_eq!(
            cafe.plan.as_ref().unwrap().stitches,
            cafe_nfd.plan.as_ref().unwrap().stitches
        );
        assert_eq!(
            report.items[4].plan.as_ref().unwrap().stitches,
            report.items[5].plan.as_ref().unwrap().stitches
        );
        assert_ne!(report.items[4].text, report.items[5].text);
        assert_ne!(cafe.project_revision, cafe_nfd.project_revision);
        assert_eq!(cafe.project_revision, cafe_again.project_revision);
        assert_eq!(
            cafe.plan.as_ref().unwrap().stitches,
            cafe_again.plan.as_ref().unwrap().stitches
        );
        assert_ne!(cafe.item_id, cafe_again.item_id);
        assert_ne!(
            report.items[16].plan.as_ref().unwrap().stitches,
            report.items[17].plan.as_ref().unwrap().stitches
        );
        assert!(report.items[16]
            .glyph_codes
            .iter()
            .any(|code| code == "U+2019"));
        assert!(report.items[17]
            .glyph_codes
            .iter()
            .any(|code| code == "U+0027"));
        assert_eq!(serde_json::to_string(&template).unwrap(), before);
        assert_eq!(serde_json::to_string(&untouched).unwrap(), before_untouched);

        let rejected = run_batch(
            &input_with(badge("ALPINE 08"), vec![row("ITEM-001", "Café")]),
            None,
        )
        .unwrap();
        assert!(!rejected.certifies_batch);
        assert_eq!(rejected.items[0].status, "failed");
        assert_eq!(rejected.items[0].text, "Café");
        assert!(rejected.items[0].project.is_none());
        let message = rejected.items[0].error.as_deref().unwrap();
        // Café fails on the first unsupported scalar, lowercase a, before é.
        // Version 1 does not skip ahead or normalize the accent.
        assert!(message.contains("U+0061"), "{message}");
        assert!(message.contains("left unchanged"), "{message}");
        assert!(!message.contains("version 2"), "{message}");

        let mut broken = badge_v2("Alpine");
        broken.objects[1].text.as_mut().unwrap().font_sha256 = font_content_sha256();
        let mismatch = run_batch(&input_with(broken, vec![row("ITEM-001", "Alpine")]), None);
        let message = mismatch.unwrap_err();
        assert!(message.contains("version 2"), "{message}");
        assert!(message.contains("not replaced"), "{message}");
        assert!(message.contains("Nothing was generated"), "{message}");
    }

    fn lined(mut project: ProjectDocument) -> ProjectDocument {
        project.hoop_height_mm = 160.0;
        let text = project.objects[1].text.as_mut().unwrap();
        let cap = text.cap_height_mm;
        text.layout = Some(crate::lettering::LineLayout {
            policy: crate::lettering::LINE_LAYOUT_POLICY.to_string(),
            alignment: "left".to_string(),
            block_width_mm: 100.0,
            first_baseline_mm: text.origin_y_mm + 92.0 * cap / 100.0,
            baseline_spacing_mm: 22.0,
        });
        project
    }

    fn tall(project: ProjectDocument, rows: Vec<BatchRow>) -> BatchInput {
        let mut input = input_with(project, rows);
        input.hoop_height_mm = Some(160.0);
        input.placement = PlacementMm {
            min_x_mm: 8.0,
            min_y_mm: 12.0,
            max_x_mm: 118.0,
            max_y_mm: 150.0,
        };
        input
    }

    fn quote_csv(rows: &[(&str, &str)]) -> String {
        let mut csv = String::from("item_id,text\n");
        for (id, text) in rows {
            csv.push_str(id);
            csv.push_str(",\"");
            csv.push_str(&text.replace('"', "\"\""));
            csv.push_str("\"\n");
        }
        csv
    }

    #[test]
    fn quoted_multiline_csv_keeps_each_item_and_its_source() {
        let example = "item_id,text\nML-001,\"Café\nAlpine 08\"\nML-002,\"Café\nAlpine 09\"\n";
        let parsed = parse_name_csv(example).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].text, "Café\nAlpine 08");
        assert_eq!(parsed[1].text, "Café\nAlpine 09");
        let crlf = parse_name_csv("item_id,text\r\nML-001,\"Café\r\nAlpine 08\"\r\n").unwrap();
        assert_eq!(crlf.len(), 1);
        assert_eq!(crlf[0].text, "Café\r\nAlpine 08");
        assert!(parse_name_csv("item_id,text\nML-001,\"A\rB\"\n")
            .unwrap_err()
            .contains("LF or CRLF"));

        let names = [
            ("ML-001", "Café\nAlpine 08"),
            ("ML-002", "Café\nAlpine 09"),
            ("ML-003", "Cafe\u{0301}\nAlpine 08"),
            ("ML-004", "Café\nAlpine 08"),
            ("ML-005", "Jürgen\nÉquipe"),
            ("ML-006", "O'Neil\nA"),
            ("ML-007", "O\u{2019}Neil\nA"),
            ("ML-008", "Alpha\n\nBeta"),
            ("ML-009", "A\n  \nB"),
            ("ML-010", "North\nRidge\nClub\n08"),
            ("ML-011", "François\n08"),
            ("ML-012", "Franc\u{0327}ois\n08"),
            ("ML-013", "Zoë\nTeam"),
            ("ML-014", "Zoe\u{0308}\nTeam"),
            ("ML-015", "García\n09"),
            ("ML-016", "Garci\u{0301}a\n09"),
            ("ML-017", "Muñoz\n10"),
            ("ML-018", "José\n11"),
            ("ML-019", "One"),
            ("ML-020", "Alpine\nClub"),
        ];
        assert_eq!(names.len(), 20);
        let from_csv = parse_name_csv(&quote_csv(&names)).unwrap();
        assert_eq!(from_csv.len(), 20);
        for (parsed_row, (id, text)) in from_csv.iter().zip(names) {
            assert_eq!(parsed_row.item_id, id);
            assert_eq!(parsed_row.text, text);
        }

        let historical = badge("ALPINE 08");
        let historical_json = serde_json::to_string(&historical).unwrap();
        assert!(!historical_json.contains("\"layout\""));
        let template = lined(badge_v2("Café\nAlpine 08"));
        let before = serde_json::to_string(&template).unwrap();
        let rows: Vec<_> = names.iter().map(|(id, text)| row(id, text)).collect();
        let input = tall(template.clone(), rows);
        let csv_input = tall(template.clone(), from_csv);
        assert_eq!(
            prepare_batch(&input).unwrap().snapshot_id,
            prepare_batch(&csv_input).unwrap().snapshot_id
        );
        let report = run_batch(&input, None).unwrap();
        assert!(
            report.certifies_batch,
            "{}",
            report
                .items
                .iter()
                .filter_map(|item| item.error.clone())
                .collect::<Vec<_>>()
                .join(" | ")
        );
        assert_eq!(report.items.len(), 20);
        assert!(!report.machine_release_allowed);
        assert_eq!(report.font_sha256, font_v2_content_sha256());
        for (item, (id, text)) in report.items.iter().zip(names) {
            assert_eq!(item.item_id, id);
            assert_eq!(item.text, text);
            assert_eq!(item.status, "completed");
            assert_eq!(stored_text(item), text);
            assert!(item.glyph_codes.iter().all(|code| code != "U+000A"));
            let plan = item.plan.as_ref().unwrap();
            assert_eq!(plan.metadata["lettering_text"], format!("object 2 {text}"));
            assert!(plan.metadata["lettering_lines"].contains("alignment left"));
            assert_eq!(
                item.font_sha256.as_deref(),
                Some(font_v2_content_sha256().as_str())
            );
        }
        let first = &report.items[0];
        let decomposed = &report.items[2];
        let duplicate = &report.items[3];
        assert_ne!(first.text, decomposed.text);
        assert_eq!(first.glyph_codes, decomposed.glyph_codes);
        assert_eq!(
            first.plan.as_ref().unwrap().stitches,
            decomposed.plan.as_ref().unwrap().stitches
        );
        assert_ne!(first.project_revision, decomposed.project_revision);
        assert_eq!(first.text, duplicate.text);
        assert_eq!(first.project_revision, duplicate.project_revision);
        assert_ne!(first.item_id, duplicate.item_id);
        assert_ne!(
            report.items[5].plan.as_ref().unwrap().stitches,
            report.items[6].plan.as_ref().unwrap().stitches
        );
        assert!(
            report.items[7].plan.as_ref().unwrap().metadata["lettering_lines"].contains("blank")
        );
        assert!(report.items[8]
            .glyph_codes
            .iter()
            .any(|code| code == "U+0020"));
        assert_eq!(serde_json::to_string(&template).unwrap(), before);
        assert_eq!(serde_json::to_string(&historical).unwrap(), historical_json);

        let mut centre = lined(badge_v2("Café"));
        centre.objects[1]
            .text
            .as_mut()
            .unwrap()
            .layout
            .as_mut()
            .unwrap()
            .alignment = "centre".to_string();
        let centred = run_batch(&tall(centre, vec![row("C-1", "Café\nAlpine 08")]), None).unwrap();
        assert!(centred.certifies_batch, "{:?}", centred.items[0].error);
        let lines = &centred.items[0].plan.as_ref().unwrap().metadata["lettering_lines"];
        assert!(lines.contains("alignment centre"));
        assert!(lines
            .lines()
            .any(|line| line.contains("dx ") && !line.contains("dx 0.000000")));

        let mut overflow = tall(
            lined(badge_v2("A")),
            vec![row("OK-1", "A"), row("OK-2", "A\nB\nC\nD")],
        );
        overflow.placement.max_y_mm = 40.0;
        let stopped = run_batch(&overflow, None).unwrap();
        assert!(!stopped.certifies_batch);
        assert_eq!(stopped.items[0].status, "completed");
        assert!(stopped.items[0].project.is_some());
        assert_eq!(stopped.items[1].status, "failed");
        assert_eq!(stopped.items[1].text, "A\nB\nC\nD");
        assert!(stopped.items[1].project.is_none());
        assert!(stopped.items[1]
            .error
            .as_deref()
            .unwrap()
            .contains("not changed"));

        let bare = run_batch(&input_with(badge("A"), vec![row("ITEM-001", "A\nB")]), None).unwrap();
        assert!(!bare.certifies_batch);
        assert_eq!(bare.items[0].text, "A\nB");
        assert!(bare.items[0].project.is_none());
        let message = bare.items[0].error.as_deref().unwrap();
        assert!(message.contains("U+000A"), "{message}");
        assert!(
            !message.contains(crate::lettering::LINE_LAYOUT_POLICY),
            "{message}"
        );
    }

    #[test]
    fn version_two_descender_failure_keeps_the_admitted_row() {
        // Cap 8 mm, origin y 28 mm, column 1.6 mm, pull 0, no underlay.
        // 'A' lowest rail is about 35.51 mm. The horizontal descender of 'g'
        // is 28 + 118 * 0.08 + 0.8 = 38.24 mm. max_y 36.5 admits A and rejects g.
        let template = badge_v2("A");
        let before = serde_json::to_string(&template).unwrap();
        let mut input = input_with(template, vec![row("ITEM-A", "A"), row("ITEM-G", "g")]);
        input.placement.max_y_mm = 36.5;
        let report = run_batch(&input, None).unwrap();
        assert!(!report.certifies_batch);
        assert_eq!(report.status, "incomplete");
        assert_eq!(
            report.items[0].status, "completed",
            "{:?}",
            report.items[0].error
        );
        assert_eq!(report.items[0].text, "A");
        assert!(report.items[0].project.is_some());
        assert_eq!(report.items[1].status, "failed");
        assert_eq!(report.items[1].text, "g");
        assert!(report.items[1].project.is_none());
        let message = report.items[1].error.as_deref().unwrap();
        assert!(message.contains("was not changed"), "{message}");
        assert!(
            message.contains("lettering size was not changed"),
            "{message}"
        );
        assert_eq!(serde_json::to_string(&input.template).unwrap(), before);
    }
}
