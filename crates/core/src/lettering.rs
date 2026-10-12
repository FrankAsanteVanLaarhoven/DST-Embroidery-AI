//! EmbroideryOS Block 01, one original single-line satin font.
//!
//! Glyph recipes are straight centreline components in a 100-unit cap box,
//! Y downward. They contain no stitch coordinates and no DST bytes. Layout
//! scales those recipes to the requested cap height and sews each component
//! with [`crate::generate_satin`]. A failure restores the plan that was
//! passed in.

use serde::{Deserialize, Serialize};

use crate::error::PlanError;
use crate::generators::{generate_satin, SatinConfig};
use crate::model::{Point, StitchKind, StitchPlan};
use crate::sha256::sha256_hex;

/// Layout policy for explicit horizontal lines. This is not a font version
/// and it is not the native document version.
pub const LINE_LAYOUT_POLICY: &str = "embroideryos-line-layout/1";
pub const MAX_LINES: usize = 4;
/// Recipe baseline. Capitals sit on this font-unit row. Y grows downward.
pub(crate) const FONT_BASELINE_UNITS: f64 = 92.0;

/// Explicit line placement. Omitted text keeps the historical single-line path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineLayout {
    pub policy: String,
    pub alignment: String,
    pub block_width_mm: f64,
    pub first_baseline_mm: f64,
    pub baseline_spacing_mm: f64,
}

pub const FONT_ID: &str = "embroideryos-block-01";
pub const FONT_VERSION: &str = "1";
pub const FONT_NAME: &str = "EmbroideryOS Block 01";
/// Cap height in the recipe is 100 units. This is not a sewing limit.
pub(crate) const CAP_UNITS: f64 = 100.0;
pub(crate) const MAX_TEXT_CHARS: usize = 32;
pub(crate) const MAX_COMPONENTS: usize = 400;
pub(crate) const MAX_CAP_MM: f64 = 200.0;
pub(crate) const MAX_SPACING_MM: f64 = 1000.0;

/// A–Z, 0–9, space, hyphen, and straight apostrophe. 39 entries.
pub const REPERTOIRE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -'";

pub(crate) type Stroke = (i16, i16, i16, i16);

pub(crate) struct Glyph {
    pub advance: i16,
    pub strokes: &'static [Stroke],
}

#[derive(Clone, Copy)]
pub struct TextRequest<'a> {
    pub object_id: u64,
    pub content: &'a str,
    pub font_id: &'a str,
    pub font_version: &'a str,
    pub font_sha256: &'a str,
    pub origin_x_mm: f64,
    pub origin_y_mm: f64,
    pub cap_height_mm: f64,
    pub letter_spacing_mm: f64,
    pub word_spacing_mm: f64,
    pub density_mm: f64,
    pub column_width_mm: f64,
    pub pull_compensation_mm: f64,
    pub max_stitch_mm: f64,
    pub centre_run_mm: Option<f64>,
    pub edge_run_mm: Option<f64>,
    pub color_index: usize,
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    /// Absent keeps the historical single-line placement.
    pub layout: Option<&'a LineLayout>,
}

pub fn font_recipe() -> String {
    let mut recipe =
        String::from("EmbroideryOS Block 01\nembroideryos-block-01\n1\ncap 100\ny-down\n");
    for character in REPERTOIRE.chars() {
        let glyph = glyph(character).expect("repertoire entry");
        recipe.push_str(&format!(
            "U+{:04X} advance {}\n",
            character as u32, glyph.advance
        ));
        for stroke in glyph.strokes {
            recipe.push_str(&format!(
                "{} {} {} {}\n",
                stroke.0, stroke.1, stroke.2, stroke.3
            ));
        }
    }
    recipe
}

pub fn font_content_sha256() -> String {
    sha256_hex(font_recipe().as_bytes())
}

pub fn generate_text(plan: &mut StitchPlan, request: &TextRequest<'_>) -> Result<(), PlanError> {
    let marker = plan.stitches.len();
    let metadata = plan.metadata.clone();
    match sew_text(plan, request) {
        Ok(()) => Ok(()),
        Err(error) => {
            plan.stitches.truncate(marker);
            plan.metadata = metadata;
            Err(error)
        }
    }
}

