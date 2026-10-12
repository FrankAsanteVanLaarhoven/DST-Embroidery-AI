//! Explicit horizontal lines for one text object.
//!
//! Line breaks are derived from LF and CRLF. That step is not NFC.
//! Font normalization still runs per derived line on the existing version path.
//! The entered string is kept. Alignment uses each line's advance, not its
//! visible bounds. A later geometry check measures the sewn stitches.

use crate::error::PlanError;
use crate::lettering::{
    advance_mm, append_meta, glyph, letter_gap, pin_font, push_trace, sew_text, stopped,
    LineLayout, TextRequest, FONT_BASELINE_UNITS, LINE_LAYOUT_POLICY, MAX_COMPONENTS, MAX_LINES,
    MAX_TEXT_CHARS,
};
use crate::lettering_v2::{derive_layout, pin_version_two, strokes_of, FONT_VERSION_2};
use crate::model::{StitchKind, StitchPlan};

const MAX_BLOCK_MM: f64 = 10_000.0;
const MAX_BASELINE_MM: f64 = 10_000.0;
const MAX_GAP_MM: f64 = 1_000.0;

#[derive(Clone, Copy)]
struct Placed {
    index: usize,
    advance: f64,
    dx: f64,
    baseline: f64,
    stitch_start: usize,
    stitch_end: usize,
    blank: bool,
}

pub(crate) fn validate_layout(object_id: u64, layout: &LineLayout) -> Result<(), PlanError> {
    if layout.policy != LINE_LAYOUT_POLICY {
        return Err(stopped(format!(
            "Object {object_id} uses layout policy {}. This engine applies {LINE_LAYOUT_POLICY}. The font version and the document version were not changed.",
            layout.policy
        )));
    }
    if !matches!(layout.alignment.as_str(), "left" | "centre" | "right") {
        return Err(stopped(format!(
            "Object {object_id} has alignment {}. Use left, centre, or right. The text was not wrapped or justified.",
            layout.alignment
        )));
    }
    if !layout.block_width_mm.is_finite()
        || layout.block_width_mm <= 0.0
        || layout.block_width_mm > MAX_BLOCK_MM
    {
        return Err(stopped(format!(
            "Object {object_id} needs a block width greater than 0 mm and at most {MAX_BLOCK_MM} mm. Text is not shrunk or wrapped to fit."
        )));
    }
    if !layout.first_baseline_mm.is_finite() || layout.first_baseline_mm.abs() > MAX_BASELINE_MM {
        return Err(stopped(format!(
            "Object {object_id} needs a first baseline from -{MAX_BASELINE_MM} mm to {MAX_BASELINE_MM} mm."
        )));
    }
    if !layout.baseline_spacing_mm.is_finite() || layout.baseline_spacing_mm.abs() > MAX_GAP_MM {
        return Err(stopped(format!(
            "Object {object_id} needs a baseline spacing from -{MAX_GAP_MM} mm to {MAX_GAP_MM} mm. Increase it when the software clearance check fails. This is not a thread-collision measurement."
        )));
    }
    Ok(())
}

/// LF and CRLF become line breaks. The returned lines do not contain those breaks.
/// A trailing CR that is not part of CRLF is rejected. The caller keeps `content`.
pub(crate) fn derive_lines(content: &str) -> Result<Vec<String>, PlanError> {
    if content.is_empty() {
        return Err(stopped(
            "The text object is empty. Enter 1 to 4 lines. The text was not trimmed.".to_string(),
        ));
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = content.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\n' {
            lines.push(std::mem::take(&mut current));
            continue;
        }
        if character == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
                lines.push(std::mem::take(&mut current));
                continue;
            }
            return Err(stopped(
                "The text contains a carriage return (U+000D) that is not part of CRLF. Horizontal layout treats LF and CRLF as line breaks. Other controls are rejected. The text was left unchanged.".to_string(),
            ));
        }
        if character.is_control() {
            return Err(stopped(format!(
                "The text contains a control character (U+{:04X}). Horizontal layout treats LF and CRLF as line breaks. Other controls are rejected. The text was left unchanged.",
                character as u32
            )));
        }
        current.push(character);
    }
    lines.push(current);
    if lines.is_empty() || lines.len() > MAX_LINES {
        return Err(stopped(format!(
            "The text has {} lines. Horizontal layout allows 1 to {MAX_LINES} lines. Remove a line break and generate again. The text was not wrapped.",
            lines.len()
        )));
    }
    Ok(lines)
}

