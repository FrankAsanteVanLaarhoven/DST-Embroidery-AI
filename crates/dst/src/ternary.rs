//! Tajima DST 3-byte records.
//!
//! The displacement is a sum of signed weights 81, 27, 9, 3, and 1 counts.
//! pyembroidery's `encode_record` is the bit-level reference: Y is negated
//! before the bits are packed, and the reader negates it back. Control bits
//! live in byte 2 and do not share positions with a normal stitch's movement
//! bits, except the end record, whose +81 and -81 Y bits cancel.

use embroidery_core::DST_MAX_DELTA_UNITS;

/// A command that fits in one DST record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordCommand {
    Stitch,
    Jump,
    ColorChange,
    End,
    SequinMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedRecord {
    /// Pattern-space delta in 0.1 mm counts. Y is positive downward.
    pub dx: i32,
    pub dy: i32,
    pub command: RecordCommand,
}

/// Pack one record.
///
/// Colour-change, end, and sequin-mode records ignore `dx` and `dy`, matching
/// pyembroidery. Stitch and jump records must already be within ±121 counts.
pub fn encode_record(dx: i32, dy: i32, command: RecordCommand) -> Result<[u8; 3], String> {
    match command {
        RecordCommand::ColorChange => Ok([0x00, 0x00, 0b1100_0011]),
        RecordCommand::End => Ok([0x00, 0x00, 0b1111_0011]),
        RecordCommand::SequinMode => Ok([0x00, 0x00, 0b0100_0011]),
        RecordCommand::Stitch | RecordCommand::Jump => {
            if dx.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
                || dy.unsigned_abs() > DST_MAX_DELTA_UNITS as u32
            {
                return Err(format!(
                    "DST record ({dx}, {dy}) exceeds ±{DST_MAX_DELTA_UNITS} counts. Split the move before encoding."
                ));
            }
            let mut bytes = [0u8; 3];
            if command == RecordCommand::Jump {
                bytes[2] |= 1 << 7;
            }
            bytes[2] |= 0b0000_0011;
            pack_x(&mut bytes, dx)?;
            // File Y grows upward. Pattern Y grows downward.
            pack_y(&mut bytes, -dy)?;
            Ok(bytes)
        }
    }
}

pub fn decode_record(bytes: [u8; 3]) -> DecodedRecord {
    DecodedRecord {
        dx: decode_dx(bytes[0], bytes[1], bytes[2]),
        dy: decode_dy(bytes[0], bytes[1], bytes[2]),
        command: classify(bytes[2]),
    }
}

fn pack_x(bytes: &mut [u8; 3], mut x: i32) -> Result<(), String> {
    if x > 40 {
        bytes[2] |= 1 << 2;
        x -= 81;
    }
    if x < -40 {
        bytes[2] |= 1 << 3;
        x += 81;
    }
    if x > 13 {
        bytes[1] |= 1 << 2;
        x -= 27;
    }
    if x < -13 {
        bytes[1] |= 1 << 3;
        x += 27;
    }
    if x > 4 {
        bytes[0] |= 1 << 2;
        x -= 9;
    }
    if x < -4 {
        bytes[0] |= 1 << 3;
        x += 9;
    }
    if x > 1 {
        bytes[1] |= 1 << 0;
        x -= 3;
    }
    if x < -1 {
        bytes[1] |= 1 << 1;
        x += 3;
    }
    if x > 0 {
        bytes[0] |= 1 << 0;
        x -= 1;
    }
    if x < 0 {
        bytes[0] |= 1 << 1;
        x += 1;
    }
    if x != 0 {
        return Err(format!(
            "DST X value {x} could not be packed. The record exceeds the ternary weights."
        ));
    }
    Ok(())
}

fn pack_y(bytes: &mut [u8; 3], mut y: i32) -> Result<(), String> {
    if y > 40 {
        bytes[2] |= 1 << 5;
        y -= 81;
    }
    if y < -40 {
        bytes[2] |= 1 << 4;
        y += 81;
    }
    if y > 13 {
        bytes[1] |= 1 << 5;
        y -= 27;
    }
    if y < -13 {
        bytes[1] |= 1 << 4;
        y += 27;
    }
    if y > 4 {
        bytes[0] |= 1 << 5;
        y -= 9;
    }
    if y < -4 {
        bytes[0] |= 1 << 4;
        y += 9;
    }
    if y > 1 {
        bytes[1] |= 1 << 7;
        y -= 3;
    }
    if y < -1 {
        bytes[1] |= 1 << 6;
        y += 3;
    }
    if y > 0 {
        bytes[0] |= 1 << 7;
        y -= 1;
    }
    if y < 0 {
        bytes[0] |= 1 << 6;
        y += 1;
    }
    if y != 0 {
        return Err(format!(
            "DST Y value {y} could not be packed. The record exceeds the ternary weights."
        ));
    }
    Ok(())
}

