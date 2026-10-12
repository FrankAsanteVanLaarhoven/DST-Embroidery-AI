//! EmbroideryOS Block 01 version 2.
//!
//! Version 1 and its recipe bytes stay in [`super::font_recipe`]. This module
//! is a separate pinned artifact. Shared capitals, digits, space, hyphen, and
//! the straight apostrophe reuse those version-1 strokes. Lowercase, U+2019,
//! and the declared accents are original recipes.
//!
//! The stored text is never rewritten. Version 2 derives a layout string with
//! NFC from `unicode-normalization` 0.1.24 and segments grapheme clusters with
//! `unicode-segmentation` 1.12.0. Compatibility normalization is not used.
//! Only the declared characters and their canonical equivalents are sewn.

use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::error::PlanError;
use crate::generators::{generate_satin, SatinConfig};
use crate::lettering::{
    advance_mm, append_meta, check_ranges, glyph, inside_hoop, inside_margin, letter_gap,
    push_trace, stopped, Stroke, TextRequest, CAP_UNITS, FONT_ID, FONT_NAME, MAX_COMPONENTS,
    MAX_TEXT_CHARS,
};
use crate::model::{Point, StitchKind, StitchPlan};
use crate::sha256::sha256_hex;

pub const FONT_VERSION_2: &str = "2";
pub const NORMALIZATION_CRATE: &str = "=0.1.24";
pub const SEGMENTATION_CRATE: &str = "=1.12.0";

/// Pinned Unicode data carried by `unicode-normalization` 0.1.24.
pub const UNICODE_DATA_VERSION: (u8, u8, u8) = (16, 0, 0);

const MAX_TEXT_BYTES: usize = 256;
const MAX_CODEPOINTS: usize = 128;
const MAX_COMBINING_MARKS: usize = 32;

/// Version 1 repertoire, then lowercase, U+2019, then the declared accent pairs.
pub const REPERTOIRE_V2: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -'abcdefghijklmnopqrstuvwxyz\u{2019}\u{00C1}\u{00E1}\u{00C9}\u{00E9}\u{00CD}\u{00ED}\u{00D3}\u{00F3}\u{00DA}\u{00FA}\u{00C4}\u{00E4}\u{00CB}\u{00EB}\u{00CF}\u{00EF}\u{00D6}\u{00F6}\u{00DC}\u{00FC}\u{00D1}\u{00F1}\u{00C7}\u{00E7}";

const BASELINE: i16 = 92;
const CAP_TOP: i16 = 8;
const X_TOP: i16 = 50;
const ASCENDER_TOP: i16 = 8;
const DESCENDER_BOTTOM: i16 = 132;
const ACCENT_TOP: i16 = -28;

const CAP_ACUTE: &[Stroke] = &[(16, -16, 46, -28)];
const CAP_DIAERESIS: &[Stroke] = &[(10, -28, 10, -16), (40, -28, 40, -16)];
const CAP_TILDE: &[Stroke] = &[(8, -18, 28, -28), (28, -28, 48, -18)];
const CEDILLA: &[Stroke] = &[(32, 116, 18, 128), (18, 128, 36, 132)];
const LOW_ACUTE: &[Stroke] = &[(6, 26, 38, 12)];
const LOW_DIAERESIS: &[Stroke] = &[(4, 26, 4, 12), (36, 26, 36, 12)];
const LOW_TILDE: &[Stroke] = &[(4, 24, 24, 12), (24, 12, 44, 24)];
const I_STEM: &[Stroke] = &[(16, 58, 16, 92)];
const I_DOT: Stroke = (16, 34, 16, 18);

