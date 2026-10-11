use std::io::{self, Write};

use embroidery_core::{mm_to_units, StitchKind, StitchPlan, ThreadSpec, DST_MAX_DELTA_UNITS};

use crate::ternary::{encode_record, RecordCommand};

const MAX_RECORDS: usize = 9_999_999;
const MAX_COLORS: usize = 999;
const MAX_HEADER_EXTENT: u32 = 99_999;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MachineRecord {
    pub dx: i32,
    pub dy: i32,
    pub command: RecordCommand,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

/// Header options. The default header matches pyembroidery's non-extended DST writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteOptions {
    /// Write `TC:` thread lines after the standard fields when they fit in 512 bytes.
    pub extended_header: bool,
}

pub fn write_dst<W: Write>(plan: &StitchPlan, writer: W) -> io::Result<()> {
    write_dst_with(plan, writer, &WriteOptions::default())
}

pub fn write_dst_vec(plan: &StitchPlan) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    write_dst(plan, &mut buffer)?;
    Ok(buffer)
}

pub fn write_dst_with<W: Write>(
    plan: &StitchPlan,
    mut writer: W,
    options: &WriteOptions,
) -> io::Result<()> {
    let (records, _links, bounds, end) = compile_machine(plan)?;
    let header = header_bytes(plan, &records, bounds, end, options)?;
    writer.write_all(&header)?;
    for record in &records {
        let bytes = encode_record(record.dx, record.dy, record.command)
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
        writer.write_all(&bytes)?;
    }
    writer.flush()
}

/// Validate, then write. Errors name every blocking finding.
pub fn compile_dst<W: Write>(
    plan: &StitchPlan,
    writer: W,
    max_stitch_mm: f64,
    max_jump_mm: f64,
) -> io::Result<()> {
    let report = embroidery_core::validate(plan, max_stitch_mm, max_jump_mm);
    if !report.ok() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "The stitch plan failed validation. Fix these errors and compile again:\n{}",
                report.errors.join("\n")
            ),
        ));
    }
    write_dst(plan, writer)
}

pub fn compile_to_vec(
    plan: &StitchPlan,
    max_stitch_mm: f64,
    max_jump_mm: f64,
) -> io::Result<Vec<u8>> {
    Ok(compile_to_vec_with_links(plan, max_stitch_mm, max_jump_mm)?.0)
}

/// One exported body record, excluding the end record, tied back to the plan.
///
/// DST does not store object identity. The link is produced while compiling so
/// a later reader of these exact bytes can still name the source object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportLink {
    pub object_id: u64,
    pub plan_stitch_index: u32,
}

/// Validate, write DST, and return a sidecar aligned with the body records.
///
/// The bytes match [`compile_to_vec`]. `links.len() + 1` equals the number of
/// 3-byte records, because the final end record has no design object.
pub fn compile_to_vec_with_links(
    plan: &StitchPlan,
    max_stitch_mm: f64,
    max_jump_mm: f64,
) -> io::Result<(Vec<u8>, Vec<ExportLink>)> {
    let report = embroidery_core::validate(plan, max_stitch_mm, max_jump_mm);
    if !report.ok() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "The stitch plan failed validation. Fix these errors and compile again:\n{}",
                report.errors.join("\n")
            ),
        ));
    }
    let (records, links, bounds, end) = compile_machine(plan)?;
    if links.len() + 1 != records.len() {
        return Err(invalid(
            "The export sidecar does not match the DST records. This is a compiler bug; report the design.",
        ));
    }
    let header = header_bytes(plan, &records, bounds, end, &WriteOptions::default())?;
    let mut buffer = Vec::with_capacity(512 + records.len() * 3);
    buffer.extend_from_slice(&header);
    for record in &records {
        let bytes = encode_record(record.dx, record.dy, record.command)
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
        buffer.extend_from_slice(&bytes);
    }
    Ok((buffer, links))
}

type CompiledMachine = (Vec<MachineRecord>, Vec<ExportLink>, Bounds, (i32, i32));