fn bit(byte: u8, position: u8) -> i32 {
    i32::from((byte >> position) & 1)
}

fn decode_dx(b0: u8, b1: u8, b2: u8) -> i32 {
    bit(b2, 2) * 81
        + bit(b2, 3) * -81
        + bit(b1, 2) * 27
        + bit(b1, 3) * -27
        + bit(b0, 2) * 9
        + bit(b0, 3) * -9
        + bit(b1, 0) * 3
        + bit(b1, 1) * -3
        + bit(b0, 0)
        - bit(b0, 1)
}

fn decode_dy(b0: u8, b1: u8, b2: u8) -> i32 {
    let file_y = bit(b2, 5) * 81
        + bit(b2, 4) * -81
        + bit(b1, 5) * 27
        + bit(b1, 4) * -27
        + bit(b0, 5) * 9
        + bit(b0, 4) * -9
        + bit(b1, 7) * 3
        + bit(b1, 6) * -3
        + bit(b0, 7)
        - bit(b0, 6);
    -file_y
}

fn classify(control: u8) -> RecordCommand {
    if control & 0b1111_0011 == 0b1111_0011 {
        RecordCommand::End
    } else if control & 0b1100_0011 == 0b1100_0011 {
        RecordCommand::ColorChange
    } else if control & 0b0100_0011 == 0b0100_0011 {
        RecordCommand::SequinMode
    } else if control & 0b1000_0011 == 0b1000_0011 {
        RecordCommand::Jump
    } else {
        RecordCommand::Stitch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_legal_delta_roundtrips_for_stitch_and_jump() {
        for dy in -DST_MAX_DELTA_UNITS..=DST_MAX_DELTA_UNITS {
            for dx in -DST_MAX_DELTA_UNITS..=DST_MAX_DELTA_UNITS {
                for command in [RecordCommand::Stitch, RecordCommand::Jump] {
                    let bytes = encode_record(dx, dy, command).unwrap();
                    let decoded = decode_record(bytes);
                    assert_eq!(
                        decoded.dx, dx,
                        "dx {dx} dy {dy} {command:?} bytes {bytes:?}"
                    );
                    assert_eq!(
                        decoded.dy, dy,
                        "dx {dx} dy {dy} {command:?} bytes {bytes:?}"
                    );
                    assert_eq!(decoded.command, command);
                }
            }
        }
    }

    #[test]
    fn control_records_match_the_public_bit_patterns() {
        assert_eq!(
            encode_record(4, -2, RecordCommand::ColorChange).unwrap(),
            [0x00, 0x00, 0xC3]
        );
        assert_eq!(
            encode_record(0, 0, RecordCommand::End).unwrap(),
            [0x00, 0x00, 0xF3]
        );
        assert_eq!(
            encode_record(0, 0, RecordCommand::SequinMode).unwrap(),
            [0x00, 0x00, 0x43]
        );
    }

    #[test]
    fn captured_pyembroidery_vectors() {
        // Captured from pyembroidery DstWriter.encode_record.
        assert_eq!(
            encode_record(1, -1, RecordCommand::Stitch).unwrap(),
            [0x81, 0x00, 0x03]
        );
        assert_eq!(
            encode_record(100, 0, RecordCommand::Stitch).unwrap(),
            [0x09, 0x04, 0x07]
        );
        assert_eq!(
            encode_record(0, -50, RecordCommand::Stitch).unwrap(),
            [0x40, 0x50, 0x23]
        );
        assert_eq!(
            encode_record(2, 2, RecordCommand::Jump).unwrap(),
            [0x82, 0x41, 0x83]
        );
        assert_eq!(
            encode_record(-4, -4, RecordCommand::Jump).unwrap(),
            [0x82, 0x82, 0x83]
        );
    }

    #[test]
    fn full_scale_movement_is_not_classified_as_a_control_record() {
        let stitch = decode_record(encode_record(121, 121, RecordCommand::Stitch).unwrap());
        let jump = decode_record(encode_record(-121, -121, RecordCommand::Jump).unwrap());
        assert_eq!(stitch.command, RecordCommand::Stitch);
        assert_eq!(jump.command, RecordCommand::Jump);
    }

    #[test]
    fn oversize_delta_is_rejected() {
        let error = encode_record(122, 0, RecordCommand::Stitch).unwrap_err();
        assert!(error.contains("121"));
    }
}