pub(crate) fn sew_text(plan: &mut StitchPlan, request: &TextRequest<'_>) -> Result<(), PlanError> {
    if request.layout.is_some() {
        return crate::lettering_layout::sew(plan, request);
    }
    if request.font_version == crate::lettering_v2::FONT_VERSION_2 {
        return crate::lettering_v2::sew(plan, request);
    }
    pin_font(request)?;
    check_ranges(request)?;
    let characters: Vec<char> = request.content.chars().collect();
    if characters.len() > MAX_TEXT_CHARS {
        return Err(stopped(format!(
            "Object {} has {} characters. This lettering step allows at most {MAX_TEXT_CHARS}. Shorten the text and generate again.",
            request.object_id,
            characters.len()
        )));
    }
    if characters.is_empty() {
        return Err(stopped(format!(
            "Object {} has no sewing characters. Enter text that includes a letter or digit, then generate again.",
            request.object_id
        )));
    }
    for (index, character) in characters.iter().copied().enumerate() {
        if glyph(character).is_none() {
            return Err(stopped(format!(
                "Object {} contains {character} (U+{:04X}) at position {}. EmbroideryOS Block 01 does not sew that character. The text was left unchanged. Use A–Z, 0–9, space, hyphen, or a straight apostrophe, then generate again.",
                request.object_id,
                character as u32,
                index + 1
            )));
        }
    }
    if characters.iter().all(|character| *character == ' ') {
        return Err(stopped(format!(
            "Object {} has no sewing characters. A space only advances the line. Enter text that includes a letter or digit, then generate again.",
            request.object_id
        )));
    }

    let scale = request.cap_height_mm / CAP_UNITS;
    let margin = request.column_width_mm / 2.0 + request.pull_compensation_mm;
    let mut component_count = 0usize;
    let mut pen = 0.0;
    for (index, character) in characters.iter().copied().enumerate() {
        let item = glyph(character).expect("checked");
        if character != ' ' {
            component_count += item.strokes.len();
        }
        if component_count > MAX_COMPONENTS {
            return Err(stopped(format!(
                "Object {} expands to more than {MAX_COMPONENTS} glyph components. Shorten the text and generate again.",
                request.object_id
            )));
        }
        if character != ' ' {
            for stroke in item.strokes {
                for (x, y) in [(stroke.0, stroke.1), (stroke.2, stroke.3)] {
                    let point = Point::new(
                        request.origin_x_mm + pen + f64::from(x) * scale,
                        request.origin_y_mm + f64::from(y) * scale,
                    );
                    if !inside_margin(point, margin, request) {
                        return Err(stopped(format!(
                            "Object {} places {character} outside the hoop at ({:.2}, {:.2}) mm. Move the text or reduce the cap height, spacing, column width, or pull compensation, then generate again.",
                            request.object_id, point.x, point.y
                        )));
                    }
                }
            }
        }
        pen += advance_mm(character, item.advance, scale, request);
        if letter_gap(index, &characters) {
            pen += request.letter_spacing_mm;
        }
    }

    let config = SatinConfig {
        max_column_width_mm: request.column_width_mm,
        density_mm: request.density_mm,
        pull_compensation_mm: request.pull_compensation_mm,
        centre_run_mm: request.centre_run_mm,
        edge_run_mm: request.edge_run_mm,
    };
    let mut trace = String::new();
    pen = 0.0;
    let mut sewn_components = 0usize;
    let stitch_start = plan.stitches.len();
    for (index, character) in characters.iter().copied().enumerate() {
        let item = glyph(character).expect("checked");
        if character == ' ' {
            push_trace(
                &mut trace,
                &format!(
                    "object {} glyph {index} U+0020 space advance {:.3}",
                    request.object_id, request.word_spacing_mm
                ),
            );
        } else {
            for (component, stroke) in item.strokes.iter().enumerate() {
                let start = Point::new(
                    request.origin_x_mm + pen + f64::from(stroke.0) * scale,
                    request.origin_y_mm + f64::from(stroke.1) * scale,
                );
                let end = Point::new(
                    request.origin_x_mm + pen + f64::from(stroke.2) * scale,
                    request.origin_y_mm + f64::from(stroke.3) * scale,
                );
                let from = plan.stitches.len();
                generate_satin(
                    plan,
                    request.object_id,
                    request.color_index,
                    &[start, end],
                    &config,
                )?;
                let to = plan.stitches.len();
                sewn_components += 1;
                push_trace(
                    &mut trace,
                    &format!(
                        "object {} glyph {index} U+{:04X} {} component {component} stitches {from}-{to}",
                        request.object_id,
                        character as u32,
                        label_for(character)
                    ),
                );
            }
        }
        pen += advance_mm(character, item.advance, scale, request);
        if letter_gap(index, &characters) {
            pen += request.letter_spacing_mm;
        }
    }
    if sewn_components == 0 {
        return Err(stopped(format!(
            "Object {} has no sewing characters. Enter text that includes a letter or digit, then generate again.",
            request.object_id
        )));
    }
    for stitch in &plan.stitches[stitch_start..] {
        if stitch.kind == StitchKind::Trim {
            return Err(stopped(format!(
                "Object {} produced a trim. Lettering does not add tie-offs.",
                request.object_id
            )));
        }
        for point in [stitch.from, stitch.to] {
            if !inside_hoop(point, request) {
                return Err(stopped(format!(
                    "Object {} sews outside the hoop at ({:.2}, {:.2}) mm. Move the text or reduce the cap height, spacing, column width, or pull compensation, then generate again.",
                    request.object_id, point.x, point.y
                )));
            }
        }
    }
    append_meta(
        plan,
        "lettering_text",
        &format!("object {} {}", request.object_id, request.content),
    );
    append_meta(plan, "lettering_trace", &trace);
    append_meta(
        plan,
        "lettering_summary",
        &format!(
            "EmbroideryOS Block 01 version 1 object {}: {} glyphs, {sewn_components} components. Cap height {:.3} mm is a software size, not a measured sewing limit.",
            request.object_id,
            characters.len(),
            request.cap_height_mm
        ),
    );
    Ok(())
}