fn compile_machine(plan: &StitchPlan) -> io::Result<CompiledMachine> {
    if plan.stitches.is_empty() {
        return Err(invalid(
            "Stitch plan contains no stitches. Generate a running, satin, or fill object before writing DST.",
        ));
    }

    let mut records = Vec::new();
    let mut links = Vec::new();
    let mut pen = (0i32, 0i32);
    let mut bounds: Option<Bounds> = None;

    for (index, stitch) in plan.stitches.iter().enumerate() {
        let link = ExportLink {
            object_id: stitch.object_id,
            plan_stitch_index: index as u32,
        };
        match stitch.kind {
            StitchKind::Jump => {
                let to = point_units(stitch.to)?;
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    to,
                    RecordCommand::Jump,
                    &mut bounds,
                    link,
                )?;
            }
            StitchKind::ColorChange => {
                let from = point_units(stitch.from)?;
                let to = point_units(stitch.to)?;
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    from,
                    RecordCommand::Jump,
                    &mut bounds,
                    link,
                )?;
                push_linked(
                    &mut records,
                    &mut links,
                    MachineRecord {
                        dx: 0,
                        dy: 0,
                        command: RecordCommand::ColorChange,
                    },
                    link,
                );
                note(&mut bounds, pen);
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    to,
                    RecordCommand::Jump,
                    &mut bounds,
                    link,
                )?;
            }
            StitchKind::Trim => {
                let from = point_units(stitch.from)?;
                let to = point_units(stitch.to)?;
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    from,
                    RecordCommand::Jump,
                    &mut bounds,
                    link,
                )?;
                // pyembroidery's default trim_at=3 signature. Net movement is zero,
                // so the jog is not part of the design bounds.
                for (dx, dy) in [(2, 2), (-4, -4), (2, 2)] {
                    push_linked(
                        &mut records,
                        &mut links,
                        MachineRecord {
                            dx,
                            dy,
                            command: RecordCommand::Jump,
                        },
                        link,
                    );
                }
                note(&mut bounds, pen);
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    to,
                    RecordCommand::Jump,
                    &mut bounds,
                    link,
                )?;
            }
            StitchKind::Running
            | StitchKind::Satin
            | StitchKind::Fill
            | StitchKind::Motif
            | StitchKind::Applique => {
                let from = point_units(stitch.from)?;
                let to = point_units(stitch.to)?;
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    from,
                    RecordCommand::Jump,
                    &mut bounds,
                    link,
                )?;
                move_pen(
                    &mut records,
                    &mut links,
                    &mut pen,
                    to,
                    RecordCommand::Stitch,
                    &mut bounds,
                    link,
                )?;
            }
        }
    }

    records.push(MachineRecord {
        dx: 0,
        dy: 0,
        command: RecordCommand::End,
    });
    note(&mut bounds, pen);

    if records.len() > MAX_RECORDS {
        return Err(invalid(format!(
            "This plan compiles to {} DST records, which does not fit the 7-digit ST field. Split the design and export the parts separately.",
            records.len()
        )));
    }

    let bounds = bounds.ok_or_else(|| {
        invalid("DST compile produced no needle positions. Add a sewing stitch and export again.")
    })?;
    Ok((records, links, bounds, pen))
}

fn push_linked(
    records: &mut Vec<MachineRecord>,
    links: &mut Vec<ExportLink>,
    record: MachineRecord,
    link: ExportLink,
) {
    records.push(record);
    links.push(link);
}

fn move_pen(
    records: &mut Vec<MachineRecord>,
    links: &mut Vec<ExportLink>,
    pen: &mut (i32, i32),
    target: (i32, i32),
    command: RecordCommand,
    bounds: &mut Option<Bounds>,
    link: ExportLink,
) -> io::Result<()> {
    let dx = target.0 - pen.0;
    let dy = target.1 - pen.1;
    if dx == 0 && dy == 0 {
        if command == RecordCommand::Stitch {
            push_linked(
                records,
                links,
                MachineRecord {
                    dx: 0,
                    dy: 0,
                    command,
                },
                link,
            );
            note(bounds, *pen);
        }
        return Ok(());
    }

    let mut x = pen.0;
    let mut y = pen.1;
    for (step_x, step_y) in split_delta(dx, dy)? {
        push_linked(
            records,
            links,
            MachineRecord {
                dx: step_x,
                dy: step_y,
                command,
            },
            link,
        );
        x += step_x;
        y += step_y;
        note(bounds, (x, y));
    }
    *pen = (x, y);
    if *pen != target {
        return Err(invalid(format!(
            "DST split of ({dx}, {dy}) ended at {pen:?} instead of {target:?}. This is a compiler bug; report the delta."
        )));
    }
    Ok(())
}