const LOWER_A: &[Stroke] = &[
    (40, 50, 40, 92),
    (14, 64, 40, 64),
    (14, 64, 14, 92),
    (14, 92, 40, 92),
];
const LOWER_B: &[Stroke] = &[
    (14, 8, 14, 92),
    (14, 60, 42, 60),
    (42, 60, 42, 92),
    (14, 92, 42, 92),
];
const LOWER_C: &[Stroke] = &[(16, 60, 16, 86), (16, 60, 40, 60), (16, 92, 40, 92)];
const LOWER_D: &[Stroke] = &[
    (42, 8, 42, 92),
    (14, 60, 42, 60),
    (14, 60, 14, 92),
    (14, 92, 42, 92),
];
const LOWER_E: &[Stroke] = &[
    (14, 60, 40, 60),
    (14, 60, 14, 92),
    (14, 76, 36, 76),
    (14, 92, 40, 92),
];
const LOWER_F: &[Stroke] = &[(22, 8, 22, 92), (8, 8, 36, 8), (8, 50, 36, 50)];
const LOWER_G: &[Stroke] = &[
    (14, 60, 14, 86),
    (40, 60, 40, 86),
    (14, 60, 40, 60),
    (14, 86, 40, 86),
    (40, 86, 40, 118),
    (14, 118, 40, 118),
];
const LOWER_H: &[Stroke] = &[(14, 8, 14, 92), (14, 58, 42, 58), (42, 58, 42, 92)];
const LOWER_I: &[Stroke] = &[(16, 58, 16, 92), I_DOT];
const LOWER_J: &[Stroke] = &[(24, 58, 24, 110), (8, 118, 24, 110), (24, 34, 24, 18)];
const LOWER_K: &[Stroke] = &[(14, 8, 14, 92), (14, 74, 44, 56), (14, 74, 44, 92)];
const LOWER_L: &[Stroke] = &[(16, 8, 16, 92), (16, 92, 30, 92)];
const LOWER_M: &[Stroke] = &[
    (10, 58, 10, 92),
    (10, 58, 32, 58),
    (32, 58, 32, 92),
    (32, 58, 56, 58),
    (56, 58, 56, 92),
];
const LOWER_N: &[Stroke] = &[(14, 58, 14, 92), (14, 58, 42, 58), (42, 58, 42, 92)];
const LOWER_O: &[Stroke] = &[
    (14, 60, 14, 86),
    (40, 60, 40, 86),
    (14, 60, 40, 60),
    (14, 92, 40, 92),
];
const LOWER_P: &[Stroke] = &[
    (14, 58, 14, 118),
    (14, 58, 42, 58),
    (42, 58, 42, 92),
    (14, 92, 42, 92),
];
const LOWER_Q: &[Stroke] = &[
    (42, 58, 42, 118),
    (14, 58, 42, 58),
    (14, 58, 14, 92),
    (14, 92, 42, 92),
];
const LOWER_R: &[Stroke] = &[(14, 58, 14, 92), (14, 58, 36, 58), (36, 58, 36, 72)];
const LOWER_S: &[Stroke] = &[
    (12, 58, 40, 58),
    (12, 58, 12, 74),
    (12, 74, 40, 74),
    (40, 74, 40, 92),
    (12, 92, 40, 92),
];
const LOWER_T: &[Stroke] = &[(22, 20, 22, 92), (8, 50, 36, 50), (12, 92, 34, 92)];
const LOWER_U: &[Stroke] = &[(14, 58, 14, 86), (40, 58, 40, 92), (14, 92, 40, 92)];
const LOWER_V: &[Stroke] = &[(10, 58, 26, 92), (42, 58, 26, 92)];
const LOWER_W: &[Stroke] = &[
    (8, 58, 20, 92),
    (32, 58, 20, 92),
    (32, 58, 44, 92),
    (60, 58, 44, 92),
];
const LOWER_X: &[Stroke] = &[(12, 58, 42, 92), (42, 58, 12, 92)];
const LOWER_Y: &[Stroke] = &[(10, 58, 26, 92), (44, 58, 16, 118)];
const LOWER_Z: &[Stroke] = &[(12, 58, 42, 58), (42, 58, 12, 92), (12, 92, 42, 92)];
const CURLY_QUOTE: &[Stroke] = &[(6, -6, 16, 10), (16, 10, 8, 26)];

pub fn font_v2_recipe() -> String {
    let mut recipe = format!(
        "EmbroideryOS Block 01\nembroideryos-block-01\n2\ny-down\nscale-box 100\nbaseline {BASELINE}\ncap-top {CAP_TOP}\ncap-body {}\nx-top {X_TOP}\nx-height {}\nascender-top {ASCENDER_TOP}\ndescender-bottom {DESCENDER_BOTTOM}\naccent-top {ACCENT_TOP}\nnormalization NFC\nunicode-normalization {NORMALIZATION_CRATE}\nunicode-segmentation {SEGMENTATION_CRATE}\nunicode-data {}.{}.{}\n",
        BASELINE - CAP_TOP,
        BASELINE - X_TOP,
        UNICODE_DATA_VERSION.0,
        UNICODE_DATA_VERSION.1,
        UNICODE_DATA_VERSION.2
    );
    for character in REPERTOIRE_V2.chars() {
        let (advance, strokes) = strokes_of(character).expect("version 2 repertoire entry");
        recipe.push_str(&format!("U+{:04X} advance {advance}\n", character as u32));
        for stroke in &strokes {
            recipe.push_str(&format!(
                "{} {} {} {}\n",
                stroke.0, stroke.1, stroke.2, stroke.3
            ));
        }
    }
    recipe
}