pub(crate) fn pin_font(request: &TextRequest<'_>) -> Result<(), PlanError> {
    let expected = font_content_sha256();
    if request.font_id == FONT_ID
        && request.font_version == FONT_VERSION
        && request.font_sha256 == expected
    {
        return Ok(());
    }
    Err(stopped(format!(
        "Object {} is pinned to font {} version {} {}. This engine sews {FONT_NAME} version {FONT_VERSION} {expected}. The saved font was not replaced.",
        request.object_id, request.font_id, request.font_version, request.font_sha256
    )))
}

pub(crate) fn check_ranges(request: &TextRequest<'_>) -> Result<(), PlanError> {
    if !request.cap_height_mm.is_finite()
        || request.cap_height_mm <= 0.0
        || request.cap_height_mm > MAX_CAP_MM
    {
        return Err(stopped(format!(
            "Object {} needs a cap height greater than 0 mm and at most {MAX_CAP_MM} mm. This is a software limit, not a measured sewing range. Enter a height such as 8, 12, or 20 mm and generate again.",
            request.object_id
        )));
    }
    for (name, value) in [
        ("letter spacing", request.letter_spacing_mm),
        ("word spacing", request.word_spacing_mm),
    ] {
        if !(0.0..=MAX_SPACING_MM).contains(&value) {
            return Err(stopped(format!(
                "Object {} needs a {name} from 0 mm to {MAX_SPACING_MM} mm. Long text is not shrunk to fit. Enter a spacing in that range and generate again.",
                request.object_id
            )));
        }
    }
    if !request.density_mm.is_finite() || request.density_mm <= 0.0 || request.density_mm > 20.0 {
        return Err(stopped(format!(
            "Object {} needs a satin station spacing greater than 0 mm and at most 20 mm. Enter that spacing and generate again.",
            request.object_id
        )));
    }
    if !request.column_width_mm.is_finite()
        || request.column_width_mm <= 0.0
        || request.column_width_mm > 48.0
    {
        return Err(stopped(format!(
            "Object {} needs a column width greater than 0 mm and at most 48 mm. Enter that width and generate again.",
            request.object_id
        )));
    }
    if !request.pull_compensation_mm.is_finite() || request.pull_compensation_mm < 0.0 {
        return Err(stopped(format!(
            "Object {} needs pull compensation of 0 mm or more. Enter that allowance and generate again.",
            request.object_id
        )));
    }
    if !request.max_stitch_mm.is_finite()
        || request.max_stitch_mm <= 0.0
        || request.max_stitch_mm > crate::DST_MAX_STITCH_MM
    {
        return Err(stopped(format!(
            "Object {} needs a maximum stitch length greater than 0 mm and at most {:.1} mm. Reduce that length and generate again.",
            request.object_id,
            crate::DST_MAX_STITCH_MM
        )));
    }
    Ok(())
}