pub(crate) fn split_delta(dx: i32, dy: i32) -> io::Result<Vec<(i32, i32)>> {
    let steps = dx
        .unsigned_abs()
        .div_ceil(DST_MAX_DELTA_UNITS as u32)
        .max(dy.unsigned_abs().div_ceil(DST_MAX_DELTA_UNITS as u32))
        .max(1);
    if steps > 1_000_000 {
        return Err(invalid(format!(
            "DST delta ({dx}, {dy}) needs {steps} records. Move the design closer together and export again."
        )));
    }
    let steps_i = steps as i32;
    let mut parts = Vec::with_capacity(steps as usize);
    let mut previous_x = 0i32;
    let mut previous_y = 0i32;
    for step in 1..=steps_i {
        let cumulative_x = ((dx as i64) * i64::from(step) / i64::from(steps_i)) as i32;
        let cumulative_y = ((dy as i64) * i64::from(step) / i64::from(steps_i)) as i32;
        let step_x = cumulative_x - previous_x;
        let step_y = cumulative_y - previous_y;
        if step_x.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
            || step_y.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
        {
            return Err(invalid(format!(
                "Could not split DST delta ({dx}, {dy}) into ±{DST_MAX_DELTA_UNITS}-count records. Report this delta."
            )));
        }
        parts.push((step_x, step_y));
        previous_x = cumulative_x;
        previous_y = cumulative_y;
    }
    Ok(parts)
}

fn note(bounds: &mut Option<Bounds>, point: (i32, i32)) {
    match bounds {
        Some(current) => {
            current.min_x = current.min_x.min(point.0);
            current.min_y = current.min_y.min(point.1);
            current.max_x = current.max_x.max(point.0);
            current.max_y = current.max_y.max(point.1);
        }
        None => {
            *bounds = Some(Bounds {
                min_x: point.0,
                min_y: point.1,
                max_x: point.0,
                max_y: point.1,
            });
        }
    }
}

fn header_bytes(
    plan: &StitchPlan,
    records: &[MachineRecord],
    bounds: Bounds,
    end: (i32, i32),
    options: &WriteOptions,
) -> io::Result<[u8; 512]> {
    let mut text = String::new();
    text.push_str(&label_line(&plan.name));
    text.push_str(&format!("ST:{:7}\r", records.len()));
    let color_changes = records
        .iter()
        .filter(|record| record.command == RecordCommand::ColorChange)
        .count();
    if color_changes > MAX_COLORS {
        return Err(invalid(format!(
            "This plan has {color_changes} colour changes. The DST CO field holds 3 digits. Split the design before {MAX_COLORS} changes."
        )));
    }
    text.push_str(&format!("CO:{color_changes:3}\r"));
    text.push_str(&extent("+X", bounds.max_x)?);
    text.push_str(&extent("-X", bounds.min_x)?);
    text.push_str(&extent("+Y", bounds.max_y)?);
    text.push_str(&extent("-Y", bounds.min_y)?);
    text.push_str(&signed("AX", end.0)?);
    text.push_str(&signed("AY", -end.1)?);
    text.push_str(&signed("MX", 0)?);
    text.push_str(&signed("MY", 0)?);
    text.push_str("PD:******\r");
    if options.extended_header {
        for stop in &plan.colors {
            if let Some(thread) = &stop.thread {
                text.push_str(&thread_line(thread));
            }
        }
    }

    let mut bytes = text.into_bytes();
    bytes.push(0x1A);
    if bytes.len() > 512 {
        return Err(invalid(
            "DST header does not fit in 512 bytes. Shorten the design name or thread list and export again.",
        ));
    }
    bytes.resize(512, b' ');
    let mut header = [b' '; 512];
    header.copy_from_slice(&bytes);
    Ok(header)
}

