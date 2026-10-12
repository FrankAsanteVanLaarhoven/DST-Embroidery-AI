#!/usr/bin/env python3
"""Strict native smoke for the two SVG geometry fixtures.

Exit 0 when both fixtures generate through the Rust FFI and an independent
DST reader accepts the bytes. Exit 2 when the native engine cannot be loaded.
Exit 1 when a fixture converts but generation or decoding fails.

This does not replace the repository regression suites, and it does not
accept A08, G02, or G03.
"""

from __future__ import annotations

import hashlib
import json
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))

from embroidery_ai.svg_geometry import SvgGeometryError, convert_svg, write_bundle

EVIDENCE = ROOT / "evidence" / "svg-native-gate.json"
FIXTURES = (
    ROOT / "fixtures" / "svg-geometry" / "badge-hole.svg",
    ROOT / "fixtures" / "svg-geometry" / "cubic-leaf.svg",
)


def main() -> int:
    record: dict = {
        "gate": "svg-geometry-native-smoke",
        "status": "BLOCKED",
        "cases_passed": 0,
        "cases_expected": len(FIXTURES),
        "python": sys.version,
        "platform": platform.platform(),
        "head": _head(),
        "converter_sha256": hashlib.sha256(
            (ROOT / "python" / "embroidery_ai" / "svg_geometry.py").read_bytes()
        ).hexdigest(),
        "note": (
            "Geometry intake only. Machine release stays false. "
            "A08, G02, and G03 are not accepted."
        ),
        "cases": [],
    }
    try:
        from embroidery_ai.engine import Engine, EngineError
    except Exception as error:  # noqa: BLE001 — the gate must record the real import failure
        record["error"] = f"embroidery_ai.engine could not be imported: {error}"
        _write(record)
        print(record["error"], file=sys.stderr)
        return 2
    try:
        engine = Engine()
    except Exception as error:  # noqa: BLE001 — a missing dylib is a blocked gate, not a pass
        record["error"] = f"The native engine is unavailable: {error}"
        _write(record)
        print(record["error"], file=sys.stderr)
        return 2
    try:
        import pyembroidery
    except Exception as error:  # noqa: BLE001
        record["status"] = "FAIL"
        record["error"] = f"The independent DST reader is unavailable: {error}"
        _write(record)
        print(record["error"], file=sys.stderr)
        return 1

    failures = []
    for path in FIXTURES:
        case = _run_case(path, engine, EngineError, pyembroidery)
        record["cases"].append(case)
        if case["status"] == "PASS":
            record["cases_passed"] += 1
        else:
            failures.append(case["name"] + ": " + case.get("error", "failed"))
    if failures:
        record["status"] = "FAIL"
        record["error"] = "; ".join(failures)
        _write(record)
        print(record["error"], file=sys.stderr)
        return 1
    record["status"] = "PASS"
    _write(record)
    print(
        f"PASS {record['cases_passed']} native SVG fixtures. "
        "Machine release remains false. This is not an A08 acceptance."
    )
    return 0


def _run_case(path: Path, engine, engine_error, pyembroidery) -> dict:
    raw = path.read_bytes()
    case = {
        "name": path.name,
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "status": "FAIL",
    }
    try:
        result = convert_svg(raw, name=path.stem)
        with tempfile.TemporaryDirectory() as temp:
            bundle = Path(temp) / "bundle"
            write_bundle(bundle, raw, result)
            reopened = json.loads((bundle / "project.json").read_text(encoding="utf-8"))
            if (bundle / "source.svg").read_bytes() != raw:
                raise SvgGeometryError("The saved source bytes differ from the input.")
        generation = engine.generate_project(reopened)
    except (SvgGeometryError, engine_error, OSError, ValueError) as error:
        case["error"] = str(error)
        return case
    readiness = generation.get("readiness") or {}
    validation = generation.get("validation") or {}
    if readiness.get("machine_release_allowed") is not False:
        case["error"] = "machine_release_allowed was not false"
        return case
    errors = validation.get("errors") or []
    if not readiness.get("engineering_export_allowed"):
        case["error"] = "engineering export was not allowed: " + "; ".join(errors)
        return case
    if errors:
        case["error"] = "validation failed: " + "; ".join(errors)
        return case
    dst_hex = generation.get("dst_hex") or ""
    if not isinstance(dst_hex, str) or not dst_hex.startswith("4c41"):
        case["error"] = "DST bytes do not start with the LA header"
        return case
    try:
        dst = bytes.fromhex(dst_hex)
    except ValueError as error:
        case["error"] = f"DST hex could not be read: {error}"
        return case
    with tempfile.TemporaryDirectory() as temp:
        dst_path = Path(temp) / "design.dst"
        dst_path.write_bytes(dst)
        try:
            pattern = pyembroidery.read(str(dst_path))
        except Exception as error:  # noqa: BLE001 — reader failure is a gate failure
            case["error"] = f"Independent DST reader failed: {error}"
            return case
    sewing = [
        item
        for item in pattern.stitches
        if len(item) >= 3 and item[2] == pyembroidery.STITCH
    ]
    if not sewing:
        case["error"] = "Independent DST reader found no sewing stitches"
        return case
    plan_sewing = sum(
        1
        for stitch in (generation.get("plan") or {}).get("stitches") or []
        if stitch.get("kind") in {"running", "satin", "fill", "motif", "applique"}
    )
    case.update(
        {
            "status": "PASS",
            "revision": generation.get("revision"),
            "dst_sha256": hashlib.sha256(dst).hexdigest(),
            "dst_bytes": len(dst),
            "plan_sewing_stitches": plan_sewing,
            "reader_sewing_stitches": len(sewing),
            "machine_release_allowed": False,
            "validation_warnings": validation.get("warnings") or [],
            "objects": len(reopened["objects"]),
            "holes": len(reopened["objects"][0].get("holes") or []),
        }
    )
    return case


def _head() -> str | None:
    try:
        completed = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError:
        return None
    if completed.returncode != 0:
        return None
    return completed.stdout.strip()


def _write(record: dict) -> None:
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
