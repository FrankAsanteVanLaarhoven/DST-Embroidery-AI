"""ctypes wrapper for ``libembroidery_ffi``.

The library compiles, validates, and scores a stitch-plan JSON document. A
:class:`~embroidery_ai.proposals.DigitisingProposal` is not a stitch plan.
Passing one here is an error, so a model cannot skip the generators.
"""

from __future__ import annotations

import ctypes
import json
import os
import sys
from pathlib import Path
from typing import Any, Mapping

_ROOT = Path(__file__).resolve().parents[2]


class EngineError(RuntimeError):
    """The engine rejected a call. The message says what failed and what to do."""


def library_path() -> Path:
    """Resolve the cdylib. ``EMBROIDERY_FFI`` overrides the search."""

    override = os.environ.get("EMBROIDERY_FFI")
    if override:
        path = Path(override)
        if not path.is_file():
            raise EngineError(
                f"EMBROIDERY_FFI points at {path}, which is not a file. "
                "Build the engine with `cargo build -p embroidery-ffi` and set the variable to that library."
            )
        return path

    names = ("libembroidery_ffi.dylib", "libembroidery_ffi.so", "embroidery_ffi.dll")
    folders = (
        _ROOT / "target" / "debug",
        _ROOT / "target" / "release",
        _ROOT / "target" / "debug" / "deps",
        _ROOT / "target" / "release" / "deps",
    )
    for folder in folders:
        for name in names:
            candidate = folder / name
            if candidate.is_file():
                return candidate
    raise EngineError(
        "Could not find libembroidery_ffi. From the repository root run "
        "`cargo build -p embroidery-ffi`, or set EMBROIDERY_FFI to the library path."
    )


