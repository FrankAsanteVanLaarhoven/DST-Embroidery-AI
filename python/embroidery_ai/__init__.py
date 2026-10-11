"""Research boundary for the embroidery engine.

Models and agents emit a :class:`DigitisingProposal`. They do not emit stitch
records or DST bytes. The Rust generators turn an approved proposal into a
stitch plan, and :class:`Engine` compiles that plan.
"""

from embroidery_ai.engine import Engine, EngineError
from embroidery_ai.proposals import DigitisingProposal, RegionProposal, StitchType

__all__ = [
    "DigitisingProposal",
    "Engine",
    "EngineError",
    "RegionProposal",
    "StitchType",
]