fn label_line(name: &str) -> String {
    let mut label: String = name
        .chars()
        .map(|character| {
            if character.is_ascii()
                && character != '\r'
                && character != '\n'
                && character != '\u{1a}'
            {
                character
            } else {
                '_'
            }
        })
        .take(16)
        .collect();
    if label.trim().is_empty() {
        label = "Untitled".to_string();
    }
    format!("LA:{label:<16}\r")
}

fn thread_line(thread: &ThreadSpec) -> String {
    let description = if thread.brand.is_empty() {
        sanitize(&thread.name)
    } else {
        sanitize(&format!("{} {}", thread.brand, thread.name))
    };
    format!(
        "TC:#{:02X}{:02X}{:02X},{},{}\r",
        thread.rgb[0],
        thread.rgb[1],
        thread.rgb[2],
        description,
        sanitize(&thread.code)
    )
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii()
                && character != ','
                && character != ':'
                && character != '\r'
                && character != '\n'
            {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn extent(label: &str, value: i32) -> io::Result<String> {
    let magnitude = value.unsigned_abs();
    if magnitude > MAX_HEADER_EXTENT {
        return Err(invalid(format!(
            "DST extent {label} is {magnitude} counts. Header fields hold five digits (±9999.9 mm). Move the design closer to the origin."
        )));
    }
    Ok(format!("{label}:{magnitude:5}\r"))
}

fn signed(label: &str, value: i32) -> io::Result<String> {
    let magnitude = value.unsigned_abs();
    if magnitude > MAX_HEADER_EXTENT {
        return Err(invalid(format!(
            "DST field {label} is {value} counts. Header fields hold five digits (±9999.9 mm). Move the design closer to the origin."
        )));
    }
    if value >= 0 {
        Ok(format!("{label}:+{magnitude:5}\r"))
    } else {
        Ok(format!("{label}:-{magnitude:5}\r"))
    }
}

fn point_units(point: embroidery_core::Point) -> io::Result<(i32, i32)> {
    let x = mm_to_units(point.x).map_err(invalid)?;
    let y = mm_to_units(point.y).map_err(invalid)?;
    Ok((x, y))
}

fn invalid(message: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use embroidery_core::{ColorStop, Point, Stitch, StitchKind, StitchPlan, ThreadSpec};

    fn running(from: (f64, f64), to: (f64, f64)) -> Stitch {
        Stitch {
            kind: StitchKind::Running,
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            color_index: 0,
            object_id: 1,
        }
    }

    fn at(kind: StitchKind, at: (f64, f64), to: (f64, f64)) -> Stitch {
        Stitch {
            kind,
            from: Point::new(at.0, at.1),
            to: Point::new(to.0, to.1),
            color_index: 0,
            object_id: 1,
        }
    }

    fn header_text(bytes: &[u8]) -> String {
        let end = bytes.iter().position(|byte| *byte == 0x1A).unwrap();
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }

    #[test]
    fn square_matches_captured_pyembroidery_file() {
        let mut plan = StitchPlan::new("Square");
        plan.stitches.push(running((0.0, 0.0), (10.0, 0.0)));
        plan.stitches.push(running((10.0, 0.0), (10.0, 10.0)));
        plan.stitches.push(running((10.0, 10.0), (0.0, 10.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        let expected_header = "LA:Square          \rST:      4\rCO:  0\r+X:  100\r-X:    0\r+Y:  100\r-Y:    0\rAX:+    0\rAY:-  100\rMX:+    0\rMY:+    0\rPD:******\r";
        assert_eq!(header_text(&bytes), expected_header);
        assert_eq!(bytes.len(), 512 + 12);
        assert_eq!(&bytes[512..], &hex_bytes("09040760101306080b0000f3"));
        assert_eq!(bytes[511], b' ');
        assert!(bytes[..512].contains(&0x1A));
    }

    #[test]
    fn negative_y_matches_captured_pyembroidery_file() {
        let mut plan = StitchPlan::new("NegY");
        plan.stitches.push(running((0.0, 0.0), (0.0, -5.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        assert!(header_text(&bytes).contains("+Y:   50\r-Y:   50\rAX:+    0\rAY:+   50\r"));
        assert_eq!(&bytes[512..], &hex_bytes("4050230000f3"));
    }

    #[test]
    fn color_change_matches_captured_pyembroidery_file() {
        let mut plan = StitchPlan::new("Color");
        plan.stitches.push(running((0.0, 0.0), (10.0, 0.0)));
        plan.stitches
            .push(at(StitchKind::ColorChange, (10.0, 0.0), (10.0, 0.0)));
        plan.stitches.push(running((10.0, 0.0), (20.0, 0.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        assert!(header_text(&bytes).contains("ST:      4\rCO:  1\r+X:  200\r-X:  100\r"));
        assert_eq!(&bytes[512..], &hex_bytes("0904070000c30904070000f3"));
    }

    #[test]
    fn trim_signature_matches_pyembroidery_body_and_counts_real_records() {
        let mut plan = StitchPlan::new("Trim0");
        plan.stitches.push(running((0.0, 0.0), (10.0, 0.0)));
        plan.stitches
            .push(at(StitchKind::Trim, (10.0, 0.0), (10.0, 0.0)));
        plan.stitches.push(running((10.0, 0.0), (20.0, 0.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        // pyembroidery's ST field counts the TRIM command once. The file contains
        // the expanded jump signature, so this compiler stores the record count.
        let header = header_text(&bytes);
        assert!(header.contains("ST:      6\r"), "{header}");
        assert!(header.contains("+X:  200\r-X:  100\r"), "{header}");
        assert!(header.contains("AX:+  200\rAY:+    0\r"), "{header}");
        assert_eq!(
            &bytes[512..],
            &hex_bytes("0904078241838282838241830904070000f3")
        );
    }

    #[test]
    fn a_long_jump_is_split_without_changing_the_endpoint() {
        let parts = split_delta(300, 0).unwrap();
        assert_eq!(parts, vec![(100, 0), (100, 0), (100, 0)]);
        let odd = split_delta(250, -50).unwrap();
        let sum_x: i32 = odd.iter().map(|part| part.0).sum();
        let sum_y: i32 = odd.iter().map(|part| part.1).sum();
        assert_eq!((sum_x, sum_y), (250, -50));
        assert!(odd
            .iter()
            .all(|part| part.0.abs() <= 121 && part.1.abs() <= 121));

        let mut plan = StitchPlan::new("Jump");
        plan.stitches
            .push(at(StitchKind::Jump, (0.0, 0.0), (30.0, 0.0)));
        plan.stitches.push(running((30.0, 0.0), (30.0, 1.0)));
        let bytes = write_dst_vec(&plan).unwrap();
        assert_eq!((bytes.len() - 512) / 3, 5);
    }

    #[test]
    fn extended_header_carries_a_thread_and_still_fits() {
        let mut plan = StitchPlan::new("Thread");
        plan.stitches.push(running((0.0, 0.0), (1.0, 0.0)));
        plan.colors.push(ColorStop {
            color_index: 0,
            thread: Some(ThreadSpec {
                brand: "Madeira".into(),
                code: "1147".into(),
                name: "Black".into(),
                rgb: [0, 0, 0],
            }),
        });
        let mut buffer = Vec::new();
        write_dst_with(
            &plan,
            &mut buffer,
            &WriteOptions {
                extended_header: true,
            },
        )
        .unwrap();
        let header = String::from_utf8_lossy(&buffer[..512]);
        assert!(header.contains("TC:#000000,Madeira Black,1147\r"));
        assert_eq!(buffer.len(), 512 + 6);
    }

    #[test]
    fn strict_compile_rejects_a_stitch_past_the_record_limit() {
        let mut plan = StitchPlan::new("Long");
        plan.stitches.push(running((0.0, 0.0), (20.0, 0.0)));
        let error = compile_to_vec(&plan, 12.1, 1_000.0).unwrap_err();
        assert!(error.to_string().contains("failed validation"));
        assert!(write_dst_vec(&plan).is_ok());
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect()
    }
}
