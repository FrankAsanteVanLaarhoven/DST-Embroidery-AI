use std::io::{self, Read};

use embroidery_core::{units_to_mm, ColorStop, Point, Stitch, StitchKind, StitchPlan, ThreadSpec};

use crate::ternary::{decode_record, DecodedRecord, RecordCommand};

/// Reader options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadOptions {
    /// Collapse the pyembroidery `trim_at=3` jump signature back into one trim.
    ///
    /// The signature is the three jump records `(2, 2), (-4, -4), (2, 2)`.
    /// It is specific, and a real design that sews that exact jog will be read
    /// back as a trim. Set this to false to keep the jumps.
    pub collapse_trim_signature: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            collapse_trim_signature: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyCommand {
    Stitch,
    Jump,
    ColorChange,
    SequinMode,
    Trim,
}

struct BodyRecord {
    dx: i32,
    dy: i32,
    command: BodyCommand,
}

pub fn read_dst<R: Read>(reader: R) -> io::Result<StitchPlan> {
    read_dst_with(reader, &ReadOptions::default())
}

pub fn read_dst_with<R: Read>(mut reader: R, options: &ReadOptions) -> io::Result<StitchPlan> {
    let mut header = [0u8; 512];
    reader.read_exact(&mut header).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "DST file is shorter than the 512-byte header. Export the design again.",
            )
        } else {
            error
        }
    })?;

    let mut plan = StitchPlan::new(String::new());
    apply_header(&mut plan, &header);

    let mut decoded = Vec::new();
    let mut saw_end = false;
    while let Some(bytes) = read_record(&mut reader)? {
        let record = decode_record(bytes);
        if record.command == RecordCommand::End {
            saw_end = true;
            break;
        }
        decoded.push(record);
    }

    let file_records = decoded.len() + usize::from(saw_end);
    if let Some(header_count) = plan
        .metadata
        .get("ST")
        .and_then(|value| value.parse::<usize>().ok())
    {
        if header_count != file_records {
            plan.metadata.insert(
                "dst_st_mismatch".into(),
                format!("header ST is {header_count}, file contains {file_records} records"),
            );
        }
    }
    plan.metadata
        .insert("dst_record_count".into(), file_records.to_string());
    if !saw_end {
        plan.metadata.insert(
            "missing_end".into(),
            "true. The file has no end record. Export it again before sewing.".into(),
        );
    }

    let body = if options.collapse_trim_signature {
        collapse_trims(decoded)
    } else {
        decoded.into_iter().map(BodyRecord::from).collect()
    };

    let mut pen = (0i32, 0i32);
    let mut color_index = 0usize;
    let mut sequin_toggles = 0usize;
    for record in body {
        let next = (pen.0 + record.dx, pen.1 + record.dy);
        match record.command {
            BodyCommand::ColorChange => {
                color_index += 1;
                push_stitch(&mut plan, StitchKind::ColorChange, pen, next, color_index);
            }
            BodyCommand::Trim => {
                push_stitch(&mut plan, StitchKind::Trim, pen, pen, color_index);
            }
            BodyCommand::SequinMode => {
                sequin_toggles += 1;
                push_stitch(&mut plan, StitchKind::Jump, pen, next, color_index);
            }
            BodyCommand::Stitch => {
                push_stitch(&mut plan, StitchKind::Running, pen, next, color_index);
            }
            BodyCommand::Jump => {
                push_stitch(&mut plan, StitchKind::Jump, pen, next, color_index);
            }
        }
        if record.command != BodyCommand::Trim {
            pen = next;
        }
    }
    if sequin_toggles > 0 {
        plan.metadata
            .insert("sequin_mode_toggles".into(), sequin_toggles.to_string());
    }
    plan.metadata.insert(
        "dst_kind_note".into(),
        "DST does not store satin, fill, or motif. Needle-down records are read back as running stitches.".into(),
    );
    Ok(plan)
}

