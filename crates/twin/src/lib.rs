//! Design -> existing Rust generators -> exact DST bytes -> raw decoded events.
//! This is a bounded generic-machine demonstration, not a machine controller.
use embroidery_core::{
    font_content_sha256, generate_fill, generate_fill_with_holes, generate_running, generate_satin,
    generate_satin_variable, generate_text, validate, validate_fill_region, ColorStop, FillConfig,
    LineLayout, Point, RunningConfig, SatinConfig, Stitch, StitchKind, StitchPlan, TextRequest,
    ThreadSpec, FONT_ID, FONT_NAME, FONT_VERSION,
};
use embroidery_dst::{
    compile_to_vec, decode_record, encode_record, read_dst_with, ReadOptions, RecordCommand,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PROFILE_ID: &str = "generic-flat-120-v1";
pub const BASE_COMMIT: &str = "98303601416a3b0d68eae37e939031732bd77c45";
const MAX_RECORDS: usize = 60_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesignObject {
    pub id: u64,
    pub label: String,
    pub kind: String,
    pub points: Vec<Point>,
    /// Explicit hole rings. Omitted when empty so older designs stay unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<Point>>,
    pub rgb: [u8; 3],
    pub spacing_mm: f64,
    pub angle_degrees: f64,
    pub max_stitch_mm: f64,
    pub width_mm: f64,
    /// One full column width per centreline point. Empty keeps `width_mm`.
    /// A non-empty list is not replaced by the constant width.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub widths_mm: Vec<f64>,
    pub compensation_mm: f64,
    pub underlay_mm: f64,
    /// Absent keeps the historical meaning of `underlay_mm`: both passes when
    /// that length is positive, and no underlay when it is 0. A present list
    /// selects only those passes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlay: Option<Vec<String>>,
    /// Absent inherits `underlay_mm` when the centre run is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre_run_mm: Option<f64>,
    /// Absent inherits `underlay_mm` when the edge run is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_run_mm: Option<f64>,
    /// Editable lettering. Omitted on geometry objects so older designs stay unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<LetteringText>,
}

/// Entered text pinned to one font recipe. Schema version stays 1.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LetteringText {
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
    /// Schema version stays 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<LineLayout>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub schema_version: u32,
    pub name: String,
    pub revision: u64,
    pub objects: Vec<DesignObject>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub profile_id: String,
    pub design: Design,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub index: usize,
    pub byte_offset: usize,
    pub raw: [u8; 3],
    pub kind: String,
    pub from_units: [i32; 2],
    pub to_units: [i32; 2],
    pub color_index: usize,
    pub start_ms: f64,
    pub end_ms: f64,
    /// Sidecar association, never claimed to be encoded by DST itself.
    pub object_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Trace {
    pub units: &'static str,
    pub decoder: &'static str,
    pub trim_policy: &'static str,
    pub timing: &'static str,
    pub events: Vec<Event>,
    pub record_count: usize,
    pub needle_placements: usize,
    pub duration_ms: f64,
}

fn in_range(v: f64, lo: f64, hi: f64, field: &str) -> Result<(), String> {
    if !v.is_finite() || v < lo || v > hi {
        return Err(format!("{field} must be finite and in [{lo}, {hi}]."));
    }
    Ok(())
}

// Simple-ring admission guard. This is not polygon repair.
// Hole topology is checked separately and only when a fill carries holes.
fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
fn on_segment(a: Point, b: Point, c: Point) -> bool {
    c.x >= a.x.min(b.x) - 1e-9
        && c.x <= a.x.max(b.x) + 1e-9
        && c.y >= a.y.min(b.y) - 1e-9
        && c.y <= a.y.max(b.y) + 1e-9
}
fn intersects(a: Point, b: Point, c: Point, d: Point) -> bool {
    let [o1, o2, o3, o4] = [
        orientation(a, b, c),
        orientation(a, b, d),
        orientation(c, d, a),
        orientation(c, d, b),
    ];
    (o1 * o2 < 0.0 && o3 * o4 < 0.0)
        || (o1.abs() < 1e-9 && on_segment(a, b, c))
        || (o2.abs() < 1e-9 && on_segment(a, b, d))
        || (o3.abs() < 1e-9 && on_segment(c, d, a))
        || (o4.abs() < 1e-9 && on_segment(c, d, b))
}
fn check_ring(points: &[Point]) -> Result<(), String> {
    let mut p = points.to_vec();
    if p.len() > 1 && p.first() == p.last() {
        p.pop();
    }
    if p.len() < 3 {
        return Err("Fill requires a simple polygon with at least three vertices.".into());
    }
    let mut area = 0.0;
    for i in 0..p.len() {
        let ni = (i + 1) % p.len();
        if p[i].distance(p[ni]) < 1e-6 {
            return Err("Fill has a zero-length edge.".into());
        }
        area += p[i].x * p[ni].y - p[ni].x * p[i].y;
        for j in i + 1..p.len() {
            let nj = (j + 1) % p.len();
            if ni == j || nj == i {
                continue;
            }
            if intersects(p[i], p[ni], p[j], p[nj]) {
                return Err("Fill self-intersects. Repair its outline before generation.".into());
            }
        }
    }
    if area.abs() < 1e-6 {
        return Err("Fill has zero area.".into());
    }
    Ok(())
}

fn run_length(specific: Option<f64>, inherited: f64, label: &str) -> Result<f64, String> {
    let length = specific.unwrap_or(inherited);
    if !length.is_finite() || !(0.2..=12.1).contains(&length) {
        return Err(format!(
            "{label} maximum stitch length must be from 0.2 mm to 12.1 mm. Reduce that length and compile again. The other pass was not substituted."
        ));
    }
    Ok(length)
}

/// Resolve satin underlay. An absent list inherits the historical `underlay_mm`
/// switch: positive sews centre run then edge run at that length, and 0 sews
/// neither. A present list sews only the named passes. An absent per-pass
/// length inherits `underlay_mm`.
fn satin_passes(o: &DesignObject) -> Result<(Option<f64>, Option<f64>), String> {
    if o.kind != "satin" && o.kind != "text" {
        if o.underlay.is_some() || o.centre_run_mm.is_some() || o.edge_run_mm.is_some() {
            return Err(format!(
                "Object {} carries a satin underlay selection. Remove centre_run, edge_run, and their lengths, or change the object to satin or text.",
                o.id
            ));
        }
        return Ok((None, None));
    }
    for (label, length) in [("Centre-run", o.centre_run_mm), ("Edge-run", o.edge_run_mm)] {
        if length.is_some() {
            run_length(length, 0.0, label)?;
        }
    }
    let names = match &o.underlay {
        None if o.underlay_mm > 0.0 => vec!["centre_run".to_string(), "edge_run".to_string()],
        None => Vec::new(),
        Some(names) => names.clone(),
    };
    let mut seen = HashSet::new();
    for name in &names {
        if !seen.insert(name.clone()) {
            return Err(format!(
                "Object {} lists underlay {name} more than once. Keep each underlay once and compile again.",
                o.id
            ));
        }
        if name == "zigzag" || name == "lattice" {
            return Err(format!(
                "Object {} requests underlay {name}, which is planned and unavailable. Remove it and compile again.",
                o.id
            ));
        }
        if name != "centre_run" && name != "edge_run" {
            return Err(format!(
                "Object {} requests underlay {name}, which this satin generator does not sew. Use centre_run, edge_run, or both, then compile again.",
                o.id
            ));
        }
    }
    let centre = if names.iter().any(|name| name == "centre_run") {
        Some(run_length(o.centre_run_mm, o.underlay_mm, "Centre-run")?)
    } else {
        None
    };
    let edge = if names.iter().any(|name| name == "edge_run") {
        Some(run_length(o.edge_run_mm, o.underlay_mm, "Edge-run")?)
    } else {
        None
    };
    Ok((centre, edge))
}

fn check_design(d: &Design) -> Result<(), String> {
    if d.schema_version != 1 {
        return Err("Unsupported native design schema; expected version 1.".into());
    }
    if d.name.is_empty() || d.name.len() > 16 || !d.name.bytes().all(|b| (32..=126).contains(&b)) {
        return Err(
            "For this milestone, use a printable ASCII design name of 1–16 characters.".into(),
        );
    }
    if d.revision > 9_007_199_254_740_991 {
        return Err("Revision exceeds the safe JSON integer range.".into());
    }
    if d.objects.is_empty() || d.objects.len() > 16 {
        return Err("A design needs 1–16 objects.".into());
    }
    let mut ids = HashSet::new();
    let mut threads = HashSet::new();
    for o in &d.objects {
        if o.id == 0 || o.id > 9_007_199_254_740_991 || !ids.insert(o.id) {
            return Err("Object IDs must be unique positive safe integers.".into());
        }
        if o.label.is_empty() || o.label.len() > 80 {
            return Err("Object labels must contain 1–80 UTF-8 bytes.".into());
        }
        if o.kind == "text" {
            let Some(text) = o.text.as_ref() else {
                return Err(format!(
                    "Object {} is text and has no lettering intent. Enter the text and pin the font. An outline was not inferred.",
                    o.id
                ));
            };
            if !o.points.is_empty() || !o.widths_mm.is_empty() || !o.holes.is_empty() {
                return Err(format!(
                    "Object {} is text and carries geometry. Lettering stores the entered string, not an outline. Remove the points, widths, and holes, then compile again.",
                    o.id
                ));
            }
            if o.angle_degrees.abs() > 1e-9 {
                return Err(if text.layout.is_some() {
                    format!(
                        "Object {} sets a text angle. Horizontal lines are not rotated. Set the angle to 0 and compile again.",
                        o.id
                    )
                } else {
                    format!(
                        "Object {} sets a text angle. This lettering step sews one horizontal line. Set the angle to 0 and compile again.",
                        o.id
                    )
                });
            }
            if let Some(layout) = text.layout.as_ref() {
                embroidery_core::lettering_layout_error(o.id, layout)?;
            }
            if !text.cap_height_mm.is_finite()
                || text.cap_height_mm <= 0.0
                || text.cap_height_mm > 200.0
            {
                return Err(
                    "Cap height must be greater than 0 mm and at most 200 mm. That is a software limit, not a measured sewing range."
                        .into(),
                );
            }
            in_range(text.origin_x_mm, -60.0, 60.0, "Text origin X")?;
            in_range(text.origin_y_mm, -60.0, 60.0, "Text origin Y")?;
            in_range(text.letter_spacing_mm, 0.0, 1000.0, "letter_spacing_mm")?;
            in_range(text.word_spacing_mm, 0.0, 1000.0, "word_spacing_mm")?;
        } else if o.text.is_some() {
            return Err(format!(
                "Object {} carries text. Text belongs on a text object. Remove it or change the object to text. The text was not dropped.",
                o.id
            ));
        } else if !["running", "satin", "fill"].contains(&o.kind.as_str()) {
            return Err(format!("Unsupported stitch generator: {}", o.kind));
        }
        if o.kind != "text" {
            if o.points.len() < 2 || o.points.len() > 256 {
                return Err("Objects need 2–256 geometry points.".into());
            }
            for p in &o.points {
                in_range(p.x, -60.0, 60.0, "X")?;
                in_range(p.y, -60.0, 60.0, "Y")?;
            }
        }
        in_range(o.spacing_mm, 0.2, 4.0, "spacing_mm")?;
        in_range(o.angle_degrees, -360.0, 360.0, "angle_degrees")?;
        in_range(o.max_stitch_mm, 0.5, 10.0, "max_stitch_mm")?;
        in_range(o.width_mm, 0.5, 10.0, "width_mm")?;
        if !o.widths_mm.is_empty() {
            if o.kind != "satin" {
                return Err(format!(
                    "Object {} carries a width list, which is used only by satin. Remove the list or change the object to satin.",
                    o.id
                ));
            }
            if o.widths_mm.len() != o.points.len() {
                return Err(format!(
                    "Object {} needs one column width for each centreline point. The constant width was not substituted.",
                    o.id
                ));
            }
            for (index, width) in o.widths_mm.iter().enumerate() {
                if !width.is_finite() || *width <= 0.0 || *width > 48.0 {
                    return Err(format!(
                        "Object {} width {} must be greater than 0 mm and at most 48 mm. The constant width was not substituted.",
                        o.id,
                        index + 1
                    ));
                }
            }
        }
        in_range(o.compensation_mm, 0.0, 0.5, "compensation_mm")?;
        in_range(o.underlay_mm, 0.0, 10.0, "underlay_mm")?;
        if o.underlay_mm > 0.0 && o.underlay_mm < 0.2 {
            return Err("Underlay is either disabled (0) or at least 0.2 mm.".into());
        }
        satin_passes(o)?;
        if o.kind == "fill" {
            check_ring(&o.points)?;
        }
        if !o.holes.is_empty() {
            if o.kind != "fill" {
                return Err(format!(
                    "Object {} is {} and has holes. Holes belong only on a fill. Remove them or change the object to fill.",
                    o.id, o.kind
                ));
            }
            if o.holes.len() > 8 {
                return Err(
                    "This increment allows at most 8 holes on one fill. Remove some holes and generate again.".into(),
                );
            }
            let mut total = o.points.len();
            for (index, hole) in o.holes.iter().enumerate() {
                if !(3..=64).contains(&hole.len()) {
                    return Err(format!(
                        "Hole {} needs 3–64 points. Redraw that ring and generate again.",
                        index + 1
                    ));
                }
                total += hole.len();
                for p in hole {
                    in_range(p.x, -60.0, 60.0, "Hole X")?;
                    in_range(p.y, -60.0, 60.0, "Hole Y")?;
                }
            }
            if total > 256 {
                return Err(
                    "Outline and holes together exceed 256 points. Simplify the region and generate again.".into(),
                );
            }
            validate_fill_region(&o.points, &o.holes).map_err(|error| error.to_string())?;
        }
        threads.insert(o.rgb);
    }
    if threads.len() > 6 {
        return Err(
            "This generic six-needle profile allows at most six distinct thread colours.".into(),
        );
    }
    Ok(())
}

/// Decode every exported record through the existing Rust codec. No JS codec.
/// Strictly admits our generated subset; this is NOT a universal DST importer.
pub fn trace_from_dst(bytes: &[u8]) -> Result<Trace, String> {
    if bytes.len() < 515
        || bytes.len() > 512 + MAX_RECORDS * 3
        || (bytes.len() - 512) % 3 != 0
        || !bytes.starts_with(b"LA:")
    {
        return Err(
            "Expected a bounded DST file with a 512-byte header and complete 3-byte records."
                .into(),
        );
    }
    let decoded_plan = read_dst_with(
        bytes,
        &ReadOptions {
            collapse_trim_signature: false,
        },
    )
    .map_err(|e| e.to_string())?;
    if decoded_plan.metadata.contains_key("missing_end")
        || decoded_plan.metadata.contains_key("dst_st_mismatch")
    {
        return Err("DST header record count or end marker is inconsistent.".into());
    }
    let mut events = Vec::new();
    let mut pen = [0i32, 0i32];
    let mut color_index = 0;
    let mut time = 0.0;
    let mut needle_placements = 0;
    let total = (bytes.len() - 512) / 3;
    if decoded_plan
        .metadata
        .get("ST")
        .and_then(|v| v.parse::<usize>().ok())
        != Some(total)
    {
        return Err("DST must declare the exact record count in its ST header.".into());
    }
    let mut ended = false;
    for (index, chunk) in bytes[512..].chunks_exact(3).enumerate() {
        let raw = [chunk[0], chunk[1], chunk[2]];
        let r = decode_record(raw);
        if encode_record(r.dx, r.dy, r.command)? != raw {
            return Err(format!("Noncanonical DST record {index}."));
        }
        if r.command == RecordCommand::SequinMode {
            return Err("Sequin mode is unsupported by this profile.".into());
        }
        let next = [pen[0] + r.dx, pen[1] + r.dy];
        if next.iter().any(|p| p.abs() > 600) {
            return Err("Decoded motion leaves the nominal 120 × 120 mm demo envelope.".into());
        }
        let (kind, duration) = match r.command {
            RecordCommand::Stitch => {
                needle_placements += 1;
                ("stitch", 100.0)
            }
            RecordCommand::Jump => {
                let distance = ((r.dx as f64).powi(2) + (r.dy as f64).powi(2)).sqrt() / 10.0;
                ("jump", (distance / 100.0 * 1000.0).max(20.0))
            }
            RecordCommand::ColorChange => {
                color_index += 1;
                ("color_change", 1000.0)
            }
            RecordCommand::End => {
                if index + 1 != total {
                    return Err("Records follow the end marker.".into());
                }
                ended = true;
                ("end", 1.0)
            }
            RecordCommand::SequinMode => unreachable!(),
        };
        events.push(Event {
            index,
            byte_offset: 512 + index * 3,
            raw,
            kind: kind.into(),
            from_units: pen,
            to_units: next,
            color_index,
            start_ms: time,
            end_ms: time + duration,
            object_id: None,
        });
        time += duration;
        pen = next;
    }
    if !ended {
        return Err("DST has no end marker.".into());
    }
    // Cross-check the public reader and raw record stream, including colour changes.
    if events.len() != decoded_plan.stitches.len() + 1 {
        return Err("Reader/raw record cardinality disagreement.".into());
    }
    for (event, stitch) in events.iter().zip(decoded_plan.stitches.iter()) {
        if (
            (stitch.to.x * 10.0).round() as i32,
            (stitch.to.y * 10.0).round() as i32,
        ) != (event.to_units[0], event.to_units[1])
        {
            return Err("Reader/raw record position disagreement.".into());
        }
    }
    Ok(Trace {
        units: "0.1 mm; X right; Y down",
        decoder: "embroidery-dst::decode_record + read_dst_with",
        trim_policy: "raw jumps; no trim inference",
        timing: "illustrative only: 600 spm, 100 mm/s jumps, 1 s colour change",
        record_count: events.len(),
        events,
        needle_placements,
        duration_ms: time,
    })
}

fn lettering_font_pin(request: &Request) -> serde_json::Value {
    request
        .design
        .objects
        .iter()
        .find_map(|object| object.text.as_ref())
        .map(|text| {
            serde_json::json!({
                "id": text.font_id,
                "version": text.font_version,
                "name": FONT_NAME,
                "sha256": text.font_sha256,
            })
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "id": FONT_ID,
                "version": FONT_VERSION,
                "name": FONT_NAME,
                "sha256": font_content_sha256(),
            })
        })
}