pub fn font_v2_content_sha256() -> String {
    sha256_hex(font_v2_recipe().as_bytes())
}

/// NFC layout string. This does not replace the entered text.
pub(crate) fn layout_text(content: &str) -> String {
    content.nfc().collect()
}

pub(crate) fn strokes_of(character: char) -> Option<(i16, Vec<Stroke>)> {
    if let Some(item) = glyph(character) {
        return Some((item.advance, item.strokes.to_vec()));
    }
    if let Some(marked) = marked_capital(character) {
        return Some(marked);
    }
    original(character)
}

pub(crate) fn sew(plan: &mut StitchPlan, request: &TextRequest<'_>) -> Result<(), PlanError> {
    pin_version_two(request)?;
    if unicode_normalization::UNICODE_VERSION != UNICODE_DATA_VERSION {
        return Err(stopped(
            "EmbroideryOS Block 01 version 2 is pinned to Unicode 16.0.0. The installed normalization data does not match. Nothing was generated. The plan was not changed.".to_string(),
        ));
    }
    check_ranges(request)?;
    let layout = derive_layout(request)?;
    let characters: Vec<char> = layout.chars().collect();
    if characters.is_empty() || characters.iter().all(|character| *character == ' ') {
        return Err(stopped(format!(
            "Object {} has no sewing characters. Enter text that includes a letter or digit, then generate again.",
            request.object_id
        )));
    }
    let glyphs: Vec<(char, i16, Vec<Stroke>)> = characters
        .iter()
        .copied()
        .map(|character| {
            let (advance, strokes) = strokes_of(character).expect("layout checked");
            (character, advance, strokes)
        })
        .collect();

    let scale = request.cap_height_mm / CAP_UNITS;
    let margin = request.column_width_mm / 2.0 + request.pull_compensation_mm;
    let mut component_count = 0usize;
    let mut pen = 0.0;
    for (index, (character, advance, strokes)) in glyphs.iter().enumerate() {
        if *character != ' ' {
            component_count += strokes.len();
        }
        if component_count > MAX_COMPONENTS {
            return Err(stopped(format!(
                "Object {} expands to more than {MAX_COMPONENTS} glyph components. Shorten the text and generate again.",
                request.object_id
            )));
        }
        if *character != ' ' {
            for stroke in strokes {
                for (x, y) in [(stroke.0, stroke.1), (stroke.2, stroke.3)] {
                    let point = Point::new(
                        request.origin_x_mm + pen + f64::from(x) * scale,
                        request.origin_y_mm + f64::from(y) * scale,
                    );
                    if !inside_margin(point, margin, request) {
                        return Err(stopped(format!(
                            "Object {} places {character} outside the hoop at ({:.2}, {:.2}) mm. The full letter, including marks and descenders, was measured. Move the text or reduce the cap height, spacing, column width, or pull compensation, then generate again.",
                            request.object_id, point.x, point.y
                        )));
                    }
                }
            }
        }
        pen += advance_mm(*character, *advance, scale, request);
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
    for (index, (character, advance, strokes)) in glyphs.iter().enumerate() {
        if *character == ' ' {
            push_trace(
                &mut trace,
                &format!(
                    "object {} glyph {index} U+0020 space advance {:.3}",
                    request.object_id, request.word_spacing_mm
                ),
            );
        } else {
            for (component, stroke) in strokes.iter().enumerate() {
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
                let visible = plan.stitches[from..to]
                    .iter()
                    .any(|stitch| stitch.kind.is_sewing());
                if !visible {
                    return Err(stopped(format!(
                        "Object {} did not sew component {component} of {character} (U+{:04X}). The mark was not dropped and the character was not replaced by a base letter.",
                        request.object_id,
                        *character as u32
                    )));
                }
                sewn_components += 1;
                push_trace(
                    &mut trace,
                    &format!(
                        "object {} glyph {index} U+{:04X} {} component {component} stitches {from}-{to}",
                        request.object_id,
                        *character as u32,
                        label_for(*character)
                    ),
                );
            }
        }
        pen += advance_mm(*character, *advance, scale, request);
        if letter_gap(index, &characters) {
            pen += request.letter_spacing_mm;
        }
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
    append_meta(
        plan,
        "lettering_layout",
        &format!("object {} NFC {layout}", request.object_id),
    );
    append_meta(
        plan,
        "lettering_normalization",
        &format!(
            "object {} NFC unicode-normalization {NORMALIZATION_CRATE} unicode-segmentation {SEGMENTATION_CRATE} unicode-data {}.{}.{}",
            request.object_id,
            UNICODE_DATA_VERSION.0,
            UNICODE_DATA_VERSION.1,
            UNICODE_DATA_VERSION.2
        ),
    );
    append_meta(plan, "lettering_trace", &trace);
    append_meta(
        plan,
        "lettering_summary",
        &format!(
            "EmbroideryOS Block 01 version 2 object {}: {} graphemes, {sewn_components} components. Cap height {:.3} mm is a software size, not the full line height and not a measured sewing limit.",
            request.object_id,
            characters.len(),
            request.cap_height_mm
        ),
    );
    Ok(())
}

pub(crate) fn pin_version_two(request: &TextRequest<'_>) -> Result<(), PlanError> {
    let expected = font_v2_content_sha256();
    if request.font_id == FONT_ID
        && request.font_version == FONT_VERSION_2
        && request.font_sha256 == expected
    {
        return Ok(());
    }
    Err(stopped(format!(
        "Object {} is pinned to font {} version {} {}. This engine sews {FONT_NAME} version {FONT_VERSION_2} {expected}. Version 1 was not used as a replacement. The saved font was not replaced.",
        request.object_id, request.font_id, request.font_version, request.font_sha256
    )))
}

pub(crate) fn derive_layout(request: &TextRequest<'_>) -> Result<String, PlanError> {
    let content = request.content;
    if content.len() > MAX_TEXT_BYTES {
        return Err(stopped(format!(
            "Object {} has {} bytes. This lettering step allows at most {MAX_TEXT_BYTES} bytes. A character count alone does not bound the input. Shorten the text and generate again. The text was left unchanged.",
            request.object_id,
            content.len()
        )));
    }
    let codepoints = content.chars().count();
    if codepoints > MAX_CODEPOINTS {
        return Err(stopped(format!(
            "Object {} has {codepoints} code points. This lettering step allows at most {MAX_CODEPOINTS}. Shorten the text and generate again. The text was left unchanged.",
            request.object_id
        )));
    }
    let graphemes: Vec<&str> = content.graphemes(true).collect();
    if graphemes.len() > MAX_TEXT_CHARS {
        return Err(stopped(format!(
            "Object {} has {} grapheme clusters. This lettering step allows at most {MAX_TEXT_CHARS}. A character count alone does not bound the input. Shorten the text and generate again. The text was left unchanged.",
            request.object_id,
            graphemes.len()
        )));
    }
    let marks = content
        .chars()
        .filter(|character| canonical_combining_class(*character) > 0)
        .count();
    if marks > MAX_COMBINING_MARKS {
        return Err(stopped(format!(
            "Object {} has {marks} combining marks. This lettering step allows at most {MAX_COMBINING_MARKS}. Marks were not removed. The text was left unchanged.",
            request.object_id
        )));
    }
    for grapheme in &graphemes {
        let mark_count = grapheme
            .chars()
            .filter(|character| canonical_combining_class(*character) > 0)
            .count();
        if mark_count > 1 {
            return Err(stopped(format!(
                "Object {} contains the combining sequence {} ({}). Only one declared mark is supported. Marks were not removed. The text was left unchanged.",
                request.object_id,
                grapheme,
                code_list(grapheme)
            )));
        }
    }
    let layout = layout_text(content);
    for (index, grapheme) in layout.graphemes(true).enumerate() {
        let mut scalars = grapheme.chars();
        let Some(character) = scalars.next() else {
            continue;
        };
        if scalars.next().is_some() || strokes_of(character).is_none() {
            return Err(stopped(format!(
                "Object {} contains {grapheme} ({}) at position {}. EmbroideryOS Block 01 version 2 does not sew that sequence. The accent was not removed and the character was not replaced. The text was left unchanged.",
                request.object_id,
                code_list(grapheme),
                index + 1
            )));
        }
    }
    Ok(layout)
}

fn marked_capital(character: char) -> Option<(i16, Vec<Stroke>)> {
    let (base, marks) = match character {
        '\u{00C1}' | '\u{00C9}' | '\u{00CD}' | '\u{00D3}' | '\u{00DA}' => {
            let base = match character {
                '\u{00C1}' => 'A',
                '\u{00C9}' => 'E',
                '\u{00CD}' => 'I',
                '\u{00D3}' => 'O',
                _ => 'U',
            };
            (base, CAP_ACUTE)
        }
        '\u{00C4}' | '\u{00CB}' | '\u{00CF}' | '\u{00D6}' | '\u{00DC}' => {
            let base = match character {
                '\u{00C4}' => 'A',
                '\u{00CB}' => 'E',
                '\u{00CF}' => 'I',
                '\u{00D6}' => 'O',
                _ => 'U',
            };
            (base, CAP_DIAERESIS)
        }
        '\u{00D1}' => ('N', CAP_TILDE),
        '\u{00C7}' => ('C', CEDILLA),
        _ => return None,
    };
    let item = glyph(base)?;
    let mut strokes = item.strokes.to_vec();
    strokes.extend_from_slice(marks);
    Some((item.advance, strokes))
}

fn original(character: char) -> Option<(i16, Vec<Stroke>)> {
    let (advance, body, marks) = match character {
        'a' => (52, LOWER_A, &[][..]),
        'b' => (52, LOWER_B, &[][..]),
        'c' => (48, LOWER_C, &[][..]),
        'd' => (52, LOWER_D, &[][..]),
        'e' => (48, LOWER_E, &[][..]),
        'f' => (40, LOWER_F, &[][..]),
        'g' => (52, LOWER_G, &[][..]),
        'h' => (52, LOWER_H, &[][..]),
        'i' => (28, LOWER_I, &[][..]),
        'j' => (36, LOWER_J, &[][..]),
        'k' => (52, LOWER_K, &[][..]),
        'l' => (32, LOWER_L, &[][..]),
        'm' => (76, LOWER_M, &[][..]),
        'n' => (52, LOWER_N, &[][..]),
        'o' => (52, LOWER_O, &[][..]),
        'p' => (52, LOWER_P, &[][..]),
        'q' => (52, LOWER_Q, &[][..]),
        'r' => (40, LOWER_R, &[][..]),
        's' => (48, LOWER_S, &[][..]),
        't' => (40, LOWER_T, &[][..]),
        'u' => (52, LOWER_U, &[][..]),
        'v' => (52, LOWER_V, &[][..]),
        'w' => (76, LOWER_W, &[][..]),
        'x' => (52, LOWER_X, &[][..]),
        'y' => (52, LOWER_Y, &[][..]),
        'z' => (48, LOWER_Z, &[][..]),
        '\u{2019}' => (22, CURLY_QUOTE, &[][..]),
        '\u{00E1}' | '\u{00E4}' => (
            52,
            LOWER_A,
            if character == '\u{00E1}' {
                LOW_ACUTE
            } else {
                LOW_DIAERESIS
            },
        ),
        '\u{00E9}' | '\u{00EB}' => (
            48,
            LOWER_E,
            if character == '\u{00E9}' {
                LOW_ACUTE
            } else {
                LOW_DIAERESIS
            },
        ),
        '\u{00ED}' | '\u{00EF}' => (
            28,
            I_STEM,
            if character == '\u{00ED}' {
                LOW_ACUTE
            } else {
                LOW_DIAERESIS
            },
        ),
        '\u{00F3}' | '\u{00F6}' => (
            52,
            LOWER_O,
            if character == '\u{00F3}' {
                LOW_ACUTE
            } else {
                LOW_DIAERESIS
            },
        ),
        '\u{00FA}' | '\u{00FC}' => (
            52,
            LOWER_U,
            if character == '\u{00FA}' {
                LOW_ACUTE
            } else {
                LOW_DIAERESIS
            },
        ),
        '\u{00F1}' => (52, LOWER_N, LOW_TILDE),
        '\u{00E7}' => (48, LOWER_C, CEDILLA),
        _ => return None,
    };
    let mut strokes = body.to_vec();
    strokes.extend_from_slice(marks);
    Some((advance, strokes))
}

fn label_for(character: char) -> &'static str {
    match character {
        '\'' => "apostrophe",
        '\u{2019}' => "quotation",
        '-' => "hyphen",
        _ => "glyph",
    }
}

fn code_list(grapheme: &str) -> String {
    grapheme
        .chars()
        .map(|character| format!("U+{:04X}", character as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::{generate_running, RunningConfig};
    use crate::lettering::{font_content_sha256, generate_text, FONT_VERSION};

    fn request<'a>(content: &'a str, cap: f64, font_sha256: &'a str) -> TextRequest<'a> {
        TextRequest {
            object_id: 3,
            content,
            font_id: FONT_ID,
            font_version: FONT_VERSION_2,
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

    fn sew(content: &str, cap: f64) -> StitchPlan {
        let hash = font_v2_content_sha256();
        let item = request(content, cap, &hash);
        let mut plan = StitchPlan::new("letter");
        generate_text(&mut plan, &item).unwrap_or_else(|error| panic!("{content}: {error}"));
        plan
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

    #[test]
    fn version_two_hash_is_independent_of_version_one() {
        assert_eq!(unicode_normalization::UNICODE_VERSION, UNICODE_DATA_VERSION);
        assert_eq!(REPERTOIRE_V2.chars().count(), 90);
        let hash = font_v2_content_sha256();
        assert_eq!(hash, font_v2_content_sha256());
        assert_ne!(hash, font_content_sha256());
        assert_eq!(
            font_content_sha256(),
            "aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd"
        );
        assert_eq!(
            hash, "78e8506438960cc575d896ba338d65d711144bb5acc956205dbb28c081074a89",
            "pin Block 01 version 2 content hash"
        );
        assert!(font_v2_recipe().contains("baseline 92"));
        assert!(font_v2_recipe().contains("x-height 42"));
        assert!(font_v2_recipe().contains("descender-bottom 132"));
        assert!(font_v2_recipe().contains("accent-top -28"));
        assert!(font_v2_recipe().contains("normalization NFC"));
    }

    #[test]
    fn normalization_preserves_source_and_rejects_unlisted_sequences() {
        let composed: String = "Café".nfc().collect();
        let decomposed: String = "Cafe\u{0301}".nfc().collect();
        assert_eq!(composed, decomposed);
        assert_ne!("Cafe\u{0301}", "Café");
        let ligature: String = "ﬁ".nfc().collect();
        assert_eq!(ligature, "ﬁ");
        let compatibility: String = "ﬁ".nfkc().collect();
        assert_eq!(compatibility, "fi");
        let clusters: Vec<&str> = "Cafe\u{0301}".graphemes(true).collect();
        assert_eq!(clusters, ["C", "a", "f", "e\u{0301}"]);
        let doubled: Vec<&str> = "e\u{0301}\u{0301}".graphemes(true).collect();
        assert_eq!(doubled, ["e\u{0301}\u{0301}"]);

        let hash = font_v2_content_sha256();
        let mut plan = occupied();
        let before = plan.clone();
        let doubled_text = "e\u{0301}\u{0301}";
        let item = request(doubled_text, 12.0, &hash);
        let message = generate_text(&mut plan, &item)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains("U+0301"), "{message}");
        assert!(message.contains("Marks were not removed"), "{message}");
        assert!(message.contains("The plan was not changed."), "{message}");

        let mut plan = occupied();
        let before = plan.clone();
        let circumflex = request("e\u{0302}", 12.0, &hash);
        let message = generate_text(&mut plan, &circumflex)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains("U+00EA"), "{message}");
        assert!(message.contains("not removed"), "{message}");
        assert!(!message.contains("replaced by e"), "{message}");

        let mut plan = occupied();
        let before = plan.clone();
        let ligature_request = request("ﬁ", 12.0, &hash);
        let message = generate_text(&mut plan, &ligature_request)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains("U+FB01"), "{message}");
        assert!(message.contains("not replaced"), "{message}");
        assert!(!message.contains("fi"), "{message}");
    }

    #[test]
    fn declared_glyphs_are_admitted_at_three_software_heights() {
        // Reference configuration, declared before evaluation:
        // origin (8, 8) mm, hoop 0–220 × 0–80 mm, density 1.5 mm,
        // column = 0.16 × cap, pull 0, letter spacing 1 mm, word spacing 4 mm,
        // underlay off. Every version-2 character is expected to be admitted
        // at 8, 12, and 20 mm. Density, spacing, and size are not adjusted.
        let hash = font_v2_content_sha256();
        for character in REPERTOIRE_V2.chars().filter(|character| *character != ' ') {
            for cap in [8.0, 12.0, 20.0] {
                let content = character.to_string();
                let item = request(&content, cap, &hash);
                let mut plan = StitchPlan::new("glyph");
                generate_text(&mut plan, &item)
                    .unwrap_or_else(|error| panic!("{character} at {cap}: {error}"));
                assert!(plan.sewing_stitch_count() > 0, "{character} at {cap}");
                assert!(plan
                    .stitches
                    .iter()
                    .all(|stitch| stitch.kind != StitchKind::Trim));
                assert!(plan.metadata["lettering_summary"].contains("version 2"));
                assert!(plan.metadata["lettering_summary"].contains("not the full line height"));
                assert_eq!(
                    plan.metadata["lettering_text"],
                    format!("object 3 {content}")
                );
                let ranges = component_ranges(&plan);
                assert!(!ranges.is_empty(), "{character}");
                for (start, end) in ranges {
                    assert!(
                        plan.stitches[start..end]
                            .iter()
                            .any(|stitch| stitch.kind.is_sewing()),
                        "{character} at {cap} dropped a component"
                    );
                }
            }
        }
    }

    #[test]
    fn narrow_clearances_keep_their_declared_outcomes() {
        // Cap 12 mm, column 1.92 mm, margin 0.96 mm, origin (8, 8).
        // 'g' descender endpoint is 22.16 mm. max_y 23 mm rejects 'g' and admits 'A'.
        let hash = font_v2_content_sha256();
        let mut tight = request("g", 12.0, &hash);
        tight.max_y = 23.0;
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &tight)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains('g'), "{message}");
        assert!(message.contains("The plan was not changed."), "{message}");
        let mut capital = request("A", 12.0, &hash);
        capital.max_y = 23.0;
        let mut admitted = StitchPlan::new("capital");
        generate_text(&mut admitted, &capital).expect("A fits above the descender limit");

        // Cap 20 mm, column 3.2 mm, margin 1.6 mm, origin (8, 8).
        // Accent endpoint of Á is 2.40 mm. min_y 1.0 mm rejects Á and admits A.
        let mut high = request("Á", 20.0, &hash);
        high.min_y = 1.0;
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &high)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains('Á'), "{message}");
        let mut plain = request("A", 20.0, &hash);
        plain.min_y = 1.0;
        let mut admitted = StitchPlan::new("plain");
        generate_text(&mut admitted, &plain).expect("A fits below the accent limit");
    }

    #[test]
    fn accented_i_is_authored_without_the_tittle() {
        let (advance, strokes) = strokes_of('\u{00ED}').unwrap();
        assert_eq!(advance, 28);
        assert!(strokes.contains(&I_STEM[0]));
        assert!(!strokes.contains(&I_DOT));
        assert_eq!(strokes.len(), 2);
        let (_advance, diaeresis) = strokes_of('\u{00EF}').unwrap();
        assert!(!diaeresis.contains(&I_DOT));
        assert_eq!(diaeresis.len(), 3);
        let (_advance, dotted) = strokes_of('i').unwrap();
        assert!(dotted.contains(&I_DOT));
        assert_ne!(sew("i", 12.0).stitches, sew("í", 12.0).stitches);
        assert_ne!(
            sew("O'Neil", 12.0).stitches,
            sew("O\u{2019}Neil", 12.0).stitches
        );
    }

    #[test]
    fn marks_stay_disconnected_and_underlays_keep_the_top_sequence() {
        for content in ["i", "é", "g", "Ç", "a"] {
            let plan = sew(content, 12.0);
            if content == "i" || content == "é" || content == "Ç" {
                assert_mark_separated(&plan, content);
            }
            let modes = [
                (None, None),
                (Some(2.5), None),
                (None, Some(2.5)),
                (Some(2.5), Some(2.5)),
            ];
            let mut sequences = Vec::new();
            let mut counts = Vec::new();
            for (centre, edge) in modes {
                let hash = font_v2_content_sha256();
                let mut item = request(content, 12.0, &hash);
                item.centre_run_mm = centre;
                item.edge_run_mm = edge;
                let mut sewn = StitchPlan::new("underlay");
                generate_text(&mut sewn, &item)
                    .unwrap_or_else(|error| panic!("{content}: {error}"));
                sequences.push(sewn.metadata["satin_top_sequence"].clone());
                counts.push(sewn.stitches.len());
                let phases = &sewn.metadata["satin_phase_summary"];
                assert!(phases.contains("Centre-run underlay"), "{content}");
                assert!(phases.contains("Edge-run underlay"), "{content}");
                assert!(phases.contains("Top satin"), "{content}");
                assert!(
                    phases.contains("Positioning and inter-phase connectors"),
                    "{content}"
                );
                assert!(sewn
                    .stitches
                    .iter()
                    .all(|stitch| stitch.kind != StitchKind::Trim));
                if content == "i" || content == "é" || content == "Ç" {
                    assert_mark_separated(&sewn, content);
                }
            }
            assert!(
                sequences.iter().all(|item| item == &sequences[0]),
                "{content}"
            );
            assert!(
                counts[0] < counts[1] && counts[0] < counts[2],
                "{content} {counts:?}"
            );
            assert!(
                counts[1] < counts[3] && counts[2] < counts[3],
                "{content} {counts:?}"
            );
        }
    }

    #[test]
    fn canonical_pairs_match_geometry_and_keep_source_text() {
        let pairs = [
            ("Á", "A\u{0301}"),
            ("á", "a\u{0301}"),
            ("É", "E\u{0301}"),
            ("é", "e\u{0301}"),
            ("Í", "I\u{0301}"),
            ("í", "i\u{0301}"),
            ("Ó", "O\u{0301}"),
            ("ó", "o\u{0301}"),
            ("Ú", "U\u{0301}"),
            ("ú", "u\u{0301}"),
            ("Ä", "A\u{0308}"),
            ("ä", "a\u{0308}"),
            ("Ë", "E\u{0308}"),
            ("ë", "e\u{0308}"),
            ("Ï", "I\u{0308}"),
            ("ï", "i\u{0308}"),
            ("Ö", "O\u{0308}"),
            ("ö", "o\u{0308}"),
            ("Ü", "U\u{0308}"),
            ("ü", "u\u{0308}"),
            ("Ñ", "N\u{0303}"),
            ("ñ", "n\u{0303}"),
            ("Ç", "C\u{0327}"),
            ("ç", "c\u{0327}"),
        ];
        assert!(pairs.len() >= 12);
        for (composed, decomposed) in pairs {
            assert_ne!(composed, decomposed);
            let left = sew(composed, 12.0);
            let right = sew(decomposed, 12.0);
            assert_eq!(left.stitches, right.stitches, "{composed}");
            assert_eq!(
                left.metadata["lettering_layout"],
                right.metadata["lettering_layout"]
            );
            assert_eq!(
                left.metadata["lettering_text"],
                format!("object 3 {composed}")
            );
            assert_eq!(
                right.metadata["lettering_text"],
                format!("object 3 {decomposed}")
            );
            assert_ne!(
                left.metadata["lettering_text"],
                right.metadata["lettering_text"]
            );
        }
        let cafe = sew("Café", 12.0);
        let cafe_nfd = sew("Cafe\u{0301}", 12.0);
        assert_eq!(cafe.stitches, cafe_nfd.stitches);
        assert_ne!(
            cafe.metadata["lettering_text"],
            cafe_nfd.metadata["lettering_text"]
        );
    }

    #[test]
    fn version_one_does_not_normalize_or_upgrade() {
        let hash = font_content_sha256();
        let mut item = request("CAFE\u{0301}", 12.0, &hash);
        item.font_version = FONT_VERSION;
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &item)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains("U+0301"), "{message}");
        assert!(!message.contains("U+00C9"), "{message}");

        let v2_hash = font_v2_content_sha256();
        let mut wrong = request("A", 12.0, &hash);
        wrong.font_version = FONT_VERSION;
        wrong.font_sha256 = &v2_hash;
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &wrong)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains("was not replaced"), "{message}");
        assert!(message.contains(&hash), "{message}");

        let mut version_two = request("Café", 12.0, "pending");
        version_two.font_sha256 = "pending";
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &version_two)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert!(message.contains("version 2"), "{message}");
        assert!(message.contains("was not replaced"), "{message}");
        assert!(message.contains(&v2_hash), "{message}");
    }

    #[test]
    fn a_failing_final_character_restores_the_plan() {
        let hash = font_v2_content_sha256();
        let mut plan = occupied();
        let before = plan.clone();
        let item = request("Ab\u{00EA}", 12.0, &hash);
        let message = generate_text(&mut plan, &item)
            .unwrap_err()
            .message()
            .to_string();
        assert_eq!(plan.stitches, before.stitches);
        assert_eq!(plan.metadata, before.metadata);
        assert!(message.contains("U+00EA"), "{message}");
        assert!(message.contains("The plan was not changed."), "{message}");
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

    fn band(plan: &StitchPlan, range: (usize, usize)) -> (f64, f64) {
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for stitch in &plan.stitches[range.0..range.1] {
            if !stitch.kind.is_sewing() {
                continue;
            }
            for point in [stitch.from, stitch.to] {
                min_y = min_y.min(point.y);
                max_y = max_y.max(point.y);
            }
        }
        (min_y, max_y)
    }

    fn assert_mark_separated(plan: &StitchPlan, content: &str) {
        let ranges = component_ranges(plan);
        assert!(ranges.len() >= 2, "{content}");
        let body = band(plan, ranges[0]);
        let mark = band(plan, *ranges.last().unwrap());
        assert!(body.0.is_finite() && mark.0.is_finite(), "{content}");
        let gap = if mark.1 < body.0 {
            body.0 - mark.1
        } else if body.1 < mark.0 {
            mark.0 - body.1
        } else {
            0.0
        };
        assert!(
            gap > 0.2,
            "{content} mark meets the body. body {body:?} mark {mark:?}"
        );
        assert!(plan
            .stitches
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .all(|stitch| stitch.from.distance(stitch.to) < 8.0));
    }
}