pub(crate) fn advance_mm(
    character: char,
    advance: i16,
    scale: f64,
    request: &TextRequest<'_>,
) -> f64 {
    if character == ' ' {
        request.word_spacing_mm
    } else {
        f64::from(advance) * scale
    }
}

pub(crate) fn letter_gap(index: usize, characters: &[char]) -> bool {
    let Some(next) = characters.get(index + 1).copied() else {
        return false;
    };
    characters[index] != ' ' && next != ' '
}

pub(crate) fn inside_margin(point: Point, margin: f64, request: &TextRequest<'_>) -> bool {
    point.x >= request.min_x - 1e-6 + margin
        && point.y >= request.min_y - 1e-6 + margin
        && point.x <= request.max_x + 1e-6 - margin
        && point.y <= request.max_y + 1e-6 - margin
}

/// Stitches may land on the hoop edge. The outward tolerance matches the
/// project hoop check. It is not an inset, and it does not accept a point
/// the project would then keep as a draft.
pub(crate) fn inside_hoop(point: Point, request: &TextRequest<'_>) -> bool {
    point.x >= request.min_x - 1e-6
        && point.y >= request.min_y - 1e-6
        && point.x <= request.max_x + 1e-6
        && point.y <= request.max_y + 1e-6
}

fn label_for(character: char) -> &'static str {
    match character {
        '\'' => "apostrophe",
        '-' => "hyphen",
        _ => "glyph",
    }
}

pub(crate) fn stopped(message: String) -> PlanError {
    PlanError::new(format!("{message} The plan was not changed."))
}

pub(crate) fn push_trace(trace: &mut String, line: &str) {
    if !trace.is_empty() {
        trace.push('\n');
    }
    trace.push_str(line);
}

pub(crate) fn append_meta(plan: &mut StitchPlan, key: &str, line: &str) {
    let entry = plan.metadata.entry(key.to_string()).or_default();
    if !entry.is_empty() {
        entry.push('\n');
    }
    entry.push_str(line);
}

