"""Project generation goes through Rust. A proposal is still not a stitch plan."""

import json
from pathlib import Path

import pytest

from embroidery_ai import DigitisingProposal, Engine, EngineError, RegionProposal

ROOT = Path(__file__).resolve().parents[2]
LINE = json.loads((ROOT / "fixtures/projects/01_running_line.json").read_text())
MOTIF = json.loads((ROOT / "fixtures/projects/09_invalid_motif.json").read_text())


def test_capabilities_mark_motif_as_planned():
    manifest = Engine().capabilities()
    motif = next(item for item in manifest["capabilities"] if item["id"] == "motif")
    assert motif["status"] == "planned"
    release = next(item for item in manifest["capabilities"] if item["id"] == "machine_release")
    assert release["status"] == "planned"


def test_running_project_exports_dst_without_machine_release():
    result = Engine().generate_project(LINE)
    assert result["readiness"]["engineering_export_allowed"] is True
    assert result["readiness"]["machine_release_allowed"] is False
    assert result["dst_hex"].startswith("4c41")
    assert result["provenance"]["generators"] == ["running"]


def test_motif_project_is_rejected_even_though_a_proposal_may_name_it():
    DigitisingProposal(
        regions=[
            RegionProposal(
                object_id=1,
                label="mark",
                stitch_type="motif",
                angle_degrees=0,
                density_mm=0.4,
                underlay=[],
                pull_compensation_mm=0,
                confidence=0.2,
            )
        ],
        color_sequence=[0],
        estimated_stitch_count=1,
        risk_flags=["motif has no generator"],
    )
    with pytest.raises(EngineError, match="unavailable"):
        Engine().generate_project(MOTIF)


def test_a_proposal_is_not_a_project():
    with pytest.raises(EngineError, match="digitising proposal"):
        Engine().generate_project({"regions": []})