class Engine:
    """One loaded copy of the C ABI."""

    def __init__(self, path: Path | None = None) -> None:
        self.path = path or library_path()
        self._lib = ctypes.CDLL(str(self.path))
        self._bind()

    def version(self) -> str:
        raw = self._lib.embroidery_engine_version()
        if raw is None:
            raise EngineError(
                "The engine returned no version string. Rebuild embroidery-ffi and load that library."
            )
        return raw.decode("utf-8")

    def compile_dst(self, plan: Mapping[str, Any]) -> bytes:
        """Compile a stitch plan to DST bytes. Does not accept a region proposal."""

        self._require_stitch_plan(plan)
        return self._call(self._lib.embroidery_compile_dst_json, plan)

    def validate(self, plan: Mapping[str, Any]) -> dict[str, Any]:
        """Return ``{"ok", "errors", "warnings"}`` for a stitch plan."""

        self._require_stitch_plan(plan)
        report = json.loads(self._call(self._lib.embroidery_validate_json, plan))
        if not isinstance(report, dict):
            raise EngineError(
                "The engine returned a validation report that is not an object. Rebuild embroidery-ffi."
            )
        return report

    def generate_project(self, project: Mapping[str, Any]) -> dict[str, Any]:
        """Generate a stitch plan from an embroidery-project document.

        This calls the Rust generators. A digitising proposal is not a project:
        it has no geometry. Motif and applique are rejected by the engine.
        """

        if project.get("format") != "embroidery-project":
            raise EngineError(
                "generate_project expects an embroidery-project document. "
                "A digitising proposal has no geometry. Build the project, then generate."
            )
        report = json.loads(self._call(self._lib.embroidery_generate_project_json, project))
        if not isinstance(report, dict):
            raise EngineError(
                "The engine returned a generation result that is not an object. Rebuild embroidery-ffi."
            )
        return report

    def capabilities(self) -> dict[str, Any]:
        """Return the engine capability manifest."""

        report = json.loads(self._call_out(self._lib.embroidery_capabilities_json))
        if not isinstance(report, dict):
            raise EngineError(
                "The engine returned a capability manifest that is not an object. Rebuild embroidery-ffi."
            )
        return report

    def score(self, plan: Mapping[str, Any]) -> dict[str, Any]:
        """Return the production-risk report for a stitch plan."""

        self._require_stitch_plan(plan)
        report = json.loads(self._call(self._lib.embroidery_score_json, plan))
        if not isinstance(report, dict):
            raise EngineError(
                "The engine returned a risk report that is not an object. Rebuild embroidery-ffi."
            )
        return report

    def _bind(self) -> None:
        pointer = ctypes.POINTER(ctypes.c_uint8)
        self._lib.embroidery_engine_version.argtypes = []
        self._lib.embroidery_engine_version.restype = ctypes.c_char_p

        for name in (
            "embroidery_compile_dst_json",
            "embroidery_validate_json",
            "embroidery_score_json",
            "embroidery_generate_project_json",
        ):
            function = getattr(self._lib, name)
            function.argtypes = [
                ctypes.c_char_p,
                ctypes.POINTER(pointer),
                ctypes.POINTER(ctypes.c_size_t),
                ctypes.POINTER(ctypes.c_size_t),
            ]
            function.restype = ctypes.c_int

        self._lib.embroidery_capabilities_json.argtypes = [
            ctypes.POINTER(pointer),
            ctypes.POINTER(ctypes.c_size_t),
            ctypes.POINTER(ctypes.c_size_t),
        ]
        self._lib.embroidery_capabilities_json.restype = ctypes.c_int

        self._lib.embroidery_free.argtypes = [
            pointer,
            ctypes.c_size_t,
            ctypes.c_size_t,
        ]
        self._lib.embroidery_free.restype = None

    def _require_stitch_plan(self, plan: Mapping[str, Any]) -> None:
        if "stitches" not in plan:
            raise EngineError(
                "This call expects a stitch plan with a stitches array. "
                "A digitising proposal only has regions. Run the Rust generators, then compile the plan they return."
            )

    def _call_out(self, function: Any) -> bytes:
        out_ptr = ctypes.POINTER(ctypes.c_uint8)()
        out_len = ctypes.c_size_t()
        out_cap = ctypes.c_size_t()
        code = function(ctypes.byref(out_ptr), ctypes.byref(out_len), ctypes.byref(out_cap))
        return self._finish(code, out_ptr, out_len, out_cap)

    def _call(self, function: Any, plan: Mapping[str, Any]) -> bytes:
        try:
            payload = json.dumps(plan).encode("utf-8")
        except (TypeError, ValueError) as error:
            raise EngineError(
                f"Could not encode the stitch plan as JSON ({error}). Pass a plan made of strings, numbers, and lists."
            ) from error

        out_ptr = ctypes.POINTER(ctypes.c_uint8)()
        out_len = ctypes.c_size_t()
        out_cap = ctypes.c_size_t()
        code = function(
            payload,
            ctypes.byref(out_ptr),
            ctypes.byref(out_len),
            ctypes.byref(out_cap),
        )
        return self._finish(code, out_ptr, out_len, out_cap)

    def _finish(self, code: int, out_ptr: Any, out_len: Any, out_cap: Any) -> bytes:
        data = b""
        if out_ptr:
            data = ctypes.string_at(out_ptr, out_len.value)
            self._lib.embroidery_free(out_ptr, out_len, out_cap)

        if code == 0:
            return data
        if code == 1:
            message = data.decode("utf-8", errors="replace")
            raise EngineError(message or "The engine rejected the plan and returned no message.")
        if code == 2:
            raise EngineError(
                "The engine rejected a null argument. Pass UTF-8 JSON."
            )
        raise EngineError(
            f"The engine returned status {code}. Rebuild embroidery-ffi and retry the call."
        )


def _main() -> None:
    """Write one square DST when invoked as a script. Used to refresh the fixture."""

    if len(sys.argv) != 2:
        raise SystemExit("usage: python -m embroidery_ai.engine FIXTURE.dst")
    plan = {
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
    Path(sys.argv[1]).write_bytes(Engine().compile_dst(plan))


if __name__ == "__main__":
    _main()
