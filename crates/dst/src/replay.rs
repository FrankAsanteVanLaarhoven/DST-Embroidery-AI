//! Illustrative byte-bound replay for a DST this compiler already wrote.
//!
//! The timing matches the TwinLab M1 demonstration clock: 600 stitches per
//! minute, jumps at 100 mm/s with a 20 ms floor, and one second for a colour
//! change. It does not impose the centred 120 mm envelope. Callers that care
//! about a hoop check the decoded positions themselves.

use serde::Serialize;

use crate::{decode_record, encode_record, RecordCommand};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReplayEvent {
    pub index: usize,
    pub byte_offset: usize,
    pub raw: [u8; 3],
    pub kind: String,
    pub from_units: [i32; 2],
    pub to_units: [i32; 2],
    pub color_index: usize,
    pub start_ms: f64,
    pub end_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IllustrativeReplay {
    pub units: &'static str,
    pub decoder: &'static str,
    pub trim_policy: &'static str,
    pub timing: &'static str,
    pub events: Vec<ReplayEvent>,
    pub record_count: usize,
    pub needle_placements: usize,
    pub duration_ms: f64,
}

pub fn illustrative_replay(bytes: &[u8]) -> Result<IllustrativeReplay, String> {
    if bytes.len() < 515 || (bytes.len() - 512) % 3 != 0 || !bytes.starts_with(b"LA:") {
        return Err(
            "Expected a DST file with a 512-byte header and complete 3-byte records.".to_string(),
        );
    }
    let total = (bytes.len() - 512) / 3;
    if total > 60_000 {
        return Err(
            "The DST replay is above 60,000 records. Split the batch item and export it again."
                .to_string(),
        );
    }
    let mut events = Vec::with_capacity(total);
    let mut pen = [0i32, 0i32];
    let mut color_index = 0usize;
    let mut time = 0.0;
    let mut needle_placements = 0usize;
    let mut ended = false;
    for (index, chunk) in bytes[512..].chunks_exact(3).enumerate() {
        let raw = [chunk[0], chunk[1], chunk[2]];
        let record = decode_record(raw);
        let encoded = encode_record(record.dx, record.dy, record.command).map_err(|_| {
            format!(
                "DST record {index} is not canonical. Export the design again before making a proof."
            )
        })?;
        if encoded != raw {
            return Err(format!(
                "DST record {index} is not canonical. Export the design again before making a proof."
            ));
        }
        if record.command == RecordCommand::SequinMode {
            return Err(
                "Sequin mode is not part of this name-drop replay. Export the design again."
                    .to_string(),
            );
        }
        let next = [pen[0] + record.dx, pen[1] + record.dy];
        if next.iter().any(|value| value.unsigned_abs() > 1_000_000) {
            return Err(
                "A decoded record leaves the proof coordinate bound. Export the design again."
                    .to_string(),
            );
        }
        let (kind, duration) = match record.command {
            RecordCommand::Stitch => {
                needle_placements += 1;
                ("stitch", 100.0)
            }
            RecordCommand::Jump => {
                let distance =
                    ((record.dx as f64).powi(2) + (record.dy as f64).powi(2)).sqrt() / 10.0;
                ("jump", (distance / 100.0 * 1000.0).max(20.0))
            }
            RecordCommand::ColorChange => {
                color_index += 1;
                ("color_change", 1000.0)
            }
            RecordCommand::End => {
                if index + 1 != total {
                    return Err(
                        "Records follow the end marker. Export the design again before making a proof."
                            .to_string(),
                    );
                }
                ended = true;
                ("end", 1.0)
            }
            RecordCommand::SequinMode => unreachable!(),
        };
        events.push(ReplayEvent {
            index,
            byte_offset: 512 + index * 3,
            raw,
            kind: kind.to_string(),
            from_units: pen,
            to_units: next,
            color_index,
            start_ms: time,
            end_ms: time + duration,
        });
        time += duration;
        pen = next;
    }
    if !ended {
        return Err("The DST file has no end record. Export the design again.".to_string());
    }
    Ok(IllustrativeReplay {
        units: "0.1 mm; X right; Y down",
        decoder: "embroidery-dst::decode_record",
        trim_policy: "raw jumps; no trim inference",
        timing: "illustrative only: 600 spm, 100 mm/s jumps, 1 s colour change",
        events,
        record_count: total,
        needle_placements,
        duration_ms: time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use embroidery_core::{Point, Stitch, StitchKind, StitchPlan};

    fn running(from: (f64, f64), to: (f64, f64)) -> Stitch {
        Stitch {
            kind: StitchKind::Running,
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            color_index: 0,
            object_id: 1,
        }
    }

    #[test]
    fn a_wide_design_replays_without_the_centred_envelope() {
        let mut plan = StitchPlan::new("Wide");
        let mut x = 0.0_f64;
        while x < 150.0 {
            let next = (x + 10.0).min(150.0);
            plan.stitches.push(running((x, 10.0), (next, 10.0)));
            x = next;
        }
        let bytes = crate::write_dst_vec(&plan).unwrap();
        let replay = illustrative_replay(&bytes).unwrap();
        assert!(replay.events.last().unwrap().to_units[0] >= 1490);
        assert_eq!(replay.events.len() * 3 + 512, bytes.len());
        for event in &replay.events {
            assert_eq!(&bytes[event.byte_offset..event.byte_offset + 3], event.raw);
        }
        assert_eq!(replay.events.last().unwrap().kind, "end");
        assert_eq!(replay.events.last().unwrap().raw, [0, 0, 243]);
    }
}