impl From<DecodedRecord> for BodyRecord {
    fn from(record: DecodedRecord) -> Self {
        let command = match record.command {
            RecordCommand::Stitch => BodyCommand::Stitch,
            RecordCommand::Jump => BodyCommand::Jump,
            RecordCommand::ColorChange => BodyCommand::ColorChange,
            RecordCommand::SequinMode => BodyCommand::SequinMode,
            RecordCommand::End => BodyCommand::Stitch,
        };
        BodyRecord {
            dx: record.dx,
            dy: record.dy,
            command,
        }
    }
}

fn collapse_trims(records: Vec<DecodedRecord>) -> Vec<BodyRecord> {
    let mut out = Vec::with_capacity(records.len());
    let mut index = 0;
    while index < records.len() {
        if index + 2 < records.len()
            && is_jump(&records[index], 2, 2)
            && is_jump(&records[index + 1], -4, -4)
            && is_jump(&records[index + 2], 2, 2)
        {
            out.push(BodyRecord {
                dx: 0,
                dy: 0,
                command: BodyCommand::Trim,
            });
            index += 3;
            continue;
        }
        out.push(BodyRecord::from(records[index]));
        index += 1;
    }
    out
}

fn is_jump(record: &DecodedRecord, dx: i32, dy: i32) -> bool {
    record.command == RecordCommand::Jump && record.dx == dx && record.dy == dy
}

fn push_stitch(
    plan: &mut StitchPlan,
    kind: StitchKind,
    from: (i32, i32),
    to: (i32, i32),
    color_index: usize,
) {
    plan.stitches.push(Stitch {
        kind,
        from: Point::new(units_to_mm(from.0), units_to_mm(from.1)),
        to: Point::new(units_to_mm(to.0), units_to_mm(to.1)),
        color_index,
        object_id: 0,
    });
}

fn read_record<R: Read>(reader: &mut R) -> io::Result<Option<[u8; 3]>> {
    let mut buffer = [0u8; 3];
    let mut filled = 0;
    while filled < 3 {
        match reader.read(&mut buffer[filled..])? {
            0 if filled == 0 => return Ok(None),
            0 => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DST file ends on a partial 3-byte record. The file is truncated; export it again.",
            )),
            count => filled += count,
        }
    }
    Ok(Some(buffer))
}

fn apply_header(plan: &mut StitchPlan, header: &[u8; 512]) {
    let text = String::from_utf8_lossy(header);
    let mut color_index = 0usize;
    for line in text.split(['\r', '\n']) {
        let line = line.trim_matches(|character: char| {
            character == ' ' || character == '\u{1a}' || character == '\0'
        });
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            continue;
        }
        match key {
            "LA" => plan.name = value.to_string(),
            "TC" => {
                if let Some(stop) = parse_thread(color_index, value) {
                    plan.colors.push(stop);
                    color_index += 1;
                }
            }
            other => {
                plan.metadata.insert(other.to_string(), value.to_string());
            }
        }
    }
    if plan.name.is_empty() {
        plan.name = "Untitled".to_string();
    }
}

fn parse_thread(color_index: usize, value: &str) -> Option<ColorStop> {
    let mut parts = value.splitn(3, ',');
    let hex = parts.next()?.trim();
    let description = parts.next().unwrap_or("").trim();
    let catalog = parts.next().unwrap_or("").trim();
    let rgb = parse_hex(hex)?;
    Some(ColorStop {
        color_index,
        thread: Some(ThreadSpec {
            brand: String::new(),
            code: catalog.to_string(),
            name: description.to_string(),
            rgb,
        }),
    })
}

fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([red, green, blue])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{write_dst_vec, write_dst_with, WriteOptions};
    use embroidery_core::{ColorStop, Point, Stitch, StitchKind, StitchPlan, ThreadSpec};
    use std::io::Cursor;

    fn running(from: (f64, f64), to: (f64, f64)) -> Stitch {
        Stitch {
            kind: StitchKind::Running,
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            color_index: 0,
            object_id: 4,
        }
    }

    #[test]
    fn square_roundtrip_keeps_quantised_positions() {
        let mut plan = StitchPlan::new("Square");
        plan.stitches.push(running((0.0, 0.0), (10.0, 0.0)));
        plan.stitches.push(running((10.0, 0.0), (10.0, 10.0)));
        plan.stitches.push(running((10.0, 10.0), (0.0, 10.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        let read = read_dst(Cursor::new(bytes)).unwrap();
        assert_eq!(read.name, "Square");
        assert_eq!(read.stitches.len(), 3);
        assert!(read
            .stitches
            .iter()
            .all(|stitch| stitch.kind == StitchKind::Running));
        let ends: Vec<_> = read
            .stitches
            .iter()
            .map(|stitch| (stitch.to.x, stitch.to.y))
            .collect();
        assert_eq!(ends, vec![(10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        assert!(!read.metadata.contains_key("dst_st_mismatch"));
    }

    #[test]
    fn color_and_trim_roundtrip() {
        let mut plan = StitchPlan::new("Mix");
        plan.stitches.push(running((0.0, 0.0), (10.0, 0.0)));
        plan.stitches.push(Stitch {
            kind: StitchKind::ColorChange,
            from: Point::new(10.0, 0.0),
            to: Point::new(10.0, 0.0),
            color_index: 1,
            object_id: 1,
        });
        plan.stitches.push(running((10.0, 0.0), (12.0, 0.0)));
        plan.stitches.push(Stitch {
            kind: StitchKind::Trim,
            from: Point::new(12.0, 0.0),
            to: Point::new(12.0, 0.0),
            color_index: 1,
            object_id: 1,
        });
        plan.stitches.push(running((12.0, 0.0), (15.0, 0.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        let read = read_dst(Cursor::new(bytes)).unwrap();
        let kinds: Vec<_> = read.stitches.iter().map(|stitch| stitch.kind).collect();
        assert_eq!(
            kinds,
            vec![
                StitchKind::Running,
                StitchKind::ColorChange,
                StitchKind::Running,
                StitchKind::Trim,
                StitchKind::Running,
            ]
        );
        assert_eq!(read.stitches[1].color_index, 1);
        assert_eq!(read.stitches[4].color_index, 1);
        assert_eq!(read.stitches[3].from, read.stitches[3].to);
        assert_eq!(read.stitches.last().unwrap().to, Point::new(15.0, 0.0));
    }

    #[test]
    fn extended_thread_survives_a_roundtrip() {
        let mut plan = StitchPlan::new("Thread");
        plan.stitches.push(running((0.0, 0.0), (1.0, 0.0)));
        plan.colors.push(ColorStop {
            color_index: 0,
            thread: Some(ThreadSpec {
                brand: String::new(),
                code: "1147".into(),
                name: "Black".into(),
                rgb: [1, 2, 3],
            }),
        });
        let mut bytes = Vec::new();
        write_dst_with(
            &plan,
            &mut bytes,
            &WriteOptions {
                extended_header: true,
            },
        )
        .unwrap();
        let read = read_dst(Cursor::new(bytes)).unwrap();
        let thread = read.colors[0].thread.as_ref().unwrap();
        assert_eq!(thread.rgb, [1, 2, 3]);
        assert_eq!(thread.code, "1147");
        assert_eq!(thread.name, "Black");
    }

    #[test]
    fn truncated_record_names_the_next_step() {
        let mut bytes = write_dst_vec(&{
            let mut plan = StitchPlan::new("Short");
            plan.stitches.push(running((0.0, 0.0), (1.0, 0.0)));
            plan
        })
        .unwrap();
        bytes.pop();
        let error = read_dst(Cursor::new(bytes)).unwrap_err();
        assert!(error.to_string().contains("truncated"));
    }
}
