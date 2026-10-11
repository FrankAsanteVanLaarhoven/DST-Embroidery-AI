"""Structured digitising proposals.

A proposal names regions, stitch type, and parameters. It has no stitch
coordinates and no DST bytes. The Rust generators are the only supported way
to turn a proposal into a stitch plan.
"""

from __future__ import annotations

import math
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator

StitchType = Literal["running", "satin", "fill", "motif", "applique"]

# Underlay names the generators understand today, plus the ones a later
# production satin/fill pass is expected to add. Unknown names are rejected
# so a model cannot invent a stitch command by putting it in this list.
UnderlayName = Literal["centre_run", "edge_run", "zigzag", "lattice"]


class RegionProposal(BaseModel):
    """One region a model wants the deterministic generators to sew."""

    model_config = ConfigDict(extra="forbid")

    object_id: int = Field(ge=0)
    label: str = Field(min_length=1, max_length=80)
    stitch_type: StitchType
    angle_degrees: float
    density_mm: float = Field(gt=0, le=12.1)
    underlay: list[UnderlayName]
    pull_compensation_mm: float = Field(ge=0, le=2.0)
    confidence: float = Field(ge=0, le=1)

    @field_validator("angle_degrees", "density_mm", "pull_compensation_mm", "confidence")
    @classmethod
    def _finite(cls, value: float) -> float:
        if not math.isfinite(value):
            raise ValueError("must be a finite number")
        return value


class DigitisingProposal(BaseModel):
    """The only document an agent may emit before a human approves generation.

    ``estimated_stitch_count`` is the model's guess. The compiler's stitch
    count is the value in the DST ``ST`` field after the Rust generators run.
    """

    model_config = ConfigDict(extra="forbid")

    regions: list[RegionProposal]
    color_sequence: list[int] = Field(min_length=1)
    estimated_stitch_count: int = Field(ge=0)
    risk_flags: list[str]

    @field_validator("color_sequence")
    @classmethod
    def _color_indexes(cls, value: list[int]) -> list[int]:
        if any(index < 0 for index in value):
            raise ValueError("color indexes must be zero or positive")
        return value

    @field_validator("risk_flags")
    @classmethod
    def _flag_text(cls, value: list[str]) -> list[str]:
        if any(not flag.strip() for flag in value):
            raise ValueError("risk flags must be non-empty text")
        return value
