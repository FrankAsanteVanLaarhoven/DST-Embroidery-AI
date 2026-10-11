//! Byte-level comparison with the installed pyembroidery oracle.
//!
//! The venv lives at the repository root. Set `EMBROIDERY_PYTHON` to point
//! somewhere else. This test is the proof that the ternary packer and the
//! unsplit header match `DstWriter`, not a second copy of our own encoder.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use embroidery_core::{Point, Stitch, StitchKind, StitchPlan};
use embroidery_dst::{encode_record, write_dst_vec, RecordCommand};

const ORACLE: &str = r#"
import io
import sys

from pyembroidery import EmbPattern
from pyembroidery.DstWriter import encode_record, write
from pyembroidery.EmbConstant import COLOR_CHANGE, COMMAND_MASK, END, JUMP, STITCH, TRIM


def main():
    mode = sys.argv[1]
    if mode == "matrix":
        out = sys.stdout.buffer
        for flag in (STITCH, JUMP):
            for y in range(-121, 122):
                for x in range(-121, 122):
                    out.write(encode_record(x, y, flag))
        out.write(encode_record(9, -4, COLOR_CHANGE))
        out.write(encode_record(0, 0, END))
        return
    if mode == "file":
        pattern = EmbPattern()
        pattern.metadata("name", sys.argv[2])
        for line in sys.stdin.read().splitlines():
            parts = line.split()
            if not parts or parts[0] == "end":
                continue
            flag = {"stitch": STITCH, "jump": JUMP, "color": COLOR_CHANGE, "trim": TRIM}[parts[0]]
            pattern.add_stitch_absolute(flag, int(parts[1]), int(parts[2]))
        if pattern.stitches:
            last = pattern.stitches[-1]
            pattern.add_stitch_absolute(END, last[0], last[1])
        else:
            pattern.add_stitch_absolute(END, 0, 0)
        buffer = io.BytesIO()
        write(pattern, buffer)
        sys.stdout.buffer.write(buffer.getvalue())
        return
    if mode == "read":
        import os
        import tempfile

        data = sys.stdin.buffer.read()
        handle, path = tempfile.mkstemp(suffix=".dst")
        os.write(handle, data)
        os.close(handle)
        try:
            pattern = EmbPattern(path)
        finally:
            os.remove(path)
        for stitch in pattern.stitches:
            command = int(stitch[2]) & COMMAND_MASK
            print(f"{int(round(stitch[0]))} {int(round(stitch[1]))} {command}")
        return
    raise SystemExit(f"unknown mode {mode}")


main()
"#;

fn python() -> PathBuf {
    if let Ok(path) = std::env::var("EMBROIDERY_PYTHON") {
        return PathBuf::from(path);
    }
    let venv = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.venv/bin/python");
    if venv.exists() {
        venv
    } else {
        PathBuf::from("python3")
    }
}