pub(crate) fn sew(plan: &mut StitchPlan, request: &TextRequest<'_>) -> Result<(), PlanError> {
    let Some(layout) = request.layout else {
        return Err(stopped(
            "Horizontal layout was requested without a layout policy. The plan was not changed."
                .to_string(),
        ));
    };
    validate_layout(request.object_id, layout)?;
    if request.font_version == FONT_VERSION_2 {
        pin_version_two(request)?;
    } else {
        pin_font(request)?;
    }
    check_whole_budget(request)?;
    let lines = derive_lines(request.content)?;
    let scale = request.cap_height_mm / crate::lettering::CAP_UNITS;
    let mut prepared: Vec<(String, bool)> = Vec::with_capacity(lines.len());
    let mut components = 0usize;
    let mut sewing_lines = 0usize;
    for (index, line) in lines.iter().enumerate() {
        let (layout_text, sew_this) = prepare_line(request, index, line)?;
        if sew_this {
            sewing_lines += 1;
            components += component_count(request, &layout_text);
        }
        if components > MAX_COMPONENTS {
            return Err(stopped(format!(
                "Object {} expands to more than {MAX_COMPONENTS} glyph components. Shorten the text and generate again.",
                request.object_id
            )));
        }
        prepared.push((layout_text, sew_this));
    }
    if sewing_lines == 0 {
        return Err(stopped(format!(
            "Object {} has no sewing characters. A blank line only uses one baseline interval. Enter text that includes a letter or digit, then generate again.",
            request.object_id
        )));
    }

    let mut placed = Vec::with_capacity(prepared.len());
    for (index, ((layout_text, sew_this), line)) in prepared.iter().zip(lines.iter()).enumerate() {
        let characters: Vec<char> = layout_text.chars().collect();
        let advance = line_advance(&characters, scale, request)?;
        let dx = delta_x(layout.alignment.as_str(), layout.block_width_mm, advance);
        let baseline = layout.first_baseline_mm + index as f64 * layout.baseline_spacing_mm;
        let stitch_start = plan.stitches.len();
        if *sew_this {
            let line_request = TextRequest {
                content: line,
                origin_x_mm: request.origin_x_mm + dx,
                origin_y_mm: baseline - FONT_BASELINE_UNITS * scale,
                layout: None,
                ..*request
            };
            sew_text(plan, &line_request)?;
        } else if !line.is_empty() {
            record_spaces(plan, request, &characters);
        }
        placed.push(Placed {
            index,
            advance,
            dx,
            baseline,
            stitch_start,
            stitch_end: plan.stitches.len(),
            blank: line.is_empty(),
        });
    }
    reject_overlap(request.object_id, plan, &placed)?;
    write_placement(plan, request, layout, &placed);
    Ok(())
}

fn check_whole_budget(request: &TextRequest<'_>) -> Result<(), PlanError> {
    if request.font_version == FONT_VERSION_2 {
        let probe = TextRequest {
            content: request.content,
            layout: None,
            ..*request
        };
        // Budget checks live at the start of derive_layout. A line break is not
        // a repertoire character, so stop once the length checks have passed.
        return match derive_layout(&probe) {
            Ok(_) => Ok(()),
            Err(error) => {
                let message = error.message();
                if message.contains("bytes")
                    || message.contains("code points")
                    || message.contains("grapheme")
                    || message.contains("combining")
                {
                    Err(error)
                } else {
                    Ok(())
                }
            }
        };
    }
    let count = request.content.chars().count();
    if count > MAX_TEXT_CHARS {
        return Err(stopped(format!(
            "Object {} has {count} characters. This lettering step allows at most {MAX_TEXT_CHARS}. Shorten the text and generate again.",
            request.object_id
        )));
    }
    Ok(())
}

