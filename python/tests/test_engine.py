"""The FFI compile of the square matches pyembroidery's DST writer and the fixture."""

import io
from pathlib import Path

import pytest
from pyembroidery import EmbPattern
from pyembroidery.DstWriter import write
from pyembroidery.EmbConstant import END, STITCH

from embroidery_ai import DigitisingProposal, Engine, EngineError, RegionProposal

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "fixtures" / "dst" / "square_running.dst"

SQUARE = {
    "name": "Square",
    "stitches": [
        {
            "kind": "running",
            "from": {"x": 0.0, "y": 0.0},
            "to": {"x": 10.0, "y": 0.0},
            "color_index": 0,
            "object_id": 1,
        },
        {
            "kind": "running",
            "from": {"x": 10.0, "y": 0.0},
            "to": {"x": 10.0, "y": 10.0},
            "color_index": 0,
            "object_id": 1,
        },
        {
            "kind": "running",
            "from": {"x": 10.0, "y": 10.0},
            "to": {"x": 0.0, "y": 10.0},
            "color_index": 0,
            "object_id": 1,
        },
    ],
    "colors": [],
    "metadata": {},
}


def pyembroidery_square() -> bytes:
    """Call DstWriter.write directly. EmbPattern.write_dst may insert jumps."""

    pattern = EmbPattern()
    pattern.metadata("name", "Square")
    pattern.add_stitch_absolute(STITCH, 100, 0)
    pattern.add_stitch_absolute(STITCH, 100, 100)
    pattern.add_stitch_absolute(STITCH, 0, 100)
    pattern.add_stitch_absolute(END, 0, 100)
    buffer = io.BytesIO()
    write(pattern, buffer)
    return buffer.getvalue()


def test_compile_matches_pyembroidery_and_fixture():
    engine = Engine()
    assert engine.version() == "0.1.0"
    compiled = engine.compile_dst(SQUARE)
    oracle = pyembroidery_square()
    assert compiled == oracle
    assert FIXTURE.is_file(), "fixtures/dst/square_running.dst is missing"
    assert compiled == FIXTURE.read_bytes()
    assert len(compiled) == 524
    assert compiled.startswith(b"LA:Square")


def test_validate_empty_plan_names_the_next_step():
    engine = Engine()
    report = engine.validate(
        {"name": "Empty", "stitches": [], "colors": [], "metadata": {}}
    )
    assert report["ok"] is False
    assert report["errors"]
    assert "Generate" in report["errors"][0]


def test_bad_json_is_an_engine_error():
    engine = Engine()
    with pytest.raises(EngineError, match="stitch plan JSON"):
        engine.compile_dst({"stitches": "nope"})


def test_proposal_is_not_compiled():
    proposal = DigitisingProposal(
        regions=[
            RegionProposal(
                object_id=1,
                label="bar",
                stitch_type="fill",
                angle_degrees=45,
                density_mm=0.4,
                underlay=["lattice"],
                pull_compensation_mm=0.1,
                confidence=0.5,
            )
        ],
        color_sequence=[2],
        estimated_stitch_count=10,
        risk_flags=[],
    )
    engine = Engine()
    with pytest.raises(EngineError, match="digitising proposal"):
        engine.compile_dst(proposal.model_dump())