pub(crate) fn glyph(character: char) -> Option<Glyph> {
    let (advance, strokes): (i16, &'static [Stroke]) = match character {
        'A' => (64, &[(16, 92, 32, 8), (48, 92, 32, 8), (20, 58, 44, 58)]),
        'B' => (
            64,
            &[
                (14, 8, 14, 92),
                (14, 8, 48, 8),
                (48, 8, 48, 50),
                (14, 50, 48, 50),
                (48, 50, 48, 92),
                (14, 92, 48, 92),
            ],
        ),
        'C' => (64, &[(14, 16, 14, 84), (14, 8, 50, 8), (14, 92, 50, 92)]),
        'D' => (
            64,
            &[
                (14, 8, 14, 92),
                (14, 8, 44, 8),
                (50, 16, 50, 84),
                (14, 92, 44, 92),
            ],
        ),
        'E' => (
            60,
            &[
                (14, 8, 14, 92),
                (14, 8, 50, 8),
                (14, 50, 42, 50),
                (14, 92, 50, 92),
            ],
        ),
        'F' => (56, &[(14, 8, 14, 92), (14, 8, 50, 8), (14, 50, 42, 50)]),
        'G' => (
            64,
            &[
                (14, 16, 14, 84),
                (14, 8, 50, 8),
                (14, 92, 50, 92),
                (50, 50, 50, 92),
                (36, 50, 50, 50),
            ],
        ),
        'H' => (64, &[(14, 8, 14, 92), (50, 8, 50, 92), (14, 50, 50, 50)]),
        'I' => (40, &[(20, 8, 20, 92), (8, 8, 32, 8), (8, 92, 32, 92)]),
        'J' => (
            52,
            &[
                (40, 8, 40, 80),
                (14, 92, 40, 92),
                (14, 72, 14, 92),
                (28, 8, 50, 8),
            ],
        ),
        'K' => (64, &[(14, 8, 14, 92), (14, 50, 50, 8), (14, 50, 50, 92)]),
        'L' => (56, &[(14, 8, 14, 92), (14, 92, 50, 92)]),
        'M' => (
            76,
            &[
                (12, 92, 12, 8),
                (12, 8, 38, 52),
                (64, 8, 38, 52),
                (64, 8, 64, 92),
            ],
        ),
        'N' => (64, &[(14, 92, 14, 8), (14, 8, 50, 92), (50, 92, 50, 8)]),
        'O' => (
            64,
            &[
                (14, 16, 14, 84),
                (50, 16, 50, 84),
                (14, 8, 50, 8),
                (14, 92, 50, 92),
            ],
        ),
        'P' => (
            60,
            &[
                (14, 8, 14, 92),
                (14, 8, 48, 8),
                (48, 8, 48, 50),
                (14, 50, 48, 50),
            ],
        ),
        'Q' => (
            68,
            &[
                (14, 16, 14, 84),
                (50, 16, 50, 84),
                (14, 8, 50, 8),
                (14, 92, 50, 92),
                (36, 68, 60, 98),
            ],
        ),
        'R' => (
            64,
            &[
                (14, 8, 14, 92),
                (14, 8, 48, 8),
                (48, 8, 48, 50),
                (14, 50, 48, 50),
                (28, 50, 52, 92),
            ],
        ),
        'S' => (
            60,
            &[
                (14, 8, 50, 8),
                (14, 8, 14, 46),
                (14, 50, 50, 50),
                (50, 50, 50, 92),
                (14, 92, 50, 92),
            ],
        ),
        'T' => (60, &[(10, 8, 50, 8), (30, 8, 30, 92)]),
        'U' => (64, &[(14, 8, 14, 84), (50, 8, 50, 84), (14, 92, 50, 92)]),
        'V' => (64, &[(12, 8, 32, 92), (52, 8, 32, 92)]),
        'W' => (
            80,
            &[
                (8, 8, 22, 92),
                (36, 8, 22, 92),
                (36, 8, 50, 92),
                (64, 8, 50, 92),
            ],
        ),
        'X' => (64, &[(14, 8, 50, 92), (50, 8, 14, 92)]),
        'Y' => (64, &[(12, 8, 32, 48), (52, 8, 32, 48), (32, 48, 32, 92)]),
        'Z' => (60, &[(14, 8, 50, 8), (50, 8, 14, 92), (14, 92, 50, 92)]),
        '0' => (
            64,
            &[
                (14, 16, 14, 84),
                (50, 16, 50, 84),
                (14, 8, 50, 8),
                (14, 92, 50, 92),
                (42, 22, 22, 78),
            ],
        ),
        '1' => (40, &[(24, 8, 24, 92), (12, 22, 24, 8), (10, 92, 38, 92)]),
        '2' => (
            60,
            &[
                (14, 8, 48, 8),
                (48, 8, 48, 46),
                (48, 46, 14, 92),
                (14, 92, 50, 92),
            ],
        ),
        '3' => (
            60,
            &[
                (14, 8, 48, 8),
                (48, 8, 48, 92),
                (14, 50, 48, 50),
                (14, 92, 48, 92),
            ],
        ),
        '4' => (64, &[(14, 8, 14, 54), (14, 54, 52, 54), (40, 8, 40, 92)]),
        '5' => (
            60,
            &[
                (50, 8, 14, 8),
                (14, 8, 14, 48),
                (14, 48, 48, 48),
                (48, 48, 48, 92),
                (48, 92, 14, 92),
            ],
        ),
        '6' => (
            60,
            &[
                (14, 8, 14, 92),
                (14, 8, 40, 8),
                (14, 50, 48, 50),
                (48, 50, 48, 92),
                (14, 92, 48, 92),
            ],
        ),
        '7' => (60, &[(14, 8, 50, 8), (50, 8, 22, 92)]),
        '8' => (
            60,
            &[
                (14, 18, 14, 46),
                (46, 18, 46, 46),
                (14, 10, 46, 10),
                (14, 50, 46, 50),
                (14, 54, 14, 86),
                (46, 54, 46, 86),
                (14, 90, 46, 90),
            ],
        ),
        '9' => (
            60,
            &[
                (14, 8, 14, 50),
                (14, 8, 48, 8),
                (48, 8, 48, 92),
                (14, 50, 48, 50),
                (14, 92, 48, 92),
            ],
        ),
        ' ' => (0, &[]),
        '-' => (40, &[(8, 50, 32, 50)]),
        '\'' => (22, &[(10, 4, 10, 28)]),
        _ => return None,
    };
    Some(Glyph { advance, strokes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::{generate_running, RunningConfig};

    fn request<'a>(content: &'a str, cap: f64, font_sha256: &'a str) -> TextRequest<'a> {
        TextRequest {
            object_id: 3,
            content,
            font_id: FONT_ID,
            font_version: FONT_VERSION,
            font_sha256,
            origin_x_mm: 8.0,
            origin_y_mm: 8.0,
            cap_height_mm: cap,
            letter_spacing_mm: 1.0,
            word_spacing_mm: 4.0,
            density_mm: 1.5,
            column_width_mm: cap * 0.16,
            pull_compensation_mm: 0.0,
            max_stitch_mm: 3.0,
            centre_run_mm: None,
            edge_run_mm: None,
            color_index: 0,
            min_x: 0.0,
            min_y: 0.0,
            max_x: 220.0,
            max_y: 80.0,
            layout: None,
        }
    }

    const PINNED_FONT_SHA256: &str =
        "aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd";

    fn sew(content: &str, cap: f64) -> StitchPlan {
        let hash = font_content_sha256();
        let item = request(content, cap, &hash);
        let mut plan = StitchPlan::new("letter");
        generate_text(&mut plan, &item).unwrap_or_else(|error| panic!("{content}: {error}"));
        plan
    }

    #[test]
    fn repertoire_has_39_entries_and_a_stable_hash() {
        assert_eq!(REPERTOIRE.chars().count(), 39);
        let drawable = REPERTOIRE
            .chars()
            .filter(|character| !glyph(*character).unwrap().strokes.is_empty())
            .count();
        assert_eq!(drawable, 38);
        assert!(glyph(' ').unwrap().strokes.is_empty());
        let hash = font_content_sha256();
        assert_eq!(hash, font_content_sha256());
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash, PINNED_FONT_SHA256,
            "pin EmbroideryOS Block 01 content hash"
        );
    }

    fn occupied() -> StitchPlan {
        let mut plan = StitchPlan::new("occupied");
        generate_running(
            &mut plan,
            9,
            0,
            &[Point::new(12.0, 12.0), Point::new(40.0, 12.0)],
            &RunningConfig {
                max_stitch_length_mm: 3.0,
            },
        )
        .unwrap();
        plan
    }

    fn reject(item: &TextRequest<'_>, plan: &mut StitchPlan) -> String {
        generate_text(plan, item)
            .expect_err("lettering must reject this input")
            .message()
            .to_string()
    }

    fn assert_preserved(before: &StitchPlan, after: &StitchPlan) {
        assert_eq!(before.stitches, after.stitches);
        assert_eq!(before.metadata, after.metadata);
    }

    fn component_ranges(plan: &StitchPlan) -> Vec<(usize, usize)> {
        plan.metadata["lettering_trace"]
            .lines()
            .filter_map(|line| {
                let rest = line.split_once("stitches ")?.1;
                let (start, end) = rest.split_once('-')?;
                Some((start.parse().unwrap(), end.parse().unwrap()))
            })
            .collect()
    }

    #[test]
    fn checkpoint_glyphs_space_and_punctuation_keep_text_identity() {
        for content in ["A H I L O V 0 8", "ALPINE 08", "O'NEIL-08"] {
            let plan = sew(content, 12.0);
            let again = sew(content, 12.0);
            assert_eq!(plan, again, "{content} must be deterministic");
            assert!(plan.sewing_stitch_count() > 0);
            assert!(plan
                .stitches
                .iter()
                .all(|stitch| stitch.kind != StitchKind::Trim));
            let text = &plan.metadata["lettering_text"];
            assert_eq!(text, &format!("object 3 {content}"));
            let trace = &plan.metadata["lettering_trace"];
            assert!(trace.contains("object 3 glyph"));
            let ranges = component_ranges(&plan);
            assert!(!ranges.is_empty());
            assert_eq!(ranges[0].0, 0);
            for window in ranges.windows(2) {
                assert_eq!(window[0].1, window[1].0);
            }
            assert_eq!(ranges.last().unwrap().1, plan.stitches.len());
        }

        let spaced = sew("A H I L O V 0 8", 12.0);
        let trace = &spaced.metadata["lettering_trace"];
        assert_eq!(trace.matches("U+0020 space advance 4.000").count(), 7);
        for code in [
            "0041", "0048", "0049", "004C", "004F", "0056", "0030", "0038",
        ] {
            assert!(trace.contains(&format!("U+{code}")));
        }

        let punctuated = sew("O'NEIL-08", 12.0);
        let trace = &punctuated.metadata["lettering_trace"];
        assert!(trace.contains("U+0027 apostrophe"));
        assert!(trace.contains("U+002D hyphen"));
        assert!(trace.contains("U+004F"));
        assert!(trace.contains("U+0030"));
        assert!(trace.contains("U+0038"));
    }

    #[test]
    fn every_drawable_glyph_is_stable_at_three_software_heights() {
        for character in REPERTOIRE.chars().filter(|character| *character != ' ') {
            for cap in [8.0, 12.0, 20.0] {
                let content = character.to_string();
                let plan = sew(&content, cap);
                assert_eq!(plan, sew(&content, cap), "{character} at {cap}");
                assert!(plan.sewing_stitch_count() > 0, "{character} at {cap}");
                assert!(plan
                    .stitches
                    .iter()
                    .all(|stitch| stitch.kind != StitchKind::Trim));
                assert!(plan.metadata["lettering_summary"].contains("software size"));
                assert!(plan.metadata["lettering_trace"]
                    .contains(&format!("U+{:04X}", character as u32)));
            }
        }
        let small = sew("A", 8.0);
        let large = sew("A", 20.0);
        assert!(large.sewing_stitch_count() > small.sewing_stitch_count());
        assert_ne!(small.stitches, large.stitches);
    }

    #[test]
    fn space_advances_without_sewing_and_letters_do_not_bridge() {
        let single = sew("A", 12.0);
        let spaced = sew("A A", 12.0);
        assert_eq!(
            spaced.sewing_stitch_count(),
            single.sewing_stitch_count() * 2
        );
        assert!(spaced.metadata["lettering_trace"].contains("U+0020 space advance 4.000"));
        assert!(spaced
            .stitches
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .all(|stitch| stitch.from.distance(stitch.to) < 6.0));
        assert!(
            spaced
                .stitches
                .iter()
                .any(|stitch| stitch.kind == StitchKind::Jump
                    && stitch.from.distance(stitch.to) > 6.0)
        );

        let hash = font_content_sha256();
        let mut wide = request("II", 12.0, &hash);
        wide.letter_spacing_mm = 20.0;
        let mut plan = StitchPlan::new("gap");
        generate_text(&mut plan, &wide).unwrap();
        assert!(plan
            .stitches
            .iter()
            .all(|stitch| stitch.kind != StitchKind::Trim));
        assert!(plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .all(|stitch| stitch.from.distance(stitch.to) < 14.0));
        assert!(plan.stitches.iter().any(|stitch| {
            stitch.kind == StitchKind::Jump && stitch.from.distance(stitch.to) > 15.0
        }));
    }

    #[test]
    fn four_underlay_selections_keep_the_top_sequence() {
        for content in ["A", "HI", "O", "8", "ALPINE 08"] {
            let modes = [
                (None, None),
                (Some(2.5), None),
                (None, Some(2.5)),
                (Some(2.5), Some(2.5)),
            ];
            let mut sequences = Vec::new();
            let mut counts = Vec::new();
            for (centre, edge) in modes {
                let hash = font_content_sha256();
                let mut item = request(content, 12.0, &hash);
                item.centre_run_mm = centre;
                item.edge_run_mm = edge;
                let mut plan = StitchPlan::new("underlay");
                generate_text(&mut plan, &item).expect(content);
                sequences.push(plan.metadata["satin_top_sequence"].clone());
                counts.push(plan.stitches.len());
                let phases = &plan.metadata["satin_phase_summary"];
                assert!(phases.contains("Centre-run underlay"), "{content}");
                assert!(phases.contains("Edge-run underlay"), "{content}");
                assert!(phases.contains("Top satin"), "{content}");
                assert!(
                    phases.contains("Positioning and inter-phase connectors"),
                    "{content}"
                );
                assert!(plan
                    .stitches
                    .iter()
                    .all(|stitch| stitch.kind != StitchKind::Trim));
                assert_eq!(plan, {
                    let mut again = StitchPlan::new("underlay");
                    generate_text(&mut again, &item).unwrap();
                    again
                });
            }
            assert!(
                sequences.iter().all(|item| item == &sequences[0]),
                "{content} top sequence changed with underlay: {sequences:?}"
            );
            assert!(
                counts[0] < counts[1],
                "{content} centre run added no stitches"
            );
            assert!(
                counts[0] < counts[2],
                "{content} edge run added no stitches"
            );
            assert!(
                counts[1] < counts[3],
                "{content} both passes did not exceed centre"
            );
            assert!(
                counts[2] < counts[3],
                "{content} both passes did not exceed edge"
            );
        }
    }

    #[test]
    fn rejected_text_leaves_a_prepopulated_plan_unchanged() {
        let hash = font_content_sha256();
        let long_text = "A".repeat(33);
        let lower = request("a", 12.0, &hash);
        let accent = request("é", 12.0, &hash);
        let curly = request("O\u{2019}NEIL", 12.0, &hash);
        let empty = request("", 12.0, &hash);
        let blanks = request("   ", 12.0, &hash);
        let zero = request("A", 0.0, &hash);
        let huge = request("A", 250.0, &hash);
        let mut negative = request("A", 12.0, &hash);
        negative.letter_spacing_mm = -1.0;
        let too_long = request(&long_text, 12.0, &hash);
        let mut wrong_id = request("A", 12.0, &hash);
        wrong_id.font_id = "other-font";
        let mut wrong_version = request("A", 12.0, &hash);
        wrong_version.font_version = "9";
        let wrong_hash = request("A", 12.0, "pending");
        let mut hoop = request("AW", 20.0, &hash);
        hoop.max_x = 30.0;
        let mut wide = request("A", 12.0, &hash);
        wide.column_width_mm = 13.0;
        let tab = request("A\tB", 12.0, &hash);
        let mixed = request("Alpine", 12.0, &hash);
        let mut far_spacing = request("AA", 12.0, &hash);
        far_spacing.letter_spacing_mm = 1001.0;

        let samples = [
            ("lowercase", &lower),
            ("accent", &accent),
            ("curly", &curly),
            ("empty", &empty),
            ("blanks", &blanks),
            ("zero cap", &zero),
            ("huge cap", &huge),
            ("negative spacing", &negative),
            ("33 characters", &too_long),
            ("wrong font id", &wrong_id),
            ("wrong version", &wrong_version),
            ("wrong hash", &wrong_hash),
            ("hoop", &hoop),
            ("wide column", &wide),
            ("tab", &tab),
            ("mixed case", &mixed),
            ("pathological spacing", &far_spacing),
        ];
        assert!(samples.len() >= 12);
        for (name, item) in samples {
            let mut plan = occupied();
            let before = plan.clone();
            let message = reject(item, &mut plan);
            assert_preserved(&before, &plan);
            assert!(
                message.contains("The plan was not changed."),
                "{name}: {message}"
            );
            match name {
                "lowercase" => {
                    assert!(
                        message.contains('a') && message.contains("U+0061"),
                        "{message}"
                    );
                    assert!(message.contains("left unchanged"), "{message}");
                }
                "accent" => assert!(
                    message.contains('é') && message.contains("U+00E9"),
                    "{message}"
                ),
                "curly" => assert!(message.contains("U+2019"), "{message}"),
                "mixed case" => {
                    assert!(
                        message.contains('l') && message.contains("U+006C"),
                        "{message}"
                    );
                    assert!(!message.to_ascii_lowercase().contains("replaced with"));
                }
                "tab" => assert!(message.contains("U+0009"), "{message}"),
                "wrong font id" | "wrong version" | "wrong hash" => {
                    assert!(message.contains(PINNED_FONT_SHA256), "{message}");
                    assert!(message.contains("was not replaced"), "{message}");
                }
                "33 characters" => assert!(message.contains("32"), "{message}"),
                "pathological spacing" => assert!(message.contains("not shrunk"), "{message}"),
                "wide column" => assert!(
                    message.contains("12.1") || message.contains("121"),
                    "{message}"
                ),
                _ => {}
            }
        }
    }

    #[test]
    fn text_appends_to_an_existing_plan_without_removing_it() {
        let mut plan = occupied();
        let before = plan.stitches.len();
        let kept = plan.stitches.clone();
        let hash = font_content_sha256();
        let item = request("A", 12.0, &hash);
        generate_text(&mut plan, &item).unwrap();
        assert_eq!(&plan.stitches[..before], kept.as_slice());
        assert!(plan.stitches.len() > before);
        assert!(plan.stitches[before..]
            .iter()
            .all(|stitch| stitch.object_id == 3));
    }
}