pub fn compile_design(request: Request) -> Result<serde_json::Value, String> {
    if request.profile_id != PROFILE_ID {
        return Err(
            "Unknown machine profile. This milestone supports only generic-flat-120-v1.".into(),
        );
    }
    check_design(&request.design)?;
    let mut plan = StitchPlan::new(&request.design.name);
    let mut palette: Vec<serde_json::Value> = Vec::new();
    let mut unique_colors: Vec<[u8; 3]> = Vec::new();
    let mut previous_color = None;
    let mut color_index = 0;
    let mut bytes = Vec::new();
    let mut spans = Vec::new();
    let mut previous_records = 0;
    for o in &request.design.objects {
        if previous_color != Some(o.rgb) {
            if previous_color.is_some() {
                color_index += 1;
                let p = plan
                    .stitches
                    .last()
                    .map(|s| s.to)
                    .unwrap_or(Point::new(0.0, 0.0));
                plan.stitches.push(Stitch {
                    kind: StitchKind::ColorChange,
                    from: p,
                    to: p,
                    color_index,
                    object_id: o.id,
                });
            }
            let needle = if let Some(n) = unique_colors.iter().position(|c| *c == o.rgb) {
                n + 1
            } else {
                unique_colors.push(o.rgb);
                unique_colors.len()
            };
            palette.push(serde_json::json!({"rgb":o.rgb,"needle":needle}));
            plan.colors.push(ColorStop {
                color_index,
                thread: Some(ThreadSpec {
                    brand: "Generic demo".into(),
                    code: needle.to_string(),
                    name: o.label.clone(),
                    rgb: o.rgb,
                }),
            });
            previous_color = Some(o.rgb);
        }
        match o.kind.as_str() {
            "running" => generate_running(
                &mut plan,
                o.id,
                color_index,
                &o.points,
                &RunningConfig {
                    max_stitch_length_mm: o.max_stitch_mm,
                },
            ),
            "satin" => {
                let (centre_run_mm, edge_run_mm) = satin_passes(o)?;
                let config = SatinConfig {
                    max_column_width_mm: o.width_mm,
                    density_mm: o.spacing_mm,
                    pull_compensation_mm: o.compensation_mm,
                    centre_run_mm,
                    edge_run_mm,
                };
                if o.widths_mm.is_empty() {
                    generate_satin(&mut plan, o.id, color_index, &o.points, &config)
                } else {
                    generate_satin_variable(
                        &mut plan,
                        o.id,
                        color_index,
                        &o.points,
                        &o.widths_mm,
                        &config,
                    )
                }
            }
            "fill" => {
                let config = FillConfig {
                    angle_degrees: o.angle_degrees,
                    row_spacing_mm: o.spacing_mm,
                    max_stitch_length_mm: o.max_stitch_mm,
                    underlay_spacing_mm: o.underlay_mm,
                };
                if o.holes.is_empty() {
                    generate_fill(&mut plan, o.id, color_index, &o.points, &config)
                } else {
                    generate_fill_with_holes(
                        &mut plan,
                        o.id,
                        color_index,
                        &o.points,
                        &o.holes,
                        &config,
                    )
                }
            }
            "text" => {
                let text = o.text.as_ref().expect("text object checked");
                let (centre_run_mm, edge_run_mm) = satin_passes(o)?;
                generate_text(
                    &mut plan,
                    &TextRequest {
                        object_id: o.id,
                        content: &text.content,
                        font_id: &text.font_id,
                        font_version: &text.font_version,
                        font_sha256: &text.font_sha256,
                        origin_x_mm: text.origin_x_mm,
                        origin_y_mm: text.origin_y_mm,
                        cap_height_mm: text.cap_height_mm,
                        letter_spacing_mm: text.letter_spacing_mm,
                        word_spacing_mm: text.word_spacing_mm,
                        density_mm: o.spacing_mm,
                        column_width_mm: o.width_mm,
                        pull_compensation_mm: o.compensation_mm,
                        max_stitch_mm: o.max_stitch_mm,
                        centre_run_mm,
                        edge_run_mm,
                        color_index,
                        min_x: -60.0,
                        min_y: -60.0,
                        max_x: 60.0,
                        max_y: 60.0,
                        layout: text.layout.as_ref(),
                    },
                )
            }
            _ => unreachable!(),
        }
        .map_err(|e| format!("Object {}: {e}", o.id))?;
        if plan.stitches.len() > MAX_RECORDS {
            return Err("Generated plan exceeds this demo's 60,000-record budget.".into());
        }
        for s in &plan.stitches {
            for p in [s.from, s.to] {
                in_range(p.x, -60.0, 60.0, "Generated X")?;
                in_range(p.y, -60.0, 60.0, "Generated Y")?;
            }
        }
        let prefix = compile_to_vec(&plan, 12.1, 120.0).map_err(|e| e.to_string())?;
        let count = (prefix.len() - 512) / 3 - 1;
        if !bytes.is_empty()
            && prefix[512..512 + previous_records * 3] != bytes[512..bytes.len() - 3]
        {
            return Err("Compiler prefix is unstable; object trace mapping refused.".into());
        }
        spans.push((o.id, previous_records, count));
        previous_records = count;
        bytes = prefix;
    }
    let validation = validate(&plan, 12.1, 120.0);
    if !validation.ok() {
        return Err(validation.errors.join("\n"));
    }
    let mut trace = trace_from_dst(&bytes)?;
    for event in &mut trace.events {
        event.object_id = spans
            .iter()
            .find(|(_, lo, hi)| event.index >= *lo && event.index < *hi)
            .map(|s| s.0);
        if event.color_index >= palette.len() {
            return Err("Decoded colour sequence does not match the sidecar.".into());
        }
    }
    Ok(serde_json::json!({
        "protocol":"twinlab-m1/1",
        "design":request.design,
        "profile":{"id":PROFILE_ID,"name":"Generic 6-needle · flat frame","units":"mm","width_mm":120,"height_mm":120,"calibrated":false,"hardware_connected":false},
        "kernel":{"version":env!("CARGO_PKG_VERSION"),"base_commit":BASE_COMMIT,"generation":"embroidery-core","export":"embroidery-dst","mode":"native-rust"},
        "generated_stitch_count":plan.sewing_stitch_count(),
        "generation":{
            "phase_summary":plan.metadata.get("satin_phase_summary").cloned().unwrap_or_default(),
            "movement_summary":plan.metadata.get("satin_movement_summary").cloned().unwrap_or_default(),
            "top_sequence":plan.metadata.get("satin_top_sequence").cloned().unwrap_or_default(),
            "lettering_text":plan.metadata.get("lettering_text").cloned().unwrap_or_default(),
            "lettering_trace":plan.metadata.get("lettering_trace").cloned().unwrap_or_default(),
            "lettering_summary":plan.metadata.get("lettering_summary").cloned().unwrap_or_default(),
            "lettering_font": lettering_font_pin(&request)
        },
        "warnings":validation.warnings,
        "palette":palette,
        "trace":trace,
        "dst_bytes":bytes
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use embroidery_core::{font_v2_content_sha256, FONT_VERSION_2};
    fn object(kind: &str) -> DesignObject {
        DesignObject {
            id: 1,
            label: "Test".into(),
            kind: kind.into(),
            points: vec![
                Point::new(-20.0, -10.0),
                Point::new(20.0, -10.0),
                Point::new(20.0, 10.0),
                Point::new(-20.0, 10.0),
            ],
            holes: Vec::new(),
            rgb: [30, 90, 200],
            spacing_mm: 0.5,
            angle_degrees: 30.0,
            max_stitch_mm: 3.0,
            width_mm: 4.0,
            widths_mm: Vec::new(),
            compensation_mm: 0.1,
            underlay_mm: 0.0,
            underlay: None,
            centre_run_mm: None,
            edge_run_mm: None,
            text: None,
        }
    }
    fn req(kind: &str) -> Request {
        Request {
            profile_id: PROFILE_ID.into(),
            design: Design {
                schema_version: 1,
                name: "Test".into(),
                revision: 1,
                objects: vec![object(kind)],
            },
        }
    }
    fn bytes(v: &serde_json::Value) -> Vec<u8> {
        serde_json::from_value(v["dst_bytes"].clone()).unwrap()
    }
    #[test]
    fn all_three_generators_roundtrip_exact_export() {
        for kind in ["running", "satin", "fill"] {
            let v = compile_design(req(kind)).unwrap();
            let b = bytes(&v);
            let t = trace_from_dst(&b).unwrap();
            assert!(t.needle_placements > 0);
            assert_eq!(t.events.len() * 3 + 512, b.len());
            for e in t.events {
                assert_eq!(&b[e.byte_offset..e.byte_offset + 3], &e.raw);
            }
        }
    }
    #[test]
    fn unchanged_design_is_byte_deterministic() {
        assert_eq!(
            bytes(&compile_design(req("fill")).unwrap()),
            bytes(&compile_design(req("fill")).unwrap())
        );
    }
    #[test]
    fn spacing_changes_the_export() {
        let a = req("fill");
        let mut b = a.clone();
        b.design.objects[0].spacing_mm = 1.2;
        assert_ne!(
            bytes(&compile_design(a).unwrap()),
            bytes(&compile_design(b).unwrap())
        );
    }
    #[test]
    fn colour_stops_and_object_spans_are_preserved() {
        let mut r = req("fill");
        let mut o = object("running");
        o.id = 2;
        o.rgb = [200, 40, 60];
        r.design.objects.push(o);
        let v = compile_design(r).unwrap();
        assert_eq!(v["palette"].as_array().unwrap().len(), 2);
        assert!(v["trace"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "color_change" && e["object_id"] == 2));
    }
    #[test]
    fn unknown_profile_is_rejected() {
        let mut r = req("fill");
        r.profile_id = "unverified-brand".into();
        assert!(compile_design(r).is_err());
    }
    #[test]
    fn unsupported_generator_is_rejected() {
        assert!(compile_design(req("chenille")).is_err());
    }
    #[test]
    fn out_of_hoop_geometry_is_rejected() {
        let mut r = req("fill");
        r.design.objects[0].points[0].x = 61.0;
        assert!(compile_design(r).is_err());
    }
    #[test]
    fn generated_satin_outside_hoop_is_rejected() {
        let mut r = req("satin");
        r.design.objects[0].points = vec![Point::new(-10.0, 59.0), Point::new(10.0, 59.0)];
        assert!(compile_design(r).is_err());
    }
    #[test]
    fn duplicate_object_ids_are_rejected() {
        let mut r = req("fill");
        r.design.objects.push(object("running"));
        assert!(compile_design(r).is_err());
    }
    #[test]
    fn self_intersecting_fill_is_rejected() {
        let mut r = req("fill");
        r.design.objects[0].points.swap(1, 2);
        let error = compile_design(r).unwrap_err();
        assert_eq!(
            error,
            "Fill self-intersects. Repair its outline before generation."
        );
    }
    #[test]
    fn a_design_without_holes_still_compiles_and_a_hole_is_deterministic() {
        let missing = r#"{"schema_version":1,"name":"Plain","revision":1,"objects":[{"id":1,"label":"Field","kind":"fill","points":[{"x":-15.0,"y":-15.0},{"x":15.0,"y":-15.0},{"x":15.0,"y":15.0},{"x":-15.0,"y":15.0}],"rgb":[30,90,200],"spacing_mm":2.0,"angle_degrees":0.0,"max_stitch_mm":4.0,"width_mm":3.0,"compensation_mm":0.0,"underlay_mm":0.0}]}"#;
        let parsed: Request = serde_json::from_str(&format!(
            r#"{{"profile_id":"{PROFILE_ID}","design":{missing}}}"#
        ))
        .unwrap();
        assert!(parsed.design.objects[0].holes.is_empty());
        let plain = compile_design(parsed).unwrap();
        assert!(!plain.to_string().contains("\"holes\""));

        let mut holed = req("fill");
        holed.design.objects[0].points = vec![
            Point::new(-15.0, -15.0),
            Point::new(15.0, -15.0),
            Point::new(15.0, 15.0),
            Point::new(-15.0, 15.0),
        ];
        holed.design.objects[0].spacing_mm = 2.0;
        holed.design.objects[0].angle_degrees = 0.0;
        holed.design.objects[0].underlay_mm = 2.0;
        holed.design.objects[0].holes = vec![vec![
            Point::new(-4.0, -4.0),
            Point::new(4.0, -4.0),
            Point::new(4.0, 4.0),
            Point::new(-4.0, 4.0),
        ]];
        let first = bytes(&compile_design(holed.clone()).unwrap());
        let second = bytes(&compile_design(holed.clone()).unwrap());
        assert_eq!(first, second);
        assert_ne!(first, bytes(&plain));

        holed.design.objects[0].kind = "running".into();
        let error = compile_design(holed).unwrap_err();
        assert!(error.contains("Holes belong only on a fill"));
    }
    #[test]
    fn empty_and_excessive_documents_are_rejected() {
        let mut r = req("fill");
        r.design.objects.clear();
        assert!(compile_design(r).is_err());
        let mut r = req("fill");
        r.design.objects = vec![object("fill"); 17];
        assert!(compile_design(r).is_err());
    }
    #[test]
    fn raw_stitch_injection_is_rejected() {
        assert!(serde_json::from_str::<Request>(r#"{"profile_id":"generic-flat-120-v1","design":{"schema_version":1,"name":"x","revision":1,"objects":[],"stitches":[]}}"#).is_err());
    }
    #[test]
    fn density_and_nonfinite_values_are_rejected() {
        let mut r = req("fill");
        r.design.objects[0].spacing_mm = 0.0001;
        assert!(compile_design(r).is_err());
        let mut r = req("fill");
        r.design.objects[0].angle_degrees = f64::NAN;
        assert!(compile_design(r).is_err());
    }
    #[test]
    fn truncated_or_extra_records_are_rejected() {
        let b = bytes(&compile_design(req("running")).unwrap());
        assert!(trace_from_dst(&b[..b.len() - 1]).is_err());
        let mut c = b.clone();
        c.extend_from_slice(&[0, 0, 3]);
        assert!(trace_from_dst(&c).is_err());
    }
    #[test]
    fn signed_axes_survive_the_codec() {
        let t = trace_from_dst(&bytes(&compile_design(req("running")).unwrap())).unwrap();
        assert!(t.events.iter().any(|e| e.to_units[0] < 0));
        assert!(t.events.iter().any(|e| e.to_units[1] < 0));
    }
    #[test]
    fn one_hundred_parameterised_export_cases() {
        for i in 0..100 {
            let mut r = req(if i % 2 == 0 { "fill" } else { "running" });
            r.design.objects[0].angle_degrees = (i as f64) * 3.0;
            r.design.objects[0].spacing_mm = 0.4 + (i % 8) as f64 * 0.1;
            let v = compile_design(r).unwrap();
            let b = bytes(&v);
            let t = trace_from_dst(&b).unwrap();
            assert_eq!(t.record_count * 3 + 512, b.len());
        }
    }
    #[test]
    fn trim_signature_remains_three_jump_records() {
        let mut p = StitchPlan::new("Trim");
        generate_running(
            &mut p,
            1,
            0,
            &[Point::new(0.0, 0.0), Point::new(2.0, 0.0)],
            &RunningConfig::default(),
        )
        .unwrap();
        p.stitches.push(Stitch {
            kind: StitchKind::Trim,
            from: Point::new(2.0, 0.0),
            to: Point::new(2.0, 0.0),
            color_index: 0,
            object_id: 1,
        });
        let t = trace_from_dst(&compile_to_vec(&p, 12.1, 120.0).unwrap()).unwrap();
        assert_eq!(t.events.iter().filter(|e| e.kind == "jump").count(), 3);
        assert!(!t.events.iter().any(|e| e.kind == "trim"));
    }
    #[test]
    fn width_lists_are_kept_and_constant_satin_bytes_stay_fixed() {
        let constant = bytes(&compile_design(req("satin")).unwrap());
        let digest = embroidery_core::sha256_hex(&constant);
        assert_eq!(
            digest, "1267d41f8fdb70b6878222e5f33cda61af04ddd3022ce36e3d5252eb08e46398",
            "constant satin bytes changed"
        );
        let mut listed = req("satin");
        listed.design.objects[0].points = vec![Point::new(-20.0, 0.0), Point::new(20.0, 0.0)];
        listed.design.objects[0].widths_mm = vec![2.0, 6.0];
        listed.design.objects[0].compensation_mm = 0.0;
        listed.design.objects[0].spacing_mm = 2.0;
        listed.design.objects[0].underlay_mm = 0.0;
        let compiled = compile_design(listed.clone()).unwrap();
        let varied = bytes(&compiled);
        assert_ne!(varied, constant);
        assert_eq!(
            compiled["design"]["objects"][0]["widths_mm"],
            serde_json::json!([2.0, 6.0])
        );
        let stored = serde_json::to_string(&listed.design).unwrap();
        let reopened: Design = serde_json::from_str(&stored).unwrap();
        assert_eq!(reopened.objects[0].widths_mm, vec![2.0, 6.0]);
        let mut mismatch = listed.clone();
        mismatch.design.objects[0].widths_mm = vec![2.0];
        let error = compile_design(mismatch).unwrap_err();
        assert!(error.contains("was not substituted"), "{error}");
        let native = r#"{"profile_id":"generic-flat-120-v1","design":{"format":"embroidery-project","format_version":"1","name":"Native","revision":1,"hoop_width_mm":80.0,"objects":[{"object_id":1,"label":"Column","kind":"satin","points":[{"x":10.0,"y":20.0},{"x":40.0,"y":20.0}],"column_widths_mm":[2.0,8.0]}]}}"#;
        assert!(serde_json::from_str::<Request>(native).is_err());
        assert_eq!(
            embroidery_core::sha256_hex(&varied),
            "cd82ac5eee2d07e84e771a13cf9567b865ece1e15b4d8174e8d3d7b6e47c0cbc",
            "variable satin bytes changed"
        );
        let mut combined = listed.clone();
        combined.design.objects[0].underlay_mm = 2.5;
        let combined_bytes = bytes(&compile_design(combined).unwrap());
        assert_eq!(
            embroidery_core::sha256_hex(&combined_bytes),
            "9de0dbdb2cb803c585be157e1b85e9df4f28216e97e045278493fe59761d4f71",
            "combined variable underlay bytes changed"
        );
        let stored = serde_json::to_string(&req("satin").design).unwrap();
        assert!(!stored.contains("centre_run_mm"));
        assert!(!stored.contains("edge_run_mm"));
        assert!(!stored.contains("\"underlay\""));
    }

    #[test]
    fn satin_underlay_passes_are_independent_and_keep_the_top_sequence() {
        let mut base = req("satin");
        base.design.objects[0].points = vec![Point::new(-20.0, 0.0), Point::new(20.0, 0.0)];
        base.design.objects[0].widths_mm = vec![2.0, 6.0];
        base.design.objects[0].compensation_mm = 0.0;
        base.design.objects[0].spacing_mm = 2.0;
        base.design.objects[0].underlay_mm = 2.5;
        let historical = compile_design(base.clone()).unwrap();
        let mut explicit = base.clone();
        explicit.design.objects[0].underlay_mm = 0.0;
        explicit.design.objects[0].underlay = Some(vec!["centre_run".into(), "edge_run".into()]);
        explicit.design.objects[0].centre_run_mm = Some(2.5);
        explicit.design.objects[0].edge_run_mm = Some(2.5);
        let listed = compile_design(explicit.clone()).unwrap();
        assert_eq!(bytes(&historical), bytes(&listed));
        assert_eq!(
            historical["generation"]["top_sequence"],
            listed["generation"]["top_sequence"]
        );

        let mut centre = explicit.clone();
        centre.design.objects[0].underlay = Some(vec!["centre_run".into()]);
        centre.design.objects[0].edge_run_mm = None;
        let centre_only = compile_design(centre.clone()).unwrap();
        assert_ne!(bytes(&centre_only), bytes(&listed));
        assert_eq!(
            centre_only["generation"]["top_sequence"],
            listed["generation"]["top_sequence"]
        );
        let phases = centre_only["generation"]["phase_summary"].as_str().unwrap();
        assert!(phases.contains("Centre-run underlay"));
        assert!(phases.contains("Edge-run underlay 0"));
        assert!(phases.contains("Top satin"));
        assert_eq!(
            bytes(&compile_design(centre.clone()).unwrap()),
            bytes(&centre_only)
        );

        let mut edge = explicit.clone();
        edge.design.objects[0].underlay = Some(vec!["edge_run".into()]);
        edge.design.objects[0].centre_run_mm = None;
        let edge_only = compile_design(edge).unwrap();
        assert_ne!(bytes(&edge_only), bytes(&centre_only));
        assert_eq!(
            edge_only["generation"]["top_sequence"],
            listed["generation"]["top_sequence"]
        );

        let mut zigzag = explicit.clone();
        zigzag.design.objects[0].underlay = Some(vec!["zigzag".into()]);
        let error = compile_design(zigzag).unwrap_err();
        assert!(error.contains("unavailable"), "{error}");

        let mut missing = centre.clone();
        missing.design.objects[0].centre_run_mm = None;
        missing.design.objects[0].underlay_mm = 0.0;
        let error = compile_design(missing).unwrap_err();
        assert!(error.contains("Centre-run"), "{error}");
    }

    #[test]
    fn lettering_compiles_without_changing_schema_or_a_rejected_name() {
        let hash = font_content_sha256();
        let mut design = req("running").design;
        design.objects.push(DesignObject {
            id: 2,
            label: "Lettering".into(),
            kind: "text".into(),
            points: Vec::new(),
            holes: Vec::new(),
            rgb: [20, 70, 140],
            spacing_mm: 0.6,
            angle_degrees: 0.0,
            max_stitch_mm: 3.0,
            width_mm: 1.6,
            widths_mm: Vec::new(),
            compensation_mm: 0.0,
            underlay_mm: 0.0,
            underlay: None,
            centre_run_mm: None,
            edge_run_mm: None,
            text: Some(LetteringText {
                content: "ALPINE 08".into(),
                font_id: FONT_ID.into(),
                font_version: FONT_VERSION.into(),
                font_sha256: hash.clone(),
                origin_x_mm: -52.0,
                origin_y_mm: -8.0,
                cap_height_mm: 10.0,
                letter_spacing_mm: 0.8,
                word_spacing_mm: 3.6,
                layout: None,
            }),
        });
        let request = Request {
            profile_id: PROFILE_ID.into(),
            design: design.clone(),
        };
        let value = compile_design(request).unwrap();
        assert_eq!(value["design"]["schema_version"], 1);
        assert!(value["generation"]["lettering_text"]
            .as_str()
            .unwrap()
            .contains("ALPINE 08"));
        assert!(value["generation"]["lettering_trace"]
            .as_str()
            .unwrap()
            .contains("U+0020 space"));
        assert_eq!(value["generation"]["lettering_font"]["sha256"], hash);
        let again = compile_design(Request {
            profile_id: PROFILE_ID.into(),
            design: design.clone(),
        })
        .unwrap();
        assert_eq!(bytes(&value), bytes(&again));

        let mut rejected = design.clone();
        rejected.objects[1].text.as_mut().unwrap().content = "O\u{2019}NEIL".into();
        let error = compile_design(Request {
            profile_id: PROFILE_ID.into(),
            design: rejected.clone(),
        })
        .unwrap_err();
        assert!(error.contains("U+2019"), "{error}");
        assert_eq!(
            rejected.objects[1].text.as_ref().unwrap().content,
            "O\u{2019}NEIL"
        );

        let mut version_one_cafe = design.clone();
        version_one_cafe.objects[1].text.as_mut().unwrap().content = "Café".into();
        let error = compile_design(Request {
            profile_id: PROFILE_ID.into(),
            design: version_one_cafe,
        })
        .unwrap_err();
        assert!(error.contains("U+0061"), "{error}");
        assert!(!error.contains("version 2"), "{error}");

        let v2_hash = font_v2_content_sha256();
        let mut version_two = design;
        let text = version_two.objects[1].text.as_mut().unwrap();
        text.content = "Café".into();
        text.font_version = FONT_VERSION_2.into();
        text.font_sha256 = v2_hash.clone();
        let sewn = compile_design(Request {
            profile_id: PROFILE_ID.into(),
            design: version_two,
        })
        .unwrap();
        assert_eq!(sewn["design"]["schema_version"], 1);
        assert_eq!(sewn["generation"]["lettering_font"]["version"], "2");
        assert_eq!(sewn["generation"]["lettering_font"]["sha256"], v2_hash);
        assert!(sewn["generation"]["lettering_text"]
            .as_str()
            .unwrap()
            .contains("Café"));
        assert_eq!(sewn["profile"]["hardware_connected"].as_bool(), Some(false));
    }
}
