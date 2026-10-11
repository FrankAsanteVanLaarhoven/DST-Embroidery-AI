//! Normalised execution trace decoded from DST bytes.

use serde::Serialize;

use embroidery_core::{sha256_hex, units_to_mm};
use embroidery_dst::{decode_record, ExportLink, RecordCommand};

use crate::{
    DEMO_COLOR_CHANGE_SECONDS, DEMO_JUMP_MM_PER_SEC, DEMO_STITCHES_PER_MINUTE, DEMO_TRIM_SECONDS,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceWarning {
    pub code: String,
    pub origin: String,
    pub observed: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceEvent {
    pub index: u32,
    pub kind: String,
    pub design_x_mm: f64,
    pub design_y_mm: f64,
    pub from_x_mm: f64,
    pub from_y_mm: f64,
    pub units_x: i32,
    pub units_y: i32,
    pub color_index: u32,
    pub object_id: Option<u64>,
    pub plan_stitch_index: Option<u32>,
    pub file_record_index: u32,
    pub file_record_count: u32,
    pub penetrates: bool,
    pub hoop_from_x_mm: f64,
    pub hoop_from_y_mm: f64,
    pub hoop_x_mm: f64,
    pub hoop_y_mm: f64,
    pub warnings: Vec<TraceWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionTrace {
    pub dst_sha256: String,
    pub encoded_record_count: u32,
    pub needle_down_count: u32,
    pub jump_count: u32,
    pub color_change_count: u32,
    pub trim_count: u32,
    pub end_count: u32,
    pub estimated_execution_seconds: f64,
    pub events: Vec<TraceEvent>,
}

struct BodyRecord {
    command: RecordCommand,
    x: i32,
    y: i32,
    from_x: i32,
    from_y: i32,
    link_index: usize,
}

pub fn decode_execution(
    dst: &[u8],
    links: &[ExportLink],
    hoop_width_mm: f64,
    hoop_height_mm: f64,
) -> Result<ExecutionTrace, String> {
    if dst.len() < 512 || (dst.len() - 512) % 3 != 0 {
        return Err(
            "The DST payload is not a 512-byte header plus 3-byte records. Export the design again."
                .to_string(),
        );
    }
    let mut decoded = Vec::new();
    let mut saw_end = false;
    for chunk in dst[512..].chunks_exact(3) {
        let mut bytes = [0u8; 3];
        bytes.copy_from_slice(chunk);
        let record = decode_record(bytes);
        if record.command == RecordCommand::End {
            saw_end = true;
            break;
        }
        decoded.push(record);
    }
    if !saw_end {
        return Err(
            "The DST file has no end record. Export the design again before rehearsing it."
                .to_string(),
        );
    }
    if decoded.len() + 1 != (dst.len() - 512) / 3 {
        return Err(
            "DST records continue after the end record. Export the design again before rehearsing it."
                .to_string(),
        );
    }
    if links.len() != decoded.len() {
        return Err(format!(
            "The export sidecar has {} links and the file has {} body records. Export the design again.",
            links.len(),
            decoded.len()
        ));
    }

    let mut pen = (0i32, 0i32);
    let mut body = Vec::new();
    for (link_index, record) in decoded.iter().enumerate() {
        let from = pen;
        let next = match record.command {
            RecordCommand::ColorChange | RecordCommand::End => pen,
            RecordCommand::Stitch | RecordCommand::Jump | RecordCommand::SequinMode => {
                (pen.0 + record.dx, pen.1 + record.dy)
            }
        };
        body.push(BodyRecord {
            command: record.command,
            x: next.0,
            y: next.1,
            from_x: from.0,
            from_y: from.1,
            link_index,
        });
        pen = next;
    }

    let mut events = Vec::new();
    let mut color = 0u32;
    let mut index = 0usize;
    {
        let mut emit = Emit {
            events: &mut events,
            links,
            hoop_width_mm,
            hoop_height_mm,
        };
        while index < body.len() {
            if index + 2 < body.len() && is_trim_signature(&body[index..index + 3], links) {
                let held = BodyRecord {
                    command: RecordCommand::Jump,
                    x: body[index].from_x,
                    y: body[index].from_y,
                    from_x: body[index].from_x,
                    from_y: body[index].from_y,
                    link_index: body[index].link_index,
                };
                emit.push("trim", &held, color, 3, false);
                index += 3;
                continue;
            }
            let record = &body[index];
            let (kind, penetrates) = match record.command {
                RecordCommand::Stitch => ("sew", true),
                RecordCommand::Jump | RecordCommand::SequinMode => ("jump", false),
                RecordCommand::ColorChange => ("color_change", false),
                RecordCommand::End => ("end", false),
            };
            if record.command == RecordCommand::ColorChange {
                color += 1;
            }
            emit.push(kind, record, color, 1, penetrates);
            index += 1;
        }
    }
    let end_index = events.len() as u32;
    events.push(end_event(body.len() as u32, pen, end_index));

    let encoded_record_count = ((dst.len() - 512) / 3) as u32;
    let file_records: u32 = events.iter().map(|event| event.file_record_count).sum();
    if file_records != encoded_record_count {
        return Err(format!(
            "The trace covers {file_records} records and the file contains {encoded_record_count}. Export the design again."
        ));
    }

    Ok(ExecutionTrace {
        dst_sha256: sha256_hex(dst),
        encoded_record_count,
        needle_down_count: count_kind(&events, "sew"),
        jump_count: count_kind(&events, "jump"),
        color_change_count: count_kind(&events, "color_change"),
        trim_count: count_kind(&events, "trim"),
        end_count: count_kind(&events, "end"),
        estimated_execution_seconds: estimate_seconds(&events),
        events,
    })
}

fn is_trim_signature(records: &[BodyRecord], links: &[ExportLink]) -> bool {
    let same_stitch = links[records[0].link_index].plan_stitch_index
        == links[records[1].link_index].plan_stitch_index
        && links[records[1].link_index].plan_stitch_index
            == links[records[2].link_index].plan_stitch_index;
    same_stitch
        && records
            .iter()
            .all(|record| record.command == RecordCommand::Jump)
        && delta(&records[0]) == (2, 2)
        && delta(&records[1]) == (-4, -4)
        && delta(&records[2]) == (2, 2)
}

fn delta(record: &BodyRecord) -> (i32, i32) {
    (record.x - record.from_x, record.y - record.from_y)
}

struct Emit<'a> {
    events: &'a mut Vec<TraceEvent>,
    links: &'a [ExportLink],
    hoop_width_mm: f64,
    hoop_height_mm: f64,
}

impl Emit<'_> {
    fn push(
        &mut self,
        kind: &str,
        record: &BodyRecord,
        color_index: u32,
        file_record_count: u32,
        penetrates: bool,
    ) {
        let link = &self.links[record.link_index];
        let from_x = units_to_mm(record.from_x);
        let from_y = units_to_mm(record.from_y);
        let x = units_to_mm(record.x);
        let y = units_to_mm(record.y);
        let mut warnings = Vec::new();
        if x < -1e-6 || y < -1e-6 || x > self.hoop_width_mm + 1e-6 || y > self.hoop_height_mm + 1e-6
        {
            warnings.push(TraceWarning {
                code: "hoop_limit".to_string(),
                origin: "kinematic_check".to_string(),
                observed: false,
                message: format!(
                    "Decoded needle {x:.1}, {y:.1} mm is outside the {:.1} by {:.1} mm hoop. This is a kinematic check, not an observed machine fault.",
                    self.hoop_width_mm, self.hoop_height_mm
                ),
            });
        }
        self.events.push(TraceEvent {
            index: self.events.len() as u32,
            kind: kind.to_string(),
            design_x_mm: x,
            design_y_mm: y,
            from_x_mm: from_x,
            from_y_mm: from_y,
            units_x: record.x,
            units_y: record.y,
            color_index,
            object_id: Some(link.object_id),
            plan_stitch_index: Some(link.plan_stitch_index),
            file_record_index: record.link_index as u32,
            file_record_count,
            penetrates,
            hoop_from_x_mm: -from_x,
            hoop_from_y_mm: -from_y,
            hoop_x_mm: -x,
            hoop_y_mm: -y,
            warnings,
        });
    }
}

fn end_event(file_record_index: u32, pen: (i32, i32), index: u32) -> TraceEvent {
    let x = units_to_mm(pen.0);
    let y = units_to_mm(pen.1);
    TraceEvent {
        index,
        kind: "end".to_string(),
        design_x_mm: x,
        design_y_mm: y,
        from_x_mm: x,
        from_y_mm: y,
        units_x: pen.0,
        units_y: pen.1,
        color_index: 0,
        object_id: None,
        plan_stitch_index: None,
        file_record_index,
        file_record_count: 1,
        penetrates: false,
        hoop_from_x_mm: -x,
        hoop_from_y_mm: -y,
        hoop_x_mm: -x,
        hoop_y_mm: -y,
        warnings: Vec::new(),
    }
}

fn count_kind(events: &[TraceEvent], kind: &str) -> u32 {
    events.iter().filter(|event| event.kind == kind).count() as u32
}

pub fn estimate_seconds(events: &[TraceEvent]) -> f64 {
    let sew_seconds = 60.0 / DEMO_STITCHES_PER_MINUTE;
    events.iter().fold(0.0, |total, event| {
        total
            + match event.kind.as_str() {
                "sew" => sew_seconds,
                "jump" => {
                    let dx = event.design_x_mm - event.from_x_mm;
                    let dy = event.design_y_mm - event.from_y_mm;
                    (dx * dx + dy * dy).sqrt() / DEMO_JUMP_MM_PER_SEC
                }
                "color_change" => DEMO_COLOR_CHANGE_SECONDS,
                "trim" => DEMO_TRIM_SECONDS,
                _ => 0.0,
            }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use embroidery_core::{generate_running, Point, RunningConfig, Stitch, StitchKind, StitchPlan};
    use embroidery_dst::compile_to_vec_with_links;
    use std::io::Cursor;

    fn compile_line() -> (Vec<u8>, Vec<ExportLink>, StitchPlan) {
        let mut plan = StitchPlan::new("Line");
        generate_running(
            &mut plan,
            7,
            0,
            &[Point::new(0.0, 0.0), Point::new(10.0, 20.0)],
            &RunningConfig {
                max_stitch_length_mm: 5.0,
            },
        )
        .unwrap();
        let (bytes, links) = compile_to_vec_with_links(&plan, 12.1, 1000.0).unwrap();
        (bytes, links, plan)
    }

    #[test]
    fn decoded_needle_points_match_the_file_and_keep_the_object() {
        let (bytes, links, _plan) = compile_line();
        let plain = compile_to_vec_matches(&bytes);
        let trace = decode_execution(&bytes, &links, 120.0, 100.0).unwrap();
        assert_eq!(trace.dst_sha256, sha256_hex(&bytes));
        assert!(trace.needle_down_count > 0);
        assert!(trace
            .events
            .iter()
            .filter(|event| event.kind == "sew")
            .all(|event| { event.object_id == Some(7) && event.warnings.is_empty() }));
        let read = embroidery_dst::read_dst(Cursor::new(bytes)).unwrap();
        let trace_sew: Vec<_> = trace
            .events
            .iter()
            .filter(|event| event.kind == "sew")
            .map(|event| (event.units_x, event.units_y))
            .collect();
        let read_sew: Vec<_> = read
            .stitches
            .iter()
            .filter(|stitch| stitch.kind == StitchKind::Running)
            .map(|stitch| {
                (
                    embroidery_core::mm_to_units(stitch.to.x).unwrap(),
                    embroidery_core::mm_to_units(stitch.to.y).unwrap(),
                )
            })
            .collect();
        assert_eq!(trace_sew, read_sew);
        assert!(plain);
    }

    fn compile_to_vec_matches(bytes: &[u8]) -> bool {
        bytes.starts_with(b"LA:") && bytes.len() >= 515
    }

    #[test]
    fn rehearsal_uses_quantised_units_rather_than_the_source_point() {
        let mut plan = StitchPlan::new("Quant");
        plan.stitches.push(Stitch {
            kind: StitchKind::Running,
            from: Point::new(0.0, 0.0),
            to: Point::new(0.05, 0.0),
            color_index: 0,
            object_id: 4,
        });
        let (bytes, links) = compile_to_vec_with_links(&plan, 12.1, 1000.0).unwrap();
        let trace = decode_execution(&bytes, &links, 50.0, 50.0).unwrap();
        let sew = trace
            .events
            .iter()
            .find(|event| event.kind == "sew")
            .unwrap();
        assert_eq!(sew.units_x, 0);
        assert_eq!(sew.design_x_mm, 0.0);
    }

    #[test]
    fn a_point_outside_the_hoop_is_a_kinematic_check() {
        let mut plan = StitchPlan::new("Outside");
        plan.stitches.push(Stitch {
            kind: StitchKind::Running,
            from: Point::new(0.0, 0.0),
            to: Point::new(10.0, 0.0),
            color_index: 0,
            object_id: 1,
        });
        let (bytes, links) = compile_to_vec_with_links(&plan, 12.1, 1000.0).unwrap();
        let trace = decode_execution(&bytes, &links, 5.0, 5.0).unwrap();
        let warning = trace
            .events
            .iter()
            .find_map(|event| event.warnings.first())
            .unwrap();
        assert_eq!(warning.code, "hoop_limit");
        assert_eq!(warning.origin, "kinematic_check");
        assert!(!warning.observed);
        assert!(warning.message.contains("kinematic check"));
    }

    #[test]
    fn estimated_time_uses_the_declared_rates() {
        let (bytes, links, _) = compile_line();
        let trace = decode_execution(&bytes, &links, 120.0, 100.0).unwrap();
        let again = estimate_seconds(&trace.events);
        assert!((trace.estimated_execution_seconds - again).abs() < 1e-9);
        assert!(trace.estimated_execution_seconds > 0.0);
    }
}
