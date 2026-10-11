"""The agent boundary accepts parameters and rejects stitch records."""

import pytest
from pydantic import ValidationError

from embroidery_ai import DigitisingProposal, RegionProposal


def region(**overrides):
    data = {
        "object_id": 1,
        "label": "letter A",
        "stitch_type": "satin",
        "angle_degrees": 0.0,
        "density_mm": 0.4,
        "underlay": ["centre_run", "edge_run"],
        "pull_compensation_mm": 0.15,
        "confidence": 0.8,
    }
    data.update(overrides)
    return data


def proposal(**overrides):
    data = {
        "regions": [region()],
        "color_sequence": [0],
        "estimated_stitch_count": 400,
        "risk_flags": [],
    }
    data.update(overrides)
    return data


def test_proposal_round_trips_without_stitches():
    parsed = DigitisingProposal.model_validate(proposal())
    dumped = parsed.model_dump()
    assert "stitches" not in dumped
    assert "stitches" not in RegionProposal.model_fields
    assert dumped["regions"][0]["stitch_type"] == "satin"


def test_extra_stitch_records_are_rejected():
    with pytest.raises(ValidationError):
        DigitisingProposal.model_validate(proposal(stitches=[{"kind": "running"}]))
    with pytest.raises(ValidationError):
        RegionProposal.model_validate(region(from_x=0, to_x=10))


@pytest.mark.parametrize(
    "overrides",
    [
        {"confidence": 1.1},
        {"confidence": -0.01},
        {"density_mm": 0},
        {"density_mm": -1},
        {"pull_compensation_mm": -0.1},
        {"stitch_type": "jump"},
        {"underlay": ["secret_command"]},
        {"angle_degrees": float("nan")},
    ],
)
def test_parameter_bounds(overrides):
    with pytest.raises(ValidationError):
        RegionProposal.model_validate(region(**overrides))


def test_color_sequence_and_flags():
    with pytest.raises(ValidationError):
        DigitisingProposal.model_validate(proposal(color_sequence=[-1]))
    with pytest.raises(ValidationError):
        DigitisingProposal.model_validate(proposal(color_sequence=[]))
    with pytest.raises(ValidationError):
        DigitisingProposal.model_validate(proposal(risk_flags=[" "]))
