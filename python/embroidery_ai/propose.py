"""Read one proposal request on stdin and write one JSON object on stdout.

The studio passes the prompt and optional image on stdin. ``XAI_API_KEY`` is
read from the environment, never from the command line. A ``geometry`` field
normalizes a design without calling the network, for tests.
"""

from __future__ import annotations

import json
import sys
from typing import Any

from embroidery_ai.geometry import GeometryError, normalize_geometry, request_geometry


def main() -> int:
    try:
        raw = json.load(sys.stdin)
    except json.JSONDecodeError:
        _emit(
            ok=False,
            error="The proposal request was not JSON. Ask again. The design was not changed.",
        )
        return 0
    if not isinstance(raw, dict):
        _emit(
            ok=False,
            error="The proposal request must be an object. Ask again. The design was not changed.",
        )
        return 0
    try:
        project = _project(raw)
    except GeometryError as error:
        _emit(ok=False, error=str(error))
        return 0
    except Exception:
        _emit(
            ok=False,
            error="The proposal helper failed before a design was returned. Ask again. The design was not changed.",
        )
        return 1
    _emit(ok=True, project=project)
    return 0


def _project(raw: dict[str, Any]) -> dict[str, Any]:
    if "geometry" in raw and "prompt" not in raw and "image_data_url" not in raw:
        return normalize_geometry(raw["geometry"])
    prompt = raw.get("prompt") if isinstance(raw.get("prompt"), str) else ""
    image = raw.get("image_data_url") if isinstance(raw.get("image_data_url"), str) else ""
    return request_geometry(prompt, image or None)


def _emit(*, ok: bool, project: dict[str, Any] | None = None, error: str | None = None) -> None:
    body: dict[str, Any] = {"ok": ok}
    if project is not None:
        body["project"] = project
    if error is not None:
        body["error"] = error
    sys.stdout.write(json.dumps(body))


if __name__ == "__main__":
    raise SystemExit(main())