fn oracle(mode: &str, args: &[&str], stdin: &[u8]) -> Vec<u8> {
    let mut command = Command::new(python());
    command.arg("-c").arg(ORACLE).arg(mode);
    for arg in args {
        command.arg(arg);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| {
            panic!("Could not start the pyembroidery oracle ({error}). Create .venv and pip install pyembroidery.")
        });
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for oracle");
    if !output.status.success() {
        panic!(
            "pyembroidery oracle failed ({})\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output.stdout
}

fn stitch(kind: StitchKind, from: (f64, f64), to: (f64, f64)) -> Stitch {
    Stitch {
        kind,
        from: Point::new(from.0, from.1),
        to: Point::new(to.0, to.1),
        color_index: 0,
        object_id: 1,
    }
}

#[test]
fn ternary_matrix_matches_pyembroidery_encode_record() {
    let expected = oracle("matrix", &[], b"");
    let mut actual = Vec::with_capacity(expected.len());
    for command in [RecordCommand::Stitch, RecordCommand::Jump] {
        for dy in -121..=121 {
            for dx in -121..=121 {
                actual.extend(encode_record(dx, dy, command).unwrap());
            }
        }
    }
    actual.extend(encode_record(9, -4, RecordCommand::ColorChange).unwrap());
    actual.extend(encode_record(0, 0, RecordCommand::End).unwrap());
    assert_eq!(actual, expected);
}

#[test]
fn unsplit_files_match_dst_writer_byte_for_byte() {
    let cases = [
        (
            "Square",
            "stitch 100 0\nstitch 100 100\nstitch 0 100\n",
            vec![
                stitch(StitchKind::Running, (0.0, 0.0), (10.0, 0.0)),
                stitch(StitchKind::Running, (10.0, 0.0), (10.0, 10.0)),
                stitch(StitchKind::Running, (10.0, 10.0), (0.0, 10.0)),
            ],
        ),
        (
            "NegY",
            "stitch 0 -50\n",
            vec![stitch(StitchKind::Running, (0.0, 0.0), (0.0, -5.0))],
        ),
        (
            "Color",
            "stitch 100 0\ncolor 100 0\nstitch 200 0\n",
            vec![
                stitch(StitchKind::Running, (0.0, 0.0), (10.0, 0.0)),
                stitch(StitchKind::ColorChange, (10.0, 0.0), (10.0, 0.0)),
                stitch(StitchKind::Running, (10.0, 0.0), (20.0, 0.0)),
            ],
        ),
    ];

    for (name, script, stitches) in cases {
        let mut plan = StitchPlan::new(name);
        plan.stitches = stitches;
        let actual = write_dst_vec(&plan).unwrap();
        let expected = oracle("file", &[name], script.as_bytes());
        assert_eq!(actual, expected, "{name} diverged from pyembroidery");
    }
}

#[test]
fn split_jump_matches_a_presplit_pyembroidery_pattern() {
    let mut plan = StitchPlan::new("Long");
    plan.stitches
        .push(stitch(StitchKind::Jump, (0.0, 0.0), (30.0, 0.0)));
    plan.stitches
        .push(stitch(StitchKind::Running, (30.0, 0.0), (30.0, 1.0)));
    let actual = write_dst_vec(&plan).unwrap();
    let expected = oracle(
        "file",
        &["Long"],
        b"jump 100 0\njump 200 0\njump 300 0\nstitch 300 10\n",
    );
    assert_eq!(actual, expected);
}

#[test]
fn pyembroidery_reads_our_square_back_to_the_same_counts() {
    let mut plan = StitchPlan::new("Square");
    plan.stitches = vec![
        stitch(StitchKind::Running, (0.0, 0.0), (10.0, 0.0)),
        stitch(StitchKind::Running, (10.0, 0.0), (10.0, 10.0)),
        stitch(StitchKind::Running, (10.0, 10.0), (0.0, 10.0)),
    ];
    let bytes = write_dst_vec(&plan).unwrap();
    let text = String::from_utf8(oracle("read", &[], &bytes)).unwrap();
    let commands: Vec<_> = text.lines().map(str::to_string).collect();
    // STITCH is 0 and END is 4 in pyembroidery.EmbConstant.
    assert_eq!(
        commands,
        vec![
            "100 0 0".to_string(),
            "100 100 0".to_string(),
            "0 100 0".to_string(),
            "0 100 4".to_string(),
        ]
    );
}

#[test]
fn square_fixture_matches_the_compiler() {
    let fixture = include_bytes!("../../../fixtures/dst/square_running.dst");
    let mut plan = StitchPlan::new("Square");
    plan.stitches = vec![
        stitch(StitchKind::Running, (0.0, 0.0), (10.0, 0.0)),
        stitch(StitchKind::Running, (10.0, 0.0), (10.0, 10.0)),
        stitch(StitchKind::Running, (10.0, 10.0), (0.0, 10.0)),
    ];
    assert_eq!(fixture.as_slice(), write_dst_vec(&plan).unwrap().as_slice());
}
