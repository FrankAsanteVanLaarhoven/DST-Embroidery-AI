#!/usr/bin/env python3
"""Write a draft native project bundle from one SVG file.

The bundle contains the original bytes, an embroidery-project version 1
document, and an import manifest. It does not generate stitches or DST.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))

from embroidery_ai.svg_geometry import SvgGeometryError, convert_svg, write_bundle


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Convert a bounded SVG into a draft embroidery project. No stitches are generated."
    )
    parser.add_argument("svg", type=Path, help="Source SVG file")
    parser.add_argument("dest", type=Path, help="New or empty output directory")
    parser.add_argument("--width-mm", type=float, default=None, help="Physical width for a pixel artboard")
    parser.add_argument(
        "--running-strokes",
        action="store_true",
        help="Interpret stroke-only paths as running centrelines",
    )
    parser.add_argument("--name", default=None, help="Native project name")
    parser.add_argument("--tolerance-mm", type=float, default=0.05, help="Curve flattening tolerance")
    args = parser.parse_args(argv)
    try:
        source = args.svg.read_bytes()
        result = convert_svg(
            source,
            width_mm=args.width_mm,
            running_strokes=args.running_strokes,
            name=args.name or args.svg.stem,
            flatten_tolerance_mm=args.tolerance_mm,
        )
        write_bundle(args.dest, source, result)
    except (OSError, SvgGeometryError) as error:
        print(str(error), file=sys.stderr)
        return 2
    print(
        f"Wrote {args.dest / 'source.svg'}, {args.dest / 'project.json'}, "
        f"and {args.dest / 'import-manifest.json'}. No stitches were generated."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