fn prepare_line(
    request: &TextRequest<'_>,
    index: usize,
    line: &str,
) -> Result<(String, bool), PlanError> {
    if line.is_empty() {
        return Ok((String::new(), false));
    }
    if request.font_version == FONT_VERSION_2 {
        let probe = TextRequest {
            content: line,
            layout: None,
            ..*request
        };
        let layout_text = derive_layout(&probe).map_err(|error| {
            stopped(format!(
                "Line {} was not sewn. {}",
                index + 1,
                error
                    .message()
                    .trim_end_matches(" The plan was not changed.")
            ))
        })?;
        let sew_this = layout_text.chars().any(|character| character != ' ');
        return Ok((layout_text, sew_this));
    }
    for (position, character) in line.chars().enumerate() {
        if glyph(character).is_none() {
            return Err(stopped(format!(
                "Object {} line {} contains {character} (U+{:04X}) at position {}. EmbroideryOS Block 01 does not sew that character. The text was left unchanged. Use A–Z, 0–9, space, hyphen, or a straight apostrophe, then generate again.",
                request.object_id,
                index + 1,
                character as u32,
                position + 1
            )));
        }
    }
    let sew_this = line.chars().any(|character| character != ' ');
    Ok((line.to_string(), sew_this))
}

fn component_count(request: &TextRequest<'_>, layout_text: &str) -> usize {
    layout_text
        .chars()
        .filter(|character| *character != ' ')
        .map(|character| {
            if request.font_version == FONT_VERSION_2 {
                strokes_of(character)
                    .map(|(_, strokes)| strokes.len())
                    .unwrap_or(0)
            } else {
                glyph(character).map(|item| item.strokes.len()).unwrap_or(0)
            }
        })
        .sum()
}

fn line_advance(
    characters: &[char],
    scale: f64,
    request: &TextRequest<'_>,
) -> Result<f64, PlanError> {
    let mut pen = 0.0;
    for (index, character) in characters.iter().copied().enumerate() {
        let advance = if request.font_version == FONT_VERSION_2 {
            strokes_of(character).map(|(advance, _)| advance)
        } else {
            glyph(character).map(|item| item.advance)
        };
        let Some(advance) = advance else {
            return Err(stopped(format!(
                "Object {} could not measure {character} (U+{:04X}). The line was not replaced.",
                request.object_id, character as u32
            )));
        };
        pen += advance_mm(character, advance, scale, request);
        if letter_gap(index, characters) {
            pen += request.letter_spacing_mm;
        }
    }
    Ok(pen)
}

fn delta_x(alignment: &str, block_width: f64, advance: f64) -> f64 {
    match alignment {
        "centre" => (block_width - advance) / 2.0,
        "right" => block_width - advance,
        _ => 0.0,
    }
}

fn record_spaces(plan: &mut StitchPlan, request: &TextRequest<'_>, characters: &[char]) {
    let mut trace = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        push_trace(
            &mut trace,
            &format!(
                "object {} glyph {index} U+{:04X} space advance {:.3}",
                request.object_id, character as u32, request.word_spacing_mm
            ),
        );
    }
    if !trace.is_empty() {
        append_meta(plan, "lettering_trace", &trace);
    }
}

fn reject_overlap(object_id: u64, plan: &StitchPlan, placed: &[Placed]) -> Result<(), PlanError> {
    let mut bands: Vec<(usize, f64, f64)> = Vec::new();
    for place in placed {
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        let mut saw = false;
        for stitch in &plan.stitches[place.stitch_start..place.stitch_end] {
            if !stitch.kind.is_sewing() {
                continue;
            }
            for point in [stitch.from, stitch.to] {
                saw = true;
                min_y = min_y.min(point.y);
                max_y = max_y.max(point.y);
            }
        }
        if saw {
            bands.push((place.index, min_y, max_y));
        }
    }
    for left in 0..bands.len() {
        for right in (left + 1)..bands.len() {
            let (a_index, a0, a1) = bands[left];
            let (b_index, b0, b1) = bands[right];
            let overlap = a1.min(b1) - a0.max(b0);
            if overlap > 1e-6 {
                return Err(stopped(format!(
                    "Object {object_id} lines {} and {} overlap in the software clearance check. Sewing extents are {a0:.2}–{a1:.2} mm and {b0:.2}–{b1:.2} mm on Y. Increase the baseline spacing. This is not a thread-collision or legibility measurement.",
                    a_index + 1,
                    b_index + 1
                )));
            }
        }
    }
    for pair in placed.windows(2) {
        let previous = &pair[0];
        let next = &pair[1];
        if previous.stitch_end == previous.stitch_start || next.stitch_end == next.stitch_start {
            continue;
        }
        let bridge = plan.stitches[previous.stitch_start..next.stitch_end]
            .iter()
            .any(|stitch| {
                stitch.kind != StitchKind::Jump
                    && stitch.kind.is_sewing()
                    && crosses(stitch.from.y, stitch.to.y, previous, next, plan)
            });
        if bridge {
            return Err(stopped(format!(
                "Object {object_id} would sew a bridge between lines {} and {}. Lines stay separate. Travel between them is a jump, not a stitch.",
                previous.index + 1,
                next.index + 1
            )));
        }
    }
    Ok(())
}

