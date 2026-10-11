"""Conservative planning policy, not a hardware probe or compatibility certificate.

The caller supplies measured facts and an independently reviewed lifecycle status.
This module never enables machine actuation, production release or AI inference.
Python 3.11+; this developer tool itself is NOT a legacy Windows executable.
"""
from __future__ import annotations
import math
from dataclasses import dataclass
from typing import Literal

Lifecycle = Literal['maintained', 'unmaintained', 'unknown']
@dataclass(frozen=True)
class Probe:
    os_name: str
    architecture: str
    ram_gib: float
    lifecycle: Lifecycle = 'unknown'
    modern_runtime: bool = False
    canvas_2d: bool = False
    webgl2_context: bool = False
    webgl2_smoke_passed: bool = False
    webgpu_adapter: bool = False
    remote_worker_available: bool = False


def require_positive(value: float, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value <= 0:
        raise ValueError(f'{name} must be positive and finite')
    return float(value)


def mm_per_pixel(dpi: float) -> float:
    """Nominal sampling pitch only; does not measure optical/physical accuracy."""
    return 25.4 / require_positive(dpi, 'dpi')


def effective_dpi(scan_dpi: float, enlargement: float) -> float:
    return require_positive(scan_dpi, 'scan_dpi') / require_positive(enlargement, 'enlargement')


def pixels_to_mm(pixels: float, dpi: float) -> float:
    return require_positive(pixels, 'pixels') * mm_per_pixel(dpi)


def choose_mode(probe: Probe) -> dict:
    """Select a proposed route. Every route remains unqualified until tested."""
    require_positive(probe.ram_gib, 'ram_gib')
    if probe.lifecycle not in ('maintained', 'unmaintained', 'unknown'):
        raise ValueError('invalid lifecycle')
    if not probe.os_name.strip() or not probe.architecture.strip():
        raise ValueError('named OS and architecture are required')
    common = {'qualification': 'unqualified_candidate', 'local_ai_enabled': False,
              'machine_control_enabled': False, 'production_release_enabled': False}
    if probe.lifecycle == 'unmaintained':
        mode, reason = 'isolated_capture_gateway', 'No direct cloud session on this legacy host. Use approved file exchange with a maintained gateway.'
    elif probe.lifecycle == 'unknown':
        mode, reason = 'manual_assessment', 'OS/security lifecycle not established. Do not infer it from a brand or user-agent.'
    elif not probe.modern_runtime or not probe.canvas_2d or probe.ram_gib < 2:
        mode, reason = 'gateway_or_static_review', 'Interactive runtime or resource prerequisite absent; static PDF/raster review or a qualified helper is required.'
    elif probe.ram_gib >= 8 and probe.webgl2_context and probe.webgl2_smoke_passed:
        mode, reason = 'interactive_3d_candidate', 'Meets proposed 3D entry criteria; scene-size and sustained performance tests still required.'
    else:
        mode, reason = 'lightweight_2d_candidate', '2D editing/review candidate. 3D is not mandatory and unavailable compute is not silently substituted.'
    return dict(common, mode=mode, reason=reason,
                compute='remote_candidate' if probe.remote_worker_available else 'local_qualified_engine_or_unavailable',
                webgpu_optional=probe.webgpu_adapter)


def format_policy(extension: str) -> dict:
    """A declared route is not a parser. Actual file content must also be validated."""
    ext=extension.lower().lstrip('.')
    if ext == 'dts':
        return {'route':'unrecognised', 'action':'possible_dst_typo_requires_content_validation', 'rename_automatically':False}
    if ext == 'dst':
        return {'route':'qualified_rust_dst_adapter', 'source_editability':'stitch_level_only', 'rename_automatically':False}
    if ext == 'pdf':
        return {'route':'source_document_or_derived_proof', 'machine_executable':False, 'rename_automatically':False}
    return {'route':'adapter_required', 'qualification':'unknown', 'rename_automatically':False}