fn crosses(from_y: f64, to_y: f64, previous: &Placed, next: &Placed, plan: &StitchPlan) -> bool {
    let (a0, a1) = band_y(plan, previous);
    let (b0, b1) = band_y(plan, next);
    let from_previous =
        (a0..=a1).contains(&from_y) || (from_y - a0).abs() < 1e-6 || (from_y - a1).abs() < 1e-6;
    let to_next = (b0..=b1).contains(&to_y) || (to_y - b0).abs() < 1e-6 || (to_y - b1).abs() < 1e-6;
    let from_next = (b0..=b1).contains(&from_y);
    let to_previous = (a0..=a1).contains(&to_y);
    (from_previous && to_next) || (from_next && to_previous)
}

fn band_y(plan: &StitchPlan, place: &Placed) -> (f64, f64) {
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for stitch in &plan.stitches[place.stitch_start..place.stitch_end] {
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

fn write_placement(
    plan: &mut StitchPlan,
    request: &TextRequest<'_>,
    layout: &LineLayout,
    placed: &[Placed],
) {
    plan.metadata.insert(
        "lettering_text".to_string(),
        format!("object {} {}", request.object_id, request.content),
    );
    let mut lines = format!("policy {}\nbreaks lf-crlf", layout.policy);
    let mut travel = String::new();
    for place in placed {
        let kind = if place.blank { " blank" } else { "" };
        lines.push_str(&format!(
            "\nline {}{kind} advance {:.6} dx {:.6} baseline {:.6} alignment {} stitches {}-{}",
            place.index,
            place.advance,
            place.dx,
            place.baseline,
            layout.alignment,
            place.stitch_start,
            place.stitch_end
        ));
    }
    for pair in placed.windows(2) {
        if pair[0].stitch_end == pair[0].stitch_start || pair[1].stitch_end == pair[1].stitch_start
        {
            continue;
        }
        push_trace(
            &mut travel,
            &format!(
                "line-travel {} {} jump connectors-only",
                pair[0].index, pair[1].index
            ),
        );
    }
    append_meta(plan, "lettering_lines", &lines);
    if !travel.is_empty() {
        append_meta(plan, "lettering_trace", &travel);
    }
    plan.metadata.insert(
        "lettering_summary".to_string(),
        format!(
            "EmbroideryOS Block 01 version {} object {}: {} lines, policy {}. Cap height {:.3} mm is a software size, not the full line height and not a measured sewing limit.",
            request.font_version,
            request.object_id,
            placed.len(),
            layout.policy,
            request.cap_height_mm
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::{generate_running, RunningConfig};
    use crate::lettering::{font_content_sha256, generate_text, FONT_ID, FONT_VERSION};
    use crate::lettering_v2::{font_v2_content_sha256, FONT_VERSION_2};
    use crate::model::{Point, StitchKind, StitchPlan};

    fn layout(alignment: &str, width: f64, baseline: f64, spacing: f64) -> LineLayout {
        LineLayout {
            policy: LINE_LAYOUT_POLICY.to_string(),
            alignment: alignment.to_string(),
            block_width_mm: width,
            first_baseline_mm: baseline,
            baseline_spacing_mm: spacing,
        }
    }

    fn request<'a>(
        content: &'a str,
        version: &'a str,
        hash: &'a str,
        item: &'a LineLayout,
        cap: f64,
    ) -> TextRequest<'a> {
        TextRequest {
            object_id: 7,
            content,
            font_id: FONT_ID,
            font_version: version,
            font_sha256: hash,
            origin_x_mm: 10.0,
            origin_y_mm: 12.0,
            cap_height_mm: cap,
            letter_spacing_mm: 1.0,
            word_spacing_mm: 4.0,
            density_mm: 1.5,
            column_width_mm: 1.6,
            pull_compensation_mm: 0.0,
            max_stitch_mm: 3.0,
            centre_run_mm: None,
            edge_run_mm: None,
            color_index: 1,
            min_x: 0.0,
            min_y: 0.0,
            max_x: 300.0,
            max_y: 200.0,
            layout: Some(item),
        }
    }

    fn sew_placed(content: &str, alignment: &str, spacing: f64) -> StitchPlan {
        let hash = font_content_sha256();
        let item = layout(alignment, 40.0, 30.0, spacing);
        let call = request(content, FONT_VERSION, &hash, &item, 10.0);
        let mut plan = StitchPlan::new("lines");
        generate_text(&mut plan, &call)
            .unwrap_or_else(|error| panic!("{content} {alignment}: {error}"));
        plan
    }

    fn rows(plan: &StitchPlan) -> Vec<(usize, f64, f64, f64, usize, usize)> {
        plan.metadata["lettering_lines"]
            .lines()
            .filter(|line| line.starts_with("line "))
            .map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let index = parts[1].parse::<usize>().expect("line index");
                let mut advance = 0.0;
                let mut dx = 0.0;
                let mut baseline = 0.0;
                let mut start = 0usize;
                let mut end = 0usize;
                for pair in parts.windows(2) {
                    match pair[0] {
                        "advance" => advance = pair[1].parse().unwrap(),
                        "dx" => dx = pair[1].parse().unwrap(),
                        "baseline" => baseline = pair[1].parse().unwrap(),
                        "stitches" => {
                            let (left, right) = pair[1].split_once('-').unwrap();
                            start = left.parse().unwrap();
                            end = right.parse().unwrap();
                        }
                        _ => {}
                    }
                }
                (index, advance, dx, baseline, start, end)
            })
            .collect()
    }

    #[test]
    fn twelve_layouts_assert_advance_offset_and_baseline() {
        let samples = ["AB", "AB\nA", "AB\nA\nABC", "AB\n\nA\nABC"];
        let scale = 10.0 / 100.0;
        let letter = 64.0 * scale;
        let advances = [
            vec![letter + 1.0 + letter],
            vec![letter + 1.0 + letter, letter],
            vec![letter + 1.0 + letter, letter, letter * 3.0 + 2.0],
            vec![letter + 1.0 + letter, 0.0, letter, letter * 3.0 + 2.0],
        ];
        assert_eq!(samples.len() * 3, 12);
        for (sample_index, text) in samples.iter().enumerate() {
            for alignment in ["left", "centre", "right"] {
                let plan = sew_placed(text, alignment, 18.0);
                let parsed = rows(&plan);
                assert_eq!(
                    parsed.len(),
                    advances[sample_index].len(),
                    "{text} {alignment}"
                );
                assert_eq!(plan.metadata["lettering_text"], format!("object 7 {text}"));
                let again = sew_placed(text, alignment, 18.0);
                assert_eq!(plan.stitches, again.stitches);
                for (row_index, (index, advance, dx, baseline, start, end)) in
                    parsed.iter().enumerate()
                {
                    let expected = advances[sample_index][row_index];
                    assert_eq!(*index, row_index);
                    assert!((advance - expected).abs() < 1e-6, "{advance} {expected}");
                    let expected_dx = match alignment {
                        "centre" => (40.0 - expected) / 2.0,
                        "right" => 40.0 - expected,
                        _ => 0.0,
                    };
                    assert!(
                        (dx - expected_dx).abs() < 1e-6,
                        "{alignment} {dx} {expected_dx}"
                    );
                    assert!((baseline - (30.0 + row_index as f64 * 18.0)).abs() < 1e-9);
                    if expected == 0.0 {
                        assert_eq!(start, end);
                    } else {
                        assert!(end > start);
                        let piece = text.split('\n').nth(row_index).expect("line");
                        let reference = historical_line(
                            piece,
                            10.0 + expected_dx,
                            *baseline - FONT_BASELINE_UNITS * scale,
                        );
                        assert_eq!(
                            sewing_of(&plan, *start, *end),
                            sewing_of(&reference, 0, reference.stitches.len()),
                            "{text} {alignment} line {row_index}"
                        );
                    }
                }
            }
        }
        let left = sew_placed("AB\nA", "left", 18.0);
        let right = sew_placed("AB\nA", "right", 18.0);
        let left_rows = rows(&left);
        let right_rows = rows(&right);
        for (left_row, right_row) in left_rows.iter().zip(right_rows.iter()) {
            let shift = min_sewing_x(&right, right_row.4, right_row.5)
                - min_sewing_x(&left, left_row.4, left_row.5);
            assert!((shift - (right_row.2 - left_row.2)).abs() < 1e-4);
            let y_shift = min_sewing_y(&right, right_row.4, right_row.5)
                - min_sewing_y(&left, left_row.4, left_row.5);
            assert!(y_shift.abs() < 1e-4);
        }
        let stacked = sew_placed("A\nA", "left", 18.0);
        let stacked_rows = rows(&stacked);
        let y_gap = min_sewing_y(&stacked, stacked_rows[1].4, stacked_rows[1].5)
            - min_sewing_y(&stacked, stacked_rows[0].4, stacked_rows[0].5);
        assert!((y_gap - 18.0).abs() < 1e-4, "{y_gap}");
    }

    fn sewing_of(
        plan: &StitchPlan,
        start: usize,
        end: usize,
    ) -> Vec<(StitchKind, f64, f64, f64, f64)> {
        plan.stitches[start..end]
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .map(|stitch| {
                (
                    stitch.kind,
                    stitch.from.x,
                    stitch.from.y,
                    stitch.to.x,
                    stitch.to.y,
                )
            })
            .collect()
    }

    fn historical_line(content: &str, origin_x: f64, origin_y: f64) -> StitchPlan {
        let hash = font_content_sha256();
        let call = TextRequest {
            object_id: 7,
            content,
            font_id: FONT_ID,
            font_version: FONT_VERSION,
            font_sha256: &hash,
            origin_x_mm: origin_x,
            origin_y_mm: origin_y,
            cap_height_mm: 10.0,
            letter_spacing_mm: 1.0,
            word_spacing_mm: 4.0,
            density_mm: 1.5,
            column_width_mm: 1.6,
            pull_compensation_mm: 0.0,
            max_stitch_mm: 3.0,
            centre_run_mm: None,
            edge_run_mm: None,
            color_index: 1,
            min_x: 0.0,
            min_y: 0.0,
            max_x: 300.0,
            max_y: 200.0,
            layout: None,
        };
        let mut plan = StitchPlan::new("reference");
        generate_text(&mut plan, &call).expect(content);
        plan
    }

    fn min_sewing_x(plan: &StitchPlan, start: usize, end: usize) -> f64 {
        plan.stitches[start..end]
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .flat_map(|stitch| [stitch.from.x, stitch.to.x])
            .fold(f64::MAX, f64::min)
    }

    fn min_sewing_y(plan: &StitchPlan, start: usize, end: usize) -> f64 {
        plan.stitches[start..end]
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .flat_map(|stitch| [stitch.from.y, stitch.to.y])
            .fold(f64::MAX, f64::min)
    }

    #[test]
    fn historical_single_line_matches_left_aligned_one_line() {
        let hash = font_content_sha256();
        let plain = TextRequest {
            object_id: 7,
            content: "ALPINE 08",
            font_id: FONT_ID,
            font_version: FONT_VERSION,
            font_sha256: &hash,
            origin_x_mm: 8.0,
            origin_y_mm: 8.0,
            cap_height_mm: 12.0,
            letter_spacing_mm: 1.0,
            word_spacing_mm: 4.0,
            density_mm: 1.5,
            column_width_mm: 1.92,
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
        };
        let mut historical = StitchPlan::new("historical");
        generate_text(&mut historical, &plain).unwrap();
        let item = layout("left", 80.0, 8.0 + 92.0 * 12.0 / 100.0, 20.0);
        let lined = TextRequest {
            layout: Some(&item),
            ..plain
        };
        let mut multiline = StitchPlan::new("lined");
        generate_text(&mut multiline, &lined).unwrap();
        assert_eq!(historical.stitches, multiline.stitches);
        assert!(!historical.metadata.contains_key("lettering_lines"));
        assert!(multiline.metadata["lettering_lines"].contains("alignment left"));
        assert_eq!(multiline.metadata["lettering_text"], "object 7 ALPINE 08");
    }

    #[test]
    fn newline_without_layout_stays_on_the_historical_path() {
        let hash = font_content_sha256();
        let plain = TextRequest {
            object_id: 7,
            content: "A\nB",
            font_id: FONT_ID,
            font_version: FONT_VERSION,
            font_sha256: &hash,
            origin_x_mm: 8.0,
            origin_y_mm: 8.0,
            cap_height_mm: 12.0,
            letter_spacing_mm: 1.0,
            word_spacing_mm: 4.0,
            density_mm: 1.5,
            column_width_mm: 1.92,
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
        };
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &plain)
            .unwrap_err()
            .message()
            .to_string();
        assert!(message.contains("U+000A"));
        assert!(!message.contains(LINE_LAYOUT_POLICY));
        assert_eq!(plan.stitches, before.stitches);
        assert_eq!(plan.metadata, before.metadata);
    }

    #[test]
    fn blank_lines_spaces_and_rejected_controls_keep_the_source() {
        let hash = font_content_sha256();
        let item = layout("centre", 40.0, 30.0, 18.0);
        let spaced = request("A\n  \nB", FONT_VERSION, &hash, &item, 10.0);
        let mut plan = StitchPlan::new("spaces");
        generate_text(&mut plan, &spaced).unwrap();
        assert_eq!(plan.metadata["lettering_text"], "object 7 A\n  \nB");
        let parsed = rows(&plan);
        assert_eq!(parsed.len(), 3);
        assert!((parsed[1].1 - 8.0).abs() < 1e-6);
        assert_eq!(parsed[1].4, parsed[1].5);
        assert!(plan.metadata["lettering_trace"].contains("U+0020"));
        let crlf_item = layout("left", 40.0, 30.0, 18.0);
        let crlf = request("A\r\nB", FONT_VERSION, &hash, &crlf_item, 10.0);
        let mut crlf_plan = StitchPlan::new("crlf");
        generate_text(&mut crlf_plan, &crlf).unwrap();
        assert_eq!(crlf_plan.metadata["lettering_text"], "object 7 A\r\nB");
        assert_eq!(rows(&crlf_plan).len(), 2);

        let mut kept = occupied();
        let before = kept.clone();
        for content in ["", "\n\n", "   ", "A\nB\nC\nD\nE", "A\tB", "A\rB"] {
            let call = request(content, FONT_VERSION, &hash, &item, 10.0);
            let message = generate_text(&mut kept, &call)
                .unwrap_err()
                .message()
                .to_string();
            assert!(
                message.contains("The plan was not changed."),
                "{content}: {message}"
            );
            assert_eq!(kept.stitches, before.stitches);
            assert_eq!(kept.metadata, before.metadata);
        }
    }

    #[test]
    fn last_line_overflow_restores_the_plan_and_does_not_bridge() {
        let hash = font_content_sha256();
        let item = layout("left", 30.0, 20.0, 30.0);
        let mut call = request("A\nB", FONT_VERSION, &hash, &item, 10.0);
        call.max_y = 36.0;
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &call)
            .unwrap_err()
            .message()
            .to_string();
        assert!(
            message.contains("outside") || message.contains("hoop"),
            "{message}"
        );
        assert!(message.contains("The plan was not changed."));
        assert_eq!(plan.stitches, before.stitches);
        assert_eq!(plan.metadata, before.metadata);

        let wide = layout("left", 40.0, 30.0, 18.0);
        let ok = request("A\nB", FONT_VERSION, &hash, &wide, 10.0);
        let mut sewn = StitchPlan::new("bridge");
        generate_text(&mut sewn, &ok).unwrap();
        let parsed = rows(&sewn);
        assert!(sewn.metadata["lettering_trace"].contains("line-travel 0 1 jump"));
        let bridge = sewn.stitches.iter().any(|stitch| {
            stitch.kind != StitchKind::Jump
                && stitch.kind.is_sewing()
                && endpoint_in(&sewn, stitch.from.y, parsed[0].4, parsed[0].5)
                && endpoint_in(&sewn, stitch.to.y, parsed[1].4, parsed[1].5)
        });
        assert!(!bridge);
        assert!(sewn
            .stitches
            .iter()
            .any(|stitch| stitch.kind == StitchKind::Jump));
    }

    fn endpoint_in(plan: &StitchPlan, y: f64, start: usize, end: usize) -> bool {
        let low = min_sewing_y(plan, start, end) - 1e-6;
        let high = plan.stitches[start..end]
            .iter()
            .filter(|stitch| stitch.kind.is_sewing())
            .flat_map(|stitch| [stitch.from.y, stitch.to.y])
            .fold(f64::MIN, f64::max)
            + 1e-6;
        (low..=high).contains(&y)
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
    fn clearance_rejects_descender_against_accent_without_claiming_collision() {
        let hash = font_v2_content_sha256();
        let tight = layout("left", 80.0, 40.0, 8.0);
        let text = "Jürgen\nÉquipe";
        let call = request(text, FONT_VERSION_2, &hash, &tight, 12.0);
        let mut plan = occupied();
        let before = plan.clone();
        let message = generate_text(&mut plan, &call)
            .unwrap_err()
            .message()
            .to_string();
        assert!(message.contains("software clearance"));
        assert!(message.contains("not a thread-collision"));
        assert!(
            !message.contains("legibility measurement")
                || message.contains("not a thread-collision or legibility")
        );
        assert_eq!(plan.stitches, before.stitches);
        let open = layout("left", 80.0, 40.0, 28.0);
        let passed = request(text, FONT_VERSION_2, &hash, &open, 12.0);
        let mut sewn = StitchPlan::new("clear");
        generate_text(&mut sewn, &passed).unwrap();
        assert_eq!(sewn.metadata["lettering_text"], format!("object 7 {text}"));
        assert!(sewn.metadata["lettering_layout"].contains("NFC"));
    }

    #[test]
    fn canonical_lines_share_geometry_and_apostrophes_do_not() {
        let hash = font_v2_content_sha256();
        let item = layout("centre", 90.0, 36.0, 22.0);
        let composed = "Café\nAlpine 08";
        let decomposed = "Cafe\u{0301}\nAlpine 08";
        let left = request(composed, FONT_VERSION_2, &hash, &item, 8.0);
        let right = request(decomposed, FONT_VERSION_2, &hash, &item, 8.0);
        let mut a = StitchPlan::new("cafe");
        let mut b = StitchPlan::new("cafe-nfd");
        generate_text(&mut a, &left).unwrap();
        generate_text(&mut b, &right).unwrap();
        assert_eq!(a.stitches, b.stitches);
        assert_eq!(a.metadata["lettering_text"], format!("object 7 {composed}"));
        assert_eq!(
            b.metadata["lettering_text"],
            format!("object 7 {decomposed}")
        );
        assert_ne!(a.metadata["lettering_text"], b.metadata["lettering_text"]);
        let straight = request("O'Neil\nA", FONT_VERSION_2, &hash, &item, 8.0);
        let curly = request("O\u{2019}Neil\nA", FONT_VERSION_2, &hash, &item, 8.0);
        let mut c = StitchPlan::new("straight");
        let mut d = StitchPlan::new("curly");
        generate_text(&mut c, &straight).unwrap();
        generate_text(&mut d, &curly).unwrap();
        assert_ne!(c.stitches, d.stitches);
        assert!(c.metadata["lettering_text"].contains("O'Neil\nA"));
        assert!(d.metadata["lettering_text"].contains("O\u{2019}Neil\nA"));
    }

    #[test]
    fn four_underlay_modes_keep_top_satin_distinct_from_connectors() {
        let hash = font_content_sha256();
        let item = layout("left", 40.0, 30.0, 18.0);
        let modes: [(&str, Option<f64>, Option<f64>); 4] = [
            ("none", None, None),
            ("centre", Some(3.0), None),
            ("edge", None, Some(3.0)),
            ("both", Some(3.0), Some(3.0)),
        ];
        for (name, centre, edge) in modes {
            let mut call = request("A\nB", FONT_VERSION, &hash, &item, 10.0);
            call.centre_run_mm = centre;
            call.edge_run_mm = edge;
            let mut plan = StitchPlan::new(name);
            generate_text(&mut plan, &call).unwrap_or_else(|error| panic!("{name}: {error}"));
            let satin = plan
                .stitches
                .iter()
                .filter(|stitch| stitch.kind == StitchKind::Satin)
                .count();
            let running = plan
                .stitches
                .iter()
                .filter(|stitch| stitch.kind == StitchKind::Running)
                .count();
            let jumps = plan
                .stitches
                .iter()
                .filter(|stitch| stitch.kind == StitchKind::Jump)
                .count();
            assert!(satin > 0, "{name}");
            assert!(jumps > 0, "{name}");
            match name {
                "none" => assert_eq!(running, 0),
                "centre" | "edge" => assert!(running > 0),
                "both" => assert!(running > running_of("centre", &hash, &item)),
                _ => unreachable!(),
            }
        }
    }

    fn running_of(name: &str, hash: &str, item: &LineLayout) -> usize {
        let mut call = request("A\nB", FONT_VERSION, hash, item, 10.0);
        if name == "centre" {
            call.centre_run_mm = Some(3.0);
        }
        let mut plan = StitchPlan::new("count");
        generate_text(&mut plan, &call).unwrap();
        plan.stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Running)
            .count()
    }
}
