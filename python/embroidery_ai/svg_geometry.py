"""Bounded SVG geometry intake for an embroidery-project version 1 draft.

This module stores the original bytes, flattens a restricted vector subset,
and writes editable native geometry. It does not generate stitches, compile
DST, call a model, or authorise a machine. Curves become polylines. The
native project has no Bézier handles, source RGB, or group fields; those
facts stay in the import manifest.

The centred TwinLab document is a different shape. This converter does not
claim that a native hoop file can be loaded as that document.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Iterator
from xml.etree import ElementTree

SVG_NS = "http://www.w3.org/2000/svg"
MM_PER_PX = 25.4 / 96.0
MM_PER_UNIT = {
    "mm": 1.0,
    "cm": 10.0,
    "in": 25.4,
    "pt": 25.4 / 72.0,
    "pc": 25.4 / 6.0,
    "q": 0.25,
}
ABSOLUTE_UNITS = frozenset(MM_PER_UNIT)
KAPPA = 0.5522847498307936
MAX_BYTES = 400_000
MAX_ELEMENTS = 400
MAX_PATH_CHARS = 100_000
MAX_COMMANDS = 8_000
MAX_OBJECTS = 64
MAX_SUBPATHS = 64
MAX_RING_POINTS = 4_096
MAX_GROUP_DEPTH = 16
MAX_NAME = 80
DEFAULT_TOLERANCE_MM = 0.05
FILL_DENSITY_MM = 1.2
FILL_MAX_STITCH_MM = 3.0
RUNNING_MAX_STITCH_MM = 3.0
COLUMN_WIDTH_MM = 2.0
TOUCH_EPS_MM = 1e-5
AREA_EPS = 1e-4
UNCHANGED = "The design was not changed."

ALIGNS = frozenset(
    {
        "xMinYMin",
        "xMidYMin",
        "xMaxYMin",
        "xMinYMid",
        "xMidYMid",
        "xMaxYMid",
        "xMinYMax",
        "xMidYMax",
        "xMaxYMax",
    }
)
SKIP_ELEMENTS = frozenset({"title", "desc", "metadata"})
NAMED_COLOURS = {
    "black": "#000000",
    "white": "#ffffff",
    "red": "#ff0000",
    "green": "#008000",
    "blue": "#0000ff",
    "yellow": "#ffff00",
    "cyan": "#00ffff",
    "aqua": "#00ffff",
    "magenta": "#ff00ff",
    "fuchsia": "#ff00ff",
    "gray": "#808080",
    "grey": "#808080",
    "orange": "#ffa500",
    "purple": "#800080",
    "navy": "#000080",
    "teal": "#008080",
    "silver": "#c0c0c0",
    "maroon": "#800000",
    "lime": "#00ff00",
    "olive": "#808000",
}
SPECIFIC_ELEMENTS = {
    "text": "This SVG contains text. Convert the text to paths and import again.",
    "tspan": "This SVG contains text. Convert the text to paths and import again.",
    "textpath": "This SVG contains text. Convert the text to paths and import again.",
    "image": "This SVG embeds an image. Replace it with paths and import again.",
    "use": "SVG use references are not converted. Expand the graphic and import again.",
    "script": "SVG script is not converted. Remove it and import again.",
    "style": "SVG style sheets are not converted. Set fill and stroke attributes and import again.",
    "lineargradient": "Gradients are not converted. Use a solid fill and import again.",
    "radialgradient": "Gradients are not converted. Use a solid fill and import again.",
    "pattern": "Paint patterns are not converted. Use a solid fill and import again.",
    "clippath": "Clipping paths are not converted. Apply the clip in the source drawing and import again.",
    "mask": "Masks are not converted. Apply the mask in the source drawing and import again.",
    "filter": "Filters are not converted. Remove the filter and import again.",
    "foreignobject": "Foreign objects are not converted. Remove them and import again.",
    "a": "Links are not converted. Remove the link and import the shapes again.",
    "symbol": "SVG symbols are not converted. Expand the graphic and import again.",
    "marker": "Markers are not converted. Remove them and import again.",
    "switch": "Conditional SVG switch elements are not converted. Remove them and import again.",
    "animate": "Animation is not converted. Remove it and import again.",
    "animatetransform": "Animation is not converted. Remove it and import again.",
    "set": "Animation is not converted. Remove it and import again.",
}
_LENGTH = re.compile(
    r"^([+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)([a-z%]*)$",
    re.IGNORECASE,
)
_TRANSFORM = re.compile(r"([A-Za-z]+)\s*\(([^)]*)\)")
_NUMBER = re.compile(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?")


class SvgGeometryError(ValueError):
    """The SVG is outside the bounded subset or is not safe to convert."""


@dataclass(frozen=True)
class Matrix:
    """Affine map x' = a x + c y + e, y' = b x + d y + f."""

    a: float = 1.0
    b: float = 0.0
    c: float = 0.0
    d: float = 1.0
    e: float = 0.0
    f: float = 0.0

    def multiply(self, other: Matrix) -> Matrix:
        """Return self ∘ other. ``other`` is applied to the point first."""

        return Matrix(
            self.a * other.a + self.c * other.b,
            self.b * other.a + self.d * other.b,
            self.a * other.c + self.c * other.d,
            self.b * other.c + self.d * other.d,
            self.a * other.e + self.c * other.f + self.e,
            self.b * other.e + self.d * other.f + self.f,
        )

    def apply(self, x: float, y: float) -> tuple[float, float]:
        return (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )


@dataclass(frozen=True)
class Paint:
    fill: str | None = "#000000"
    stroke: str | None = None
    fill_rule: str = "nonzero"
    stroke_width: str | None = None


@dataclass
class Frame:
    width_mm: float
    height_mm: float
    matrix: Matrix
    width_source: str
    view_box: list[float] | None
    preserve_aspect_ratio: str


def convert_svg(
    source: bytes | str,
    *,
    width_mm: float | None = None,
    running_strokes: bool = False,
    name: str | None = None,
    flatten_tolerance_mm: float = DEFAULT_TOLERANCE_MM,
) -> dict:
    """Convert SVG bytes into a project, a manifest, and explicit losses."""

    raw = _source_bytes(source)
    _reject_dangerous_markup(raw)
    if not math.isfinite(flatten_tolerance_mm) or not 0.01 <= flatten_tolerance_mm <= 1.0:
        raise SvgGeometryError(
            "The flattening tolerance must be from 0.01 mm to 1 mm. "
            f"Choose a tolerance in that range and import again. {UNCHANGED}"
        )
    if width_mm is not None and (
        not isinstance(width_mm, (int, float))
        or isinstance(width_mm, bool)
        or not math.isfinite(float(width_mm))
        or float(width_mm) <= 0
        or float(width_mm) > 10_000
    ):
        raise SvgGeometryError(
            "Artboard width must be a positive number of millimetres up to 10000. "
            f"Enter the physical width and import again. {UNCHANGED}"
        )
    try:
        root = ElementTree.fromstring(raw.decode("utf-8").lstrip("\ufeff"))
    except ElementTree.ParseError as error:
        raise SvgGeometryError(
            "This SVG is not well-formed XML. Repair the file and import again. "
            f"{UNCHANGED}"
        ) from error
    if _local_name(root) != "svg":
        raise SvgGeometryError(
            "The file is not an SVG document. Choose an SVG file and import again. "
            f"{UNCHANGED}"
        )
    frame = _frame(root, None if width_mm is None else float(width_mm))
    converter = _Converter(
        frame=frame,
        running_strokes=running_strokes,
        tolerance=float(flatten_tolerance_mm),
    )
    converter.convert(root)
    project_name = _project_name(name, _attr(root, "id"))
    project = {
        "format": "embroidery-project",
        "format_version": "1",
        "name": project_name,
        "hoop_width_mm": _q(frame.width_mm),
        "hoop_height_mm": _q(frame.height_mm),
        "objects": converter.objects,
        "review": None,
    }
    losses = _document_losses(converter)
    manifest = {
        "kind": "embroideryos-svg-import-manifest",
        "version": "1",
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "source_byte_length": len(raw),
        "artboard": {
            "width_mm": _q(frame.width_mm),
            "height_mm": _q(frame.height_mm),
            "width_source": frame.width_source,
            "view_box": frame.view_box,
            "preserve_aspect_ratio": frame.preserve_aspect_ratio,
            "flatten_tolerance_mm": float(flatten_tolerance_mm),
        },
        "coordinate_frame": "native-hoop-nonnegative-v1",
        "not_interchangeable_with": "twinlab-generic-flat-120-v1",
        "timing_model": None,
        "machine_release_allowed": False,
        "approval_allowed": False,
        "review": None,
        "palette": [
            {"color_index": index, "source": colour}
            for index, colour in enumerate(converter.palette)
        ],
        "objects": converter.records,
        "losses": losses,
        "generator": "embroidery_ai.svg_geometry",
        "notes": [
            "This bundle is a draft geometry import. It contains no stitches and no DST.",
            "A08, G02, and G03 are not accepted by this conversion.",
        ],
    }
    return {"project": project, "manifest": manifest, "losses": losses}


def write_bundle(destination: Path, source: bytes, result: dict) -> None:
    """Write source.svg, project.json, and import-manifest.json into a new directory."""

    destination = Path(destination)
    if destination.exists():
        if not destination.is_dir():
            raise SvgGeometryError(
                f"{destination} is not a directory. Choose a new directory. {UNCHANGED}"
            )
        if any(destination.iterdir()):
            raise SvgGeometryError(
                f"{destination} is not empty. Choose a new directory so the original "
                f"bundle is not overwritten. {UNCHANGED}"
            )
    else:
        destination.mkdir(parents=True)
    project_path = destination / "project.json"
    manifest_path = destination / "import-manifest.json"
    source_path = destination / "source.svg"
    source_path.write_bytes(source)
    project_path.write_text(_dump(result["project"]), encoding="utf-8")
    manifest_path.write_text(_dump(result["manifest"]), encoding="utf-8")


def main() -> int:
    """Read one JSON request from stdin and write one JSON object to stdout."""

    try:
        raw = json.load(sys.stdin)
    except json.JSONDecodeError:
        _emit(ok=False, error=f"The SVG request was not JSON. Send the file again. {UNCHANGED}")
        return 0
    if not isinstance(raw, dict):
        _emit(ok=False, error=f"The SVG request must be an object. Send the file again. {UNCHANGED}")
        return 0
    svg_text = raw.get("svg_text", "")
    if not isinstance(svg_text, str) or not svg_text.strip():
        _emit(
            ok=False,
            error=f"Choose an SVG file, then preview the import. {UNCHANGED}",
        )
        return 0
    try:
        result = convert_svg(
            svg_text,
            width_mm=_optional_width(raw.get("width_mm")),
            running_strokes=bool(raw.get("running_strokes", False)),
            name=raw.get("name") if isinstance(raw.get("name"), str) else None,
            flatten_tolerance_mm=_optional_tolerance(raw.get("flatten_tolerance_mm")),
        )
    except SvgGeometryError as error:
        _emit(ok=False, error=str(error))
        return 0
    except Exception:
        _emit(
            ok=False,
            error=f"The SVG could not be converted. Repair the file and import again. {UNCHANGED}",
        )
        return 0
    _emit(ok=True, **result)
    return 0


class _Converter:
    def __init__(self, frame: Frame, running_strokes: bool, tolerance: float) -> None:
        self.frame = frame
        self.running_strokes = running_strokes
        self.tolerance = tolerance
        self.objects: list[dict] = []
        self.records: list[dict] = []
        self.palette: list[str] = []
        self.losses: list[str] = []
        self._loss_keys: set[str] = set()
        self.elements = 0
        self.commands = 0
        self.curved = False
        self.anonymous_groups = 0
        self.fill_count = 0
        self.running_count = 0

    def convert(self, root: ElementTree.Element) -> None:
        self._reject_attributes(root)
        self.elements = 1
        root_transform = _parse_transform(_attr(root, "transform"))
        base = root_transform.multiply(self.frame.matrix)
        paint = Paint().with_element(root)
        for child in list(root):
            self._walk(child, base, paint, [], 1)
        if not self.objects:
            raise SvgGeometryError(
                "This SVG has no paintable geometry in the supported subset. "
                f"Add a shape or path and import again. {UNCHANGED}"
            )

    def _walk(
        self,
        element: ElementTree.Element,
        ctm: Matrix,
        paint: Paint,
        ancestry: list[str],
        depth: int,
    ) -> None:
        if depth > MAX_GROUP_DEPTH:
            raise SvgGeometryError(
                "This SVG nests groups more than 16 deep. Flatten the grouping and "
                f"import again. {UNCHANGED}"
            )
        self.elements += 1
        if self.elements > MAX_ELEMENTS:
            raise SvgGeometryError(
                f"This SVG has more than {MAX_ELEMENTS} elements. Simplify it and "
                f"import again. {UNCHANGED}"
            )
        name = _local_name(element)
        if name in SKIP_ELEMENTS:
            return
        if name == "defs":
            if list(element):
                self._loss(
                    "defs",
                    "Definitions were not painted. Symbols, gradients, and reuse are not converted.",
                )
            return
        if name == "svg":
            raise SvgGeometryError(
                "Nested svg viewports are not converted. Expand the drawing to one "
                f"viewport and import again. {UNCHANGED}"
            )
        specific = SPECIFIC_ELEMENTS.get(name)
        if specific:
            raise SvgGeometryError(f"{specific} {UNCHANGED}")
        self._reject_attributes(element)
        if name == "g":
            group_id = self._group_id(element)
            local = _parse_transform(_attr(element, "transform"))
            child_ctm = ctm.multiply(local)
            child_paint = paint.with_element(element)
            child_ancestry = [*ancestry, group_id]
            for child in list(element):
                self._walk(child, child_ctm, child_paint, child_ancestry, depth + 1)
            return
        if name not in {"rect", "circle", "ellipse", "line", "polyline", "polygon", "path"}:
            raise SvgGeometryError(
                f"The element {name} is not in the supported SVG subset. Remove it "
                f"and import again. {UNCHANGED}"
            )
        local = _parse_transform(_attr(element, "transform"))
        shape_ctm = ctm.multiply(local)
        shape_paint = paint.with_element(element)
        self._emit(element, name, shape_ctm, shape_paint, ancestry)

    def _group_id(self, element: ElementTree.Element) -> str:
        explicit = _attr(element, "id")
        if explicit:
            return _clean_token(explicit)
        self.anonymous_groups += 1
        return f"anonymous-group-{self.anonymous_groups}"

    def _emit(
        self,
        element: ElementTree.Element,
        name: str,
        ctm: Matrix,
        paint: Paint,
        ancestry: list[str],
    ) -> None:
        if name != "line" and paint.fill is None and paint.stroke is None:
            self._loss(
                "unpainted",
                "An element with no fill and no stroke was skipped.",
            )
            return
        subpaths = self._subpaths(element, name, ctm)
        if not subpaths:
            raise SvgGeometryError(
                f"A {name} element did not produce geometry. Repair that element and "
                f"import again. {UNCHANGED}"
            )
        if name == "polyline" and paint.fill is not None:
            for item in subpaths:
                item["closed"] = True
        if name == "line":
            self._emit_running(element, name, ctm, paint, ancestry, subpaths)
            return
        filled = paint.fill is not None
        if filled and any(not item["closed"] for item in subpaths):
            raise SvgGeometryError(
                "A filled path contains an open subpath. Close it, or split the stroke "
                f"into its own element, and import again. {UNCHANGED}"
            )
        if paint.stroke is not None:
            self._loss(
                "stroke-width",
                "Source stroke width, caps, joins, and dashes are not embroidery "
                "columns. The outline or the centreline was kept.",
            )
        if filled:
            curved = any(item["curved"] for item in subpaths)
            rings = [item["points"] for item in subpaths]
            regions = _regions(rings, paint.fill_rule)
            for region in regions:
                region["curved"] = curved
                self._add_fill(element, name, ancestry, paint, region)
            return
        if not self.running_strokes:
            raise SvgGeometryError(
                "Stroke-only artwork needs an explicit centreline interpretation. "
                "Turn on running centrelines and import again. "
                f"{UNCHANGED}"
            )
        self._emit_running(element, name, ctm, paint, ancestry, subpaths)

    def _emit_running(
        self,
        element: ElementTree.Element,
        name: str,
        ctm: Matrix,
        paint: Paint,
        ancestry: list[str],
        subpaths: list[dict],
    ) -> None:
        del ctm
        if paint.stroke is None and name == "line":
            raise SvgGeometryError(
                "A line has no area to fill. Set a stroke, turn on running centrelines, "
                f"and import again. {UNCHANGED}"
            )
        if not self.running_strokes:
            raise SvgGeometryError(
                "Stroke-only artwork needs an explicit centreline interpretation. "
                "Turn on running centrelines and import again. "
                f"{UNCHANGED}"
            )
        self._loss(
            "centreline",
            "Running stitches follow the path centreline. They do not reproduce the painted stroke width.",
        )
        for item in subpaths:
            points = list(item["points"])
            if item["closed"] and not _near(_xy(points[0]), _xy(points[-1])):
                points.append(points[0])
            self._add_running(element, name, ancestry, paint, points)

    def _subpaths(
        self,
        element: ElementTree.Element,
        name: str,
        ctm: Matrix,
    ) -> list[dict]:
        if name == "path":
            data = _attr(element, "d") or ""
            if len(data) > MAX_PATH_CHARS:
                raise SvgGeometryError(
                    "A path is longer than the import limit. Simplify it and import again. "
                    f"{UNCHANGED}"
                )
            parser = _PathParser(data, ctm, self.tolerance, self)
            subpaths = parser.parse()
        elif name == "rect":
            subpaths = [_closed(_rect_points(element, ctm))]
        elif name in {"circle", "ellipse"}:
            self.curved = True
            subpaths = [_closed(_ellipse_points(element, name, ctm, self.tolerance))]
            subpaths[0]["curved"] = True
        elif name == "polygon":
            subpaths = [_closed(_point_list(element, ctm))]
        elif name == "polyline":
            subpaths = [{"points": _point_list(element, ctm), "closed": False, "curved": False}]
        elif name == "line":
            subpaths = [{"points": _line_points(element, ctm), "closed": False, "curved": False}]
        else:
            subpaths = []
        if len(subpaths) > MAX_SUBPATHS:
            raise SvgGeometryError(
                f"One element has more than {MAX_SUBPATHS} subpaths. Split it and "
                f"import again. {UNCHANGED}"
            )
        cleaned = []
        for item in subpaths:
            points = _dedupe(item["points"])
            if item["closed"]:
                if len(points) >= 2 and _near(_xy(points[0]), _xy(points[-1])):
                    points = points[:-1]
                if len(points) < 3:
                    raise SvgGeometryError(
                        "A closed shape needs at least three distinct points. Repair "
                        f"the outline and import again. {UNCHANGED}"
                    )
            elif len(points) < 2:
                raise SvgGeometryError(
                    "A line needs two distinct points. Repair it and import again. "
                    f"{UNCHANGED}"
                )
            if len(points) > MAX_RING_POINTS:
                raise SvgGeometryError(
                    f"A shape flattened to more than {MAX_RING_POINTS} points. Raise the "
                    f"tolerance slightly or simplify the path and import again. {UNCHANGED}"
                )
            _ensure_inside(points, self.frame)
            if item["curved"]:
                self.curved = True
            cleaned.append({"points": points, "closed": item["closed"], "curved": item["curved"]})
        return cleaned

    def _add_fill(
        self,
        element: ElementTree.Element,
        name: str,
        ancestry: list[str],
        paint: Paint,
        region: dict,
    ) -> None:
        self.fill_count += 1
        label = _object_label(_attr(element, "id"), "Fill", self.fill_count, region["index"])
        colour = paint.fill or "#000000"
        obj = _base_object(self._next_id(), label, "fill", region["outer"], colour, self)
        if region["holes"]:
            obj["holes"] = region["holes"]
        self.objects.append(obj)
        self.records.append(
            self._record(obj, element, name, ancestry, paint, "fill", region["curved"])
        )

    def _add_running(
        self,
        element: ElementTree.Element,
        name: str,
        ancestry: list[str],
        paint: Paint,
        points: list[dict],
    ) -> None:
        self.running_count += 1
        label = _object_label(_attr(element, "id"), "Running", self.running_count, 1)
        colour = paint.stroke or paint.fill or "#000000"
        obj = _base_object(self._next_id(), label, "running", points, colour, self)
        obj["max_stitch_mm"] = RUNNING_MAX_STITCH_MM
        self.objects.append(obj)
        self.records.append(self._record(obj, element, name, ancestry, paint, "running", False))

    def _next_id(self) -> int:
        if len(self.objects) >= MAX_OBJECTS:
            raise SvgGeometryError(
                f"This SVG produces more than {MAX_OBJECTS} objects. Split the artwork "
                f"and import again. {UNCHANGED}"
            )
        return len(self.objects) + 1

    def _record(
        self,
        obj: dict,
        element: ElementTree.Element,
        name: str,
        ancestry: list[str],
        paint: Paint,
        role: str,
        curved: bool,
    ) -> dict:
        return {
            "object_id": obj["object_id"],
            "source_element": name,
            "source_id": _attr(element, "id"),
            "groups": list(ancestry),
            "transform": _attr(element, "transform"),
            "paint": paint.fill if role == "fill" else paint.stroke,
            "fill_rule": paint.fill_rule if role == "fill" else None,
            "role": role,
            "flattened": curved,
        }

    def _reject_attributes(self, element: ElementTree.Element) -> None:
        for key in element.attrib:
            local = key.split("}", 1)[-1]
            lowered = local.lower()
            if lowered in {"href", "xlink:href"} or key.endswith("}href"):
                raise SvgGeometryError(
                    "External and internal SVG references are not converted. Remove "
                    f"the reference and import again. {UNCHANGED}"
                )
            if lowered == "style":
                raise SvgGeometryError(
                    "SVG style attributes are not converted. Set fill and stroke on "
                    f"the element and import again. {UNCHANGED}"
                )
            if lowered == "class":
                raise SvgGeometryError(
                    "SVG classes are not converted. Set fill and stroke on the element "
                    f"and import again. {UNCHANGED}"
                )
            if lowered in {"clip-path", "mask", "filter", "marker-start", "marker-mid", "marker-end"}:
                value = (element.attrib[key] or "").strip()
                if value and value.lower() != "none":
                    raise SvgGeometryError(
                        f"The {local} effect is not converted. Remove it and import again. "
                        f"{UNCHANGED}"
                    )
            if lowered in {"opacity", "fill-opacity", "stroke-opacity"}:
                value = (element.attrib[key] or "").strip()
                if value not in {"", "1", "1.0"}:
                    raise SvgGeometryError(
                        "Partial opacity is not converted. Use a solid colour and import "
                        f"again. {UNCHANGED}"
                    )
            if lowered == "display" and (element.attrib[key] or "").strip() not in {"", "inline"}:
                raise SvgGeometryError(
                    "A hidden SVG element was not dropped silently. Remove it or make it "
                    f"visible and import again. {UNCHANGED}"
                )
            if lowered == "visibility" and (element.attrib[key] or "").strip() not in {"", "visible"}:
                raise SvgGeometryError(
                    "A hidden SVG element was not dropped silently. Remove it or make it "
                    f"visible and import again. {UNCHANGED}"
                )

    def _loss(self, key: str, text: str) -> None:
        if key in self._loss_keys:
            return
        self._loss_keys.add(key)
        self.losses.append(text)

    def note_curve(self) -> None:
        self.curved = True

    def note_command(self) -> None:
        self.commands += 1
        if self.commands > MAX_COMMANDS:
            raise SvgGeometryError(
                f"This SVG has more than {MAX_COMMANDS} path commands. Simplify it and "
                f"import again. {UNCHANGED}"
            )


def _document_losses(converter: _Converter) -> list[str]:
    losses = list(converter.losses)
    losses.append(
        "Draft fill spacing is 1.2 mm at angle 0, with a 3.0 mm maximum stitch. "
        "These are starter parameters, not a digitising decision."
    )
    if converter.curved:
        losses.append(
            f"Curves were flattened at {converter.tolerance:g} mm. "
            "The native project does not keep Bézier handles."
        )
    losses.append(
        "Source colours stay in the import manifest. The native project stores color_index only, not RGB and not a thread brand."
    )
    losses.append(
        "Coordinates use the native hoop, from the origin toward positive x and y. "
        "This is not a centred TwinLab generic-flat-120-v1 document."
    )
    losses.append(
        "No geometry review was recorded. Machine release is not authorised."
    )
    return losses


def _base_object(
    object_id: int,
    label: str,
    kind: str,
    points: list[dict],
    colour: str,
    converter: _Converter,
) -> dict:
    if colour not in converter.palette:
        converter.palette.append(colour)
    return {
        "object_id": object_id,
        "label": label,
        "kind": kind,
        "points": points,
        "angle_degrees": 0.0,
        "density_mm": FILL_DENSITY_MM,
        "column_width_mm": COLUMN_WIDTH_MM,
        "max_stitch_mm": FILL_MAX_STITCH_MM,
        "underlay_spacing_mm": 0.0,
        "underlay": [],
        "pull_compensation_mm": 0.0,
        "color_index": converter.palette.index(colour),
    }


def _regions(rings: list[list[dict]], fill_rule: str) -> list[dict]:
    if len(rings) == 1:
        _require_simple(rings[0], "Outline")
        return [{"outer": rings[0], "holes": [], "index": 1, "curved": False}]
    for index, ring in enumerate(rings, start=1):
        _require_simple(ring, f"Ring {index}")
    for left in range(len(rings)):
        for right in range(left + 1, len(rings)):
            if _rings_touch(rings[left], rings[right]):
                raise SvgGeometryError(
                    "Two rings touch or cross. Separate them, or leave a gap around a hole, "
                    f"and import again. {UNCHANGED}"
                )
    containers: list[list[int]] = [[] for _ in rings]
    for index, ring in enumerate(rings):
        for other in range(len(rings)):
            if index != other and _strictly_inside(ring, rings[other]):
                containers[index].append(other)
    for index, parents in enumerate(containers):
        for parent in parents:
            if index in containers[parent]:
                raise SvgGeometryError(
                    "Two rings overlap without one sitting inside the other. Separate them "
                    f"and import again. {UNCHANGED}"
                )
    depths = [len(parents) for parents in containers]
    if any(depth >= 2 for depth in depths):
        raise SvgGeometryError(
            "A ring is nested more than one level deep. This importer keeps one outer "
            f"boundary and its direct holes only. Simplify the rings and import again. {UNCHANGED}"
        )
    outers = [index for index, depth in enumerate(depths) if depth == 0]
    if not outers:
        raise SvgGeometryError(
            "The filled rings have no outer boundary. Repair the path and import again. "
            f"{UNCHANGED}"
        )
    regions = []
    for position, outer in enumerate(outers, start=1):
        holes = []
        for index, parents in enumerate(containers):
            if parents == [outer]:
                if fill_rule == "nonzero" and _same_winding(rings[outer], rings[index]):
                    raise SvgGeometryError(
                        "A nested ring uses the same winding as its container. Nonzero fill "
                        "would not open a hole. Reverse that ring, or use evenodd, and "
                        f"import again. {UNCHANGED}"
                    )
                holes.append(rings[index])
        regions.append({"outer": rings[outer], "holes": holes, "index": position, "curved": False})
    return regions


def _require_simple(ring: list[dict], label: str) -> None:
    if abs(_signed_area(ring)) <= AREA_EPS:
        raise SvgGeometryError(
            f"{label} has no area. Repair that ring and import again. {UNCHANGED}"
        )
    count = len(ring)
    for index in range(count):
        start = _xy(ring[index])
        end = _xy(ring[(index + 1) % count])
        if _near(start, end, 1e-6):
            raise SvgGeometryError(
                f"{label} has a zero-length edge. Repair that ring and import again. {UNCHANGED}"
            )
        for other in range(index + 1, count):
            if other == index or (index + 1) % count == other or (other + 1) % count == index:
                continue
            a = start
            b = end
            c = _xy(ring[other])
            d = _xy(ring[(other + 1) % count])
            if _segments_conflict(a, b, c, d, allow_shared_endpoint=True):
                raise SvgGeometryError(
                    f"{label} crosses itself. Repair that ring and import again. {UNCHANGED}"
                )


def _rings_touch(left: list[dict], right: list[dict]) -> bool:
    for index in range(len(left)):
        a = _xy(left[index])
        b = _xy(left[(index + 1) % len(left)])
        for other in range(len(right)):
            c = _xy(right[other])
            d = _xy(right[(other + 1) % len(right)])
            if _segments_conflict(a, b, c, d, allow_shared_endpoint=False):
                return True
    return False


def _strictly_inside(inner: list[dict], outer: list[dict]) -> bool:
    return all(_point_strictly_inside(_xy(point), outer) for point in inner)


def _same_winding(outer: list[dict], inner: list[dict]) -> bool:
    return _signed_area(outer) * _signed_area(inner) > 0


def _signed_area(ring: list[dict]) -> float:
    total = 0.0
    for index, point in enumerate(ring):
        nxt = ring[(index + 1) % len(ring)]
        total += point["x"] * nxt["y"] - nxt["x"] * point["y"]
    return total / 2.0


def _point_strictly_inside(point: tuple[float, float], ring: list[dict]) -> bool:
    if _on_boundary(point, ring):
        return False
    crossings = 0
    x, y = point
    for index, start in enumerate(ring):
        end = ring[(index + 1) % len(ring)]
        y1, y2 = start["y"], end["y"]
        if (y1 > y) == (y2 > y):
            continue
        x_hit = start["x"] + (y - y1) * (end["x"] - start["x"]) / (y2 - y1)
        if x_hit > x:
            crossings += 1
    return crossings % 2 == 1


def _on_boundary(point: tuple[float, float], ring: list[dict]) -> bool:
    for index, start in enumerate(ring):
        end = ring[(index + 1) % len(ring)]
        if _distance_to_segment(point, _xy(start), _xy(end)) <= TOUCH_EPS_MM:
            return True
    return False


def _segments_conflict(
    a: tuple[float, float],
    b: tuple[float, float],
    c: tuple[float, float],
    d: tuple[float, float],
    allow_shared_endpoint: bool,
) -> bool:
    if allow_shared_endpoint and (_near(a, c) or _near(a, d) or _near(b, c) or _near(b, d)):
        shared = {pair for pair in ((a, c), (a, d), (b, c), (b, d)) if _near(pair[0], pair[1])}
        if len(shared) == 1 and not _collinear_overlap(a, b, c, d):
            return False
    o1 = _orient(a, b, c)
    o2 = _orient(a, b, d)
    o3 = _orient(c, d, a)
    o4 = _orient(c, d, b)
    if o1 * o2 < 0 and o3 * o4 < 0:
        return True
    if o1 == 0 and _on_segment(a, b, c):
        return True
    if o2 == 0 and _on_segment(a, b, d):
        return True
    if o3 == 0 and _on_segment(c, d, a):
        return True
    if o4 == 0 and _on_segment(c, d, b):
        return True
    return False


def _collinear_overlap(
    a: tuple[float, float],
    b: tuple[float, float],
    c: tuple[float, float],
    d: tuple[float, float],
) -> bool:
    if _orient(a, b, c) != 0 or _orient(a, b, d) != 0:
        return False
    return _ranges_overlap(a[0], b[0], c[0], d[0]) and _ranges_overlap(a[1], b[1], c[1], d[1])


def _ranges_overlap(a: float, b: float, c: float, d: float) -> bool:
    return max(min(a, b), min(c, d)) <= min(max(a, b), max(c, d)) + TOUCH_EPS_MM


def _orient(a: tuple[float, float], b: tuple[float, float], c: tuple[float, float]) -> int:
    value = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    if abs(value) <= 1e-7:
        return 0
    return 1 if value > 0 else -1


def _on_segment(
    a: tuple[float, float],
    b: tuple[float, float],
    point: tuple[float, float],
) -> bool:
    return _distance_to_segment(point, a, b) <= TOUCH_EPS_MM


def _distance_to_segment(
    point: tuple[float, float],
    start: tuple[float, float],
    end: tuple[float, float],
) -> float:
    dx = end[0] - start[0]
    dy = end[1] - start[1]
    length_sq = dx * dx + dy * dy
    if length_sq <= 1e-18:
        return math.hypot(point[0] - start[0], point[1] - start[1])
    t = ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / length_sq
    t = min(1.0, max(0.0, t))
    return math.hypot(point[0] - (start[0] + t * dx), point[1] - (start[1] + t * dy))


class _PathParser:
    def __init__(self, text: str, ctm: Matrix, tolerance: float, converter: _Converter) -> None:
        self.text = text
        self.i = 0
        self.ctm = ctm
        self.tolerance = tolerance
        self.converter = converter
        self.cx = 0.0
        self.cy = 0.0
        self.sx = 0.0
        self.sy = 0.0
        self.last_cmd = ""
        self.prev_cubic: tuple[float, float] | None = None
        self.prev_quad: tuple[float, float] | None = None
        self.subpaths: list[dict] = []
        self.current: list[dict] | None = None
        self.curved = False

    def parse(self) -> list[dict]:
        while True:
            self._skip()
            if self.i >= len(self.text):
                break
            char = self.text[self.i]
            if char.isalpha():
                command = char
                self.i += 1
            elif self.last_cmd:
                command = "L" if self.last_cmd == "M" else "l" if self.last_cmd == "m" else self.last_cmd
            else:
                raise SvgGeometryError(
                    f"A path has a number before its first command. Repair the path and import again. {UNCHANGED}"
                )
            self.converter.note_command()
            self._dispatch(command)
            self.last_cmd = command
        self._finish(False)
        return self.subpaths

    def _dispatch(self, command: str) -> None:
        if command in {"A", "a"}:
            raise SvgGeometryError(
                "This SVG uses an arc command. Replace arcs with cubic curves and import again. "
                f"{UNCHANGED}"
            )
        if command in {"M", "m"}:
            x, y = self._absolute_pair(command)
            self._finish(False)
            self.cx, self.cy = x, y
            self.sx, self.sy = x, y
            self.current = [self._point(x, y)]
            self.curved = False
            self.prev_cubic = None
            self.prev_quad = None
            while self._has_number():
                x, y = self._absolute_pair("l" if command == "m" else "L")
                self._line_to(x, y)
            return
        if command in {"Z", "z"}:
            if not self.current:
                raise SvgGeometryError(
                    f"A path closes a subpath that was not started. Repair the path and import again. {UNCHANGED}"
                )
            self.cx, self.cy = self.sx, self.sy
            self._finish(True)
            return
        if command in {"L", "l"}:
            while True:
                x, y = self._absolute_pair(command)
                self._line_to(x, y)
                if not self._has_number():
                    return
        if command in {"H", "h"}:
            while True:
                value = self._number()
                x = self.cx + value if command == "h" else value
                self._line_to(x, self.cy)
                if not self._has_number():
                    return
        if command in {"V", "v"}:
            while True:
                value = self._number()
                y = self.cy + value if command == "v" else value
                self._line_to(self.cx, y)
                if not self._has_number():
                    return
        if command in {"C", "c"}:
            while True:
                x1, y1 = self._absolute_pair(command)
                x2, y2 = self._absolute_pair(command)
                x, y = self._absolute_pair(command)
                self._curve_to((self.cx, self.cy), (x1, y1), (x2, y2), (x, y))
                self.prev_cubic = (x2, y2)
                self.prev_quad = None
                if not self._has_number():
                    return
        if command in {"S", "s"}:
            while True:
                if self.last_cmd in {"C", "c", "S", "s"} and self.prev_cubic is not None:
                    x1 = 2 * self.cx - self.prev_cubic[0]
                    y1 = 2 * self.cy - self.prev_cubic[1]
                else:
                    x1, y1 = self.cx, self.cy
                x2, y2 = self._absolute_pair(command)
                x, y = self._absolute_pair(command)
                self._curve_to((self.cx, self.cy), (x1, y1), (x2, y2), (x, y))
                self.prev_cubic = (x2, y2)
                self.prev_quad = None
                self.last_cmd = command
                if not self._has_number():
                    return
        if command in {"Q", "q"}:
            while True:
                x1, y1 = self._absolute_pair(command)
                x, y = self._absolute_pair(command)
                self._quad_to((self.cx, self.cy), (x1, y1), (x, y))
                self.prev_quad = (x1, y1)
                self.prev_cubic = None
                if not self._has_number():
                    return
        if command in {"T", "t"}:
            while True:
                if self.last_cmd in {"Q", "q", "T", "t"} and self.prev_quad is not None:
                    x1 = 2 * self.cx - self.prev_quad[0]
                    y1 = 2 * self.cy - self.prev_quad[1]
                else:
                    x1, y1 = self.cx, self.cy
                x, y = self._absolute_pair(command)
                self._quad_to((self.cx, self.cy), (x1, y1), (x, y))
                self.prev_quad = (x1, y1)
                self.prev_cubic = None
                self.last_cmd = command
                if not self._has_number():
                    return
        raise SvgGeometryError(
            f"The path command {command} is not supported. Use lines or cubic and quadratic curves, "
            f"and import again. {UNCHANGED}"
        )

    def _line_to(self, x: float, y: float) -> None:
        self._ensure_current()
        assert self.current is not None
        self.current.append(self._point(x, y))
        self.cx, self.cy = x, y
        self.prev_cubic = None
        self.prev_quad = None

    def _curve_to(
        self,
        start: tuple[float, float],
        control_1: tuple[float, float],
        control_2: tuple[float, float],
        end: tuple[float, float],
    ) -> None:
        self._ensure_current()
        assert self.current is not None
        points = _flatten_cubic(
            self.ctm.apply(*start),
            self.ctm.apply(*control_1),
            self.ctm.apply(*control_2),
            self.ctm.apply(*end),
            self.tolerance,
        )
        self.current.extend(_pack(point) for point in points)
        self.cx, self.cy = end
        self.curved = True
        self.converter.note_curve()

    def _quad_to(
        self,
        start: tuple[float, float],
        control: tuple[float, float],
        end: tuple[float, float],
    ) -> None:
        self._ensure_current()
        assert self.current is not None
        points = _flatten_quadratic(
            self.ctm.apply(*start),
            self.ctm.apply(*control),
            self.ctm.apply(*end),
            self.tolerance,
        )
        self.current.extend(_pack(point) for point in points)
        self.cx, self.cy = end
        self.curved = True
        self.converter.note_curve()

    def _finish(self, closed: bool) -> None:
        if not self.current:
            return
        self.subpaths.append(
            {"points": self.current, "closed": closed, "curved": self.curved}
        )
        self.current = None
        self.curved = False

    def _ensure_current(self) -> None:
        if self.current is None:
            raise SvgGeometryError(
                f"A path command appears before a move command. Repair the path and import again. {UNCHANGED}"
            )

    def _absolute_pair(self, command: str) -> tuple[float, float]:
        x, y = self._number(), self._number()
        if command.islower():
            x += self.cx
            y += self.cy
        return x, y

    def _point(self, x: float, y: float) -> dict:
        return _pack(self.ctm.apply(x, y))

    def _number(self) -> float:
        self._skip_separator()
        start = self.i
        if self.i < len(self.text) and self.text[self.i] in "+-":
            self.i += 1
        saw_digit = False
        while self.i < len(self.text) and self.text[self.i].isdigit():
            saw_digit = True
            self.i += 1
        if self.i < len(self.text) and self.text[self.i] == ".":
            self.i += 1
            while self.i < len(self.text) and self.text[self.i].isdigit():
                saw_digit = True
                self.i += 1
        if self.i < len(self.text) and self.text[self.i] in "eE":
            mark = self.i
            self.i += 1
            if self.i < len(self.text) and self.text[self.i] in "+-":
                self.i += 1
            exponent = False
            while self.i < len(self.text) and self.text[self.i].isdigit():
                exponent = True
                self.i += 1
            if not exponent:
                self.i = mark
        if not saw_digit:
            raise SvgGeometryError(
                f"A path has a missing number. Repair the path and import again. {UNCHANGED}"
            )
        try:
            return float(self.text[start:self.i])
        except ValueError as error:
            raise SvgGeometryError(
                f"A path has a number that cannot be read. Repair the path and import again. {UNCHANGED}"
            ) from error

    def _has_number(self) -> bool:
        saved = self.i
        self._skip_separator()
        found = self.i < len(self.text) and (
            self.text[self.i].isdigit() or self.text[self.i] in "+-."
        )
        self.i = saved
        return found

    def _skip(self) -> None:
        while self.i < len(self.text) and self.text[self.i] in " \t\r\n,":
            self.i += 1

    def _skip_separator(self) -> None:
        self._skip()


def _flatten_cubic(
    p0: tuple[float, float],
    p1: tuple[float, float],
    p2: tuple[float, float],
    p3: tuple[float, float],
    tolerance: float,
    depth: int = 0,
) -> list[tuple[float, float]]:
    if depth >= 16 or (
        _point_line_distance(p1, p0, p3) <= tolerance
        and _point_line_distance(p2, p0, p3) <= tolerance
    ):
        return [] if _near(p0, p3, tolerance) and depth > 0 else [p3]
    left, right = _split_cubic(p0, p1, p2, p3)
    return _flatten_cubic(*left, tolerance, depth + 1) + _flatten_cubic(*right, tolerance, depth + 1)


def _flatten_quadratic(
    p0: tuple[float, float],
    p1: tuple[float, float],
    p2: tuple[float, float],
    tolerance: float,
    depth: int = 0,
) -> list[tuple[float, float]]:
    if depth >= 16 or _point_line_distance(p1, p0, p2) <= tolerance:
        return [] if _near(p0, p2, tolerance) and depth > 0 else [p2]
    a = _mid(p0, p1)
    b = _mid(p1, p2)
    m = _mid(a, b)
    return _flatten_quadratic(p0, a, m, tolerance, depth + 1) + _flatten_quadratic(
        m, b, p2, tolerance, depth + 1
    )


def _split_cubic(
    p0: tuple[float, float],
    p1: tuple[float, float],
    p2: tuple[float, float],
    p3: tuple[float, float],
) -> tuple[
    tuple[tuple[float, float], tuple[float, float], tuple[float, float], tuple[float, float]],
    tuple[tuple[float, float], tuple[float, float], tuple[float, float], tuple[float, float]],
]:
    p01 = _mid(p0, p1)
    p12 = _mid(p1, p2)
    p23 = _mid(p2, p3)
    p012 = _mid(p01, p12)
    p123 = _mid(p12, p23)
    p0123 = _mid(p012, p123)
    return (p0, p01, p012, p0123), (p0123, p123, p23, p3)


def _point_line_distance(
    point: tuple[float, float],
    start: tuple[float, float],
    end: tuple[float, float],
) -> float:
    dx = end[0] - start[0]
    dy = end[1] - start[1]
    length = math.hypot(dx, dy)
    if length <= 1e-12:
        return math.hypot(point[0] - start[0], point[1] - start[1])
    return abs(dy * point[0] - dx * point[1] + end[0] * start[1] - end[1] * start[0]) / length


def _rect_points(element: ElementTree.Element, ctm: Matrix) -> list[dict]:
    rx = _user_length(element, "rx", ctm, 0.0)
    ry = _user_length(element, "ry", ctm, 0.0)
    if rx or ry:
        raise SvgGeometryError(
            "Rounded rectangle corners are not converted. Use a sharp rectangle or a "
            f"path and import again. {UNCHANGED}"
        )
    x = _user_length(element, "x", ctm, 0.0)
    y = _user_length(element, "y", ctm, 0.0)
    width = _user_length(element, "width", ctm, None)
    height = _user_length(element, "height", ctm, None)
    if width is None or height is None or width <= 0 or height <= 0:
        raise SvgGeometryError(
            f"A rectangle needs a positive width and height. Repair it and import again. {UNCHANGED}"
        )
    corners = [(x, y), (x + width, y), (x + width, y + height), (x, y + height)]
    return [_pack(ctm.apply(*corner)) for corner in corners]


def _ellipse_points(
    element: ElementTree.Element,
    name: str,
    ctm: Matrix,
    tolerance: float,
) -> list[dict]:
    cx = _user_length(element, "cx", ctm, 0.0)
    cy = _user_length(element, "cy", ctm, 0.0)
    if name == "circle":
        radius = _user_length(element, "r", ctm, None)
        if radius is None or radius <= 0:
            raise SvgGeometryError(
                f"A circle needs a positive radius. Repair it and import again. {UNCHANGED}"
            )
        rx = ry = radius
    else:
        rx = _user_length(element, "rx", ctm, None)
        ry = _user_length(element, "ry", ctm, None)
        if rx is None or ry is None or rx <= 0 or ry <= 0:
            raise SvgGeometryError(
                f"An ellipse needs positive radii. Repair it and import again. {UNCHANGED}"
            )
    kx, ky = rx * KAPPA, ry * KAPPA
    cubics = [
        ((cx + rx, cy), (cx + rx, cy + ky), (cx + kx, cy + ry), (cx, cy + ry)),
        ((cx, cy + ry), (cx - kx, cy + ry), (cx - rx, cy + ky), (cx - rx, cy)),
        ((cx - rx, cy), (cx - rx, cy - ky), (cx - kx, cy - ry), (cx, cy - ry)),
        ((cx, cy - ry), (cx + kx, cy - ry), (cx + rx, cy - ky), (cx + rx, cy)),
    ]
    points = [_pack(ctm.apply(cx + rx, cy))]
    for start, c1, c2, end in cubics:
        points.extend(
            _pack(point)
            for point in _flatten_cubic(
                ctm.apply(*start),
                ctm.apply(*c1),
                ctm.apply(*c2),
                ctm.apply(*end),
                tolerance,
            )
        )
    return points


def _point_list(element: ElementTree.Element, ctm: Matrix) -> list[dict]:
    raw = _attr(element, "points")
    if not raw:
        raise SvgGeometryError(
            f"A polygon or polyline is missing its points. Repair it and import again. {UNCHANGED}"
        )
    numbers = [float(item) for item in _NUMBER.findall(raw)]
    if len(numbers) < 4 or len(numbers) % 2:
        raise SvgGeometryError(
            f"A point list needs complete x, y pairs. Repair it and import again. {UNCHANGED}"
        )
    return [_pack(ctm.apply(numbers[i], numbers[i + 1])) for i in range(0, len(numbers), 2)]


def _line_points(element: ElementTree.Element, ctm: Matrix) -> list[dict]:
    x1 = _user_length(element, "x1", ctm, 0.0)
    y1 = _user_length(element, "y1", ctm, 0.0)
    x2 = _user_length(element, "x2", ctm, 0.0)
    y2 = _user_length(element, "y2", ctm, 0.0)
    return [_pack(ctm.apply(x1, y1)), _pack(ctm.apply(x2, y2))]


def _user_length(
    element: ElementTree.Element,
    name: str,
    ctm: Matrix,
    default: float | None,
) -> float | None:
    raw = _attr(element, name)
    if raw is None or raw.strip() == "":
        return default
    match = _LENGTH.match(raw.strip())
    if not match:
        raise SvgGeometryError(
            f"The length {name} could not be read. Use a number and a supported unit, "
            f"then import again. {UNCHANGED}"
        )
    value = float(match.group(1))
    unit = match.group(2).lower()
    if unit == "%":
        raise SvgGeometryError(
            "Percentage lengths are not converted. Use user units or millimetres and "
            f"import again. {UNCHANGED}"
        )
    if unit in {"", "px"}:
        return value
    if unit not in ABSOLUTE_UNITS:
        raise SvgGeometryError(
            f"The unit {unit} is not converted. Use mm, cm, in, pt, pc, px, or user units, "
            f"and import again. {UNCHANGED}"
        )
    scale_x = math.hypot(ctm.a, ctm.b)
    scale_y = math.hypot(ctm.c, ctm.d)
    if scale_x <= 1e-12 or scale_y <= 1e-12:
        raise SvgGeometryError(
            f"A transform removes a coordinate axis. Repair the transform and import again. {UNCHANGED}"
        )
    mm = value * MM_PER_UNIT[unit]
    if name in {"y", "y1", "y2", "cy", "height", "ry"}:
        return mm / scale_y
    return mm / scale_x


def _frame(root: ElementTree.Element, width_mm: float | None) -> Frame:
    width = _viewport_length(_attr(root, "width"), "width")
    height = _viewport_length(_attr(root, "height"), "height")
    view_box = _view_box(_attr(root, "viewBox"))
    align, meet = _aspect(_attr(root, "preserveAspectRatio"))
    if meet == "slice":
        raise SvgGeometryError(
            "preserveAspectRatio slice is not converted. Use meet or none and import again. "
            f"{UNCHANGED}"
        )
    pixel_width = width is not None and width[1] == "px"
    pixel_height = height is not None and height[1] == "px"
    if width is None and height is None and view_box is None and width_mm is None:
        raise SvgGeometryError(
            "This SVG has no width. Set a physical width, or pass the artboard width in "
            f"millimetres, and import again. {UNCHANGED}"
        )
    if (pixel_width or width is None) and width_mm is None and not (
        width and width[1] == "absolute"
    ):
        if width is None or pixel_width:
            if not (width and width[1] == "absolute"):
                if width_mm is None and (pixel_width or (width is None and (height is None or pixel_height))):
                    raise SvgGeometryError(
                        "This SVG sizes the artboard in pixels. Enter the physical artboard "
                        f"width in millimetres and import again. {UNCHANGED}"
                    )
    if width and width[1] == "absolute" and width_mm is not None:
        raise SvgGeometryError(
            "This SVG already has a physical width. Remove the artboard width override "
            f"and import again. {UNCHANGED}"
        )
    if width and width[1] == "absolute":
        vp_w = width[0]
        width_source = "svg-absolute"
    else:
        assert width_mm is not None
        vp_w = float(width_mm)
        width_source = "width-mm-argument"
    if height and height[1] == "absolute" and width_source == "svg-absolute":
        vp_h = height[0]
    elif height and height[1] == "px" and width and width[1] == "px":
        vp_h = vp_w * (height[0] / width[0])
    elif view_box is not None:
        vp_h = vp_w * (view_box[3] / view_box[2])
    elif height and height[1] == "absolute":
        vp_h = height[0]
    else:
        raise SvgGeometryError(
            "This SVG has no height. Set the artboard height, or add a viewBox, and "
            f"import again. {UNCHANGED}"
        )
    if vp_w <= 0 or vp_h <= 0 or vp_w > 10_000 or vp_h > 10_000:
        raise SvgGeometryError(
            "The artboard must be positive and at most 10000 mm on each side. Set the "
            f"size and import again. {UNCHANGED}"
        )
    if view_box is None:
        scale = vp_w / width[0] if width and width[1] == "px" else MM_PER_PX
        if width and width[1] == "absolute":
            scale = MM_PER_PX
        matrix = Matrix(scale, 0, 0, scale, 0, 0)
        box = None
        aspect = "none"
    else:
        matrix = _view_matrix(view_box, vp_w, vp_h, align)
        box = [view_box[0], view_box[1], view_box[2], view_box[3]]
        aspect = align if align == "none" else f"{align} {meet}"
    return Frame(vp_w, vp_h, matrix, width_source, box, aspect)


def _viewport_length(raw: str | None, label: str) -> tuple[float, str] | None:
    if raw is None or raw.strip() == "":
        return None
    match = _LENGTH.match(raw.strip())
    if not match:
        raise SvgGeometryError(
            f"The SVG {label} could not be read. Use a length such as 100mm and import again. {UNCHANGED}"
        )
    value = float(match.group(1))
    unit = match.group(2).lower()
    if value <= 0:
        raise SvgGeometryError(
            f"The SVG {label} must be positive. Set the artboard size and import again. {UNCHANGED}"
        )
    if unit == "%":
        raise SvgGeometryError(
            f"A percentage {label} is not converted. Use millimetres or pixels with an "
            f"artboard width and import again. {UNCHANGED}"
        )
    if unit in ABSOLUTE_UNITS:
        return value * MM_PER_UNIT[unit], "absolute"
    if unit in {"", "px"}:
        return value, "px"
    raise SvgGeometryError(
        f"The unit on {label} is not converted. Use mm, cm, in, pt, pc, or px, and "
        f"import again. {UNCHANGED}"
    )


def _view_box(raw: str | None) -> tuple[float, float, float, float] | None:
    if raw is None or raw.strip() == "":
        return None
    numbers = [float(item) for item in _NUMBER.findall(raw)]
    if len(numbers) != 4:
        raise SvgGeometryError(
            f"The viewBox must contain four numbers. Repair it and import again. {UNCHANGED}"
        )
    if numbers[2] <= 0 or numbers[3] <= 0:
        raise SvgGeometryError(
            f"The viewBox width and height must be positive. Repair it and import again. {UNCHANGED}"
        )
    return numbers[0], numbers[1], numbers[2], numbers[3]


def _aspect(raw: str | None) -> tuple[str, str]:
    if raw is None or raw.strip() == "":
        return "xMidYMid", "meet"
    parts = raw.replace(",", " ").split()
    align = parts[0]
    meet = parts[1] if len(parts) > 1 else "meet"
    if align != "none" and align not in ALIGNS:
        raise SvgGeometryError(
            f"preserveAspectRatio {align} is not converted. Use meet or none and import again. {UNCHANGED}"
        )
    if meet not in {"meet", "slice"}:
        raise SvgGeometryError(
            f"preserveAspectRatio {meet} is not converted. Use meet or none and import again. {UNCHANGED}"
        )
    if align == "none":
        return "none", "meet"
    return align, meet


def _view_matrix(
    view_box: tuple[float, float, float, float],
    vp_w: float,
    vp_h: float,
    align: str,
) -> Matrix:
    min_x, min_y, vb_w, vb_h = view_box
    scale_x = vp_w / vb_w
    scale_y = vp_h / vb_h
    if align != "none":
        scale = min(scale_x, scale_y)
        scale_x = scale_y = scale
    tx = -min_x * scale_x
    ty = -min_y * scale_y
    if "xMid" in align:
        tx += (vp_w - vb_w * scale_x) / 2.0
    elif "xMax" in align:
        tx += vp_w - vb_w * scale_x
    if "YMid" in align:
        ty += (vp_h - vb_h * scale_y) / 2.0
    elif "YMax" in align:
        ty += vp_h - vb_h * scale_y
    return Matrix(scale_x, 0.0, 0.0, scale_y, tx, ty)


def _parse_transform(raw: str | None) -> Matrix:
    if raw is None or raw.strip() == "":
        return Matrix()
    position = 0
    result = Matrix()
    for match in _TRANSFORM.finditer(raw):
        junk = raw[position:match.start()].strip().strip(",")
        if junk:
            raise SvgGeometryError(
                f"The transform {raw!r} could not be read. Repair it and import again. {UNCHANGED}"
            )
        result = result.multiply(_transform_function(match.group(1), match.group(2)))
        position = match.end()
    if raw[position:].strip().strip(","):
        raise SvgGeometryError(
            f"The transform {raw!r} could not be read. Repair it and import again. {UNCHANGED}"
        )
    return result


def _transform_function(name: str, argument_text: str) -> Matrix:
    args = [float(item) for item in _NUMBER.findall(argument_text)]
    if name == "translate":
        if len(args) not in {1, 2}:
            raise SvgGeometryError(
                f"translate needs one or two numbers. Repair the transform and import again. {UNCHANGED}"
            )
        return Matrix(1, 0, 0, 1, args[0], args[1] if len(args) == 2 else 0.0)
    if name == "scale":
        if len(args) not in {1, 2}:
            raise SvgGeometryError(
                f"scale needs one or two numbers. Repair the transform and import again. {UNCHANGED}"
            )
        sy = args[1] if len(args) == 2 else args[0]
        return Matrix(args[0], 0, 0, sy, 0, 0)
    if name == "rotate":
        if len(args) not in {1, 3}:
            raise SvgGeometryError(
                f"rotate needs an angle, or an angle and a pivot. Repair the transform and import again. {UNCHANGED}"
            )
        radians = math.radians(args[0])
        rotation = Matrix(math.cos(radians), math.sin(radians), -math.sin(radians), math.cos(radians), 0, 0)
        if len(args) == 1:
            return rotation
        return Matrix(1, 0, 0, 1, args[1], args[2]).multiply(rotation).multiply(
            Matrix(1, 0, 0, 1, -args[1], -args[2])
        )
    if name == "skewX":
        if len(args) != 1:
            raise SvgGeometryError(
                f"skewX needs one angle. Repair the transform and import again. {UNCHANGED}"
            )
        return Matrix(1, 0, math.tan(math.radians(args[0])), 1, 0, 0)
    if name == "skewY":
        if len(args) != 1:
            raise SvgGeometryError(
                f"skewY needs one angle. Repair the transform and import again. {UNCHANGED}"
            )
        return Matrix(1, math.tan(math.radians(args[0])), 0, 1, 0, 0)
    if name == "matrix":
        if len(args) != 6:
            raise SvgGeometryError(
                f"matrix needs six numbers. Repair the transform and import again. {UNCHANGED}"
            )
        return Matrix(*args)
    raise SvgGeometryError(
        f"The transform {name} is not supported. Use translate, scale, rotate, skew, or matrix, "
        f"and import again. {UNCHANGED}"
    )


def Paint_with_element(self: Paint, element: ElementTree.Element) -> Paint:
    fill = _attr(element, "fill")
    stroke = _attr(element, "stroke")
    rule = _attr(element, "fill-rule")
    width = _attr(element, "stroke-width")
    next_rule = self.fill_rule
    if rule is not None and rule != "inherit":
        if rule not in {"nonzero", "evenodd"}:
            raise SvgGeometryError(
                f"The fill rule {rule} is not converted. Use nonzero or evenodd and import again. {UNCHANGED}"
            )
        next_rule = rule
    return Paint(
        fill=self.fill if fill is None or fill == "inherit" else _paint_colour(fill),
        stroke=self.stroke if stroke is None or stroke == "inherit" else _paint_colour(stroke),
        fill_rule=next_rule,
        stroke_width=self.stroke_width if width is None else width,
    )


Paint.with_element = Paint_with_element  # type: ignore[attr-defined]


def _paint_colour(raw: str) -> str | None:
    text = raw.strip()
    lowered = text.lower()
    if lowered in {"none", "transparent"}:
        return None
    if lowered in {"currentcolor", "context-fill", "context-stroke"}:
        raise SvgGeometryError(
            "currentColor is not converted. Set a solid colour and import again. "
            f"{UNCHANGED}"
        )
    if lowered.startswith("url("):
        raise SvgGeometryError(
            "Paint servers such as gradients are not converted. Use a solid colour and "
            f"import again. {UNCHANGED}"
        )
    if lowered in NAMED_COLOURS:
        return NAMED_COLOURS[lowered]
    hex_text = lowered[1:] if lowered.startswith("#") else None
    if hex_text is not None:
        if len(hex_text) == 3:
            hex_text = "".join(char * 2 for char in hex_text)
        if len(hex_text) == 4:
            hex_text = "".join(char * 2 for char in hex_text)
        if len(hex_text) == 8:
            if hex_text[6:] not in {"ff"}:
                raise SvgGeometryError(
                    "A colour with partial alpha is not converted. Use a solid colour and "
                    f"import again. {UNCHANGED}"
                )
            hex_text = hex_text[:6]
        if len(hex_text) == 6 and all(char in "0123456789abcdef" for char in hex_text):
            return f"#{hex_text}"
    rgb = re.fullmatch(
        r"rgb\(\s*([+-]?\d*\.?\d+%?)\s*,\s*([+-]?\d*\.?\d+%?)\s*,\s*([+-]?\d*\.?\d+%?)\s*\)",
        lowered,
    )
    if rgb:
        channels = [_rgb_channel(item) for item in rgb.groups()]
        return "#" + "".join(f"{channel:02x}" for channel in channels)
    raise SvgGeometryError(
        f"The colour {raw!r} is not converted. Use a hex colour or a solid named colour "
        f"and import again. {UNCHANGED}"
    )


def _rgb_channel(raw: str) -> int:
    if raw.endswith("%"):
        value = float(raw[:-1]) * 2.55
    else:
        value = float(raw)
    if value < 0 or value > 255:
        raise SvgGeometryError(
            f"A colour channel is outside 0 to 255. Repair the colour and import again. {UNCHANGED}"
        )
    return int(round(value))


def _local_name(element: ElementTree.Element) -> str:
    tag = element.tag
    if not isinstance(tag, str):
        raise SvgGeometryError(
            f"This SVG contains an unsupported XML construct. Remove it and import again. {UNCHANGED}"
        )
    if tag.startswith("{"):
        namespace, name = tag[1:].split("}", 1)
        if namespace != SVG_NS:
            raise SvgGeometryError(
                f"The element {name} is not in the SVG namespace. Remove it and import again. {UNCHANGED}"
            )
        return name.lower()
    return tag.lower()


def _attr(element: ElementTree.Element, name: str) -> str | None:
    if name in element.attrib:
        return element.attrib[name]
    qualified = f"{{{SVG_NS}}}{name}"
    if qualified in element.attrib:
        return element.attrib[qualified]
    return None


def _reject_dangerous_markup(raw: bytes) -> None:
    if len(raw) > MAX_BYTES:
        raise SvgGeometryError(
            "This SVG is larger than the import limit. Simplify the file and import again. "
            f"{UNCHANGED}"
        )
    if b"\x00" in raw:
        raise SvgGeometryError(
            f"This SVG is not UTF-8 text. Save it as UTF-8 and import again. {UNCHANGED}"
        )
    try:
        text = raw.decode("utf-8").lstrip("\ufeff")
    except UnicodeDecodeError as error:
        raise SvgGeometryError(
            f"This SVG is not UTF-8 text. Save it as UTF-8 and import again. {UNCHANGED}"
        ) from error
    lowered = text.lower()
    if any(token in lowered for token in ("<!doctype", "<!entity", "<!element", "<!attlist")):
        raise SvgGeometryError(
            "Document type declarations and entities are not accepted. Save a plain SVG "
            f"document and import again. {UNCHANGED}"
        )
    if "<![cdata[" in lowered:
        raise SvgGeometryError(
            f"CDATA sections are not accepted. Save a plain SVG document and import again. {UNCHANGED}"
        )
    search = 0
    seen_xml = False
    while True:
        start = text.find("<?", search)
        if start < 0:
            break
        end = text.find("?>", start)
        if end < 0:
            raise SvgGeometryError(
                f"A processing instruction is not closed. Repair the file and import again. {UNCHANGED}"
            )
        body = text[start + 2:end].strip().lower()
        if body.startswith("xml") and not seen_xml and "<" not in text[:start].replace("\ufeff", ""):
            seen_xml = True
        else:
            raise SvgGeometryError(
                "Processing instructions other than the XML declaration are not accepted. "
                f"Remove them and import again. {UNCHANGED}"
            )
        search = end + 2


def _source_bytes(source: bytes | str) -> bytes:
    if isinstance(source, str):
        raw = source.encode("utf-8")
    else:
        raw = bytes(source)
    if not raw.strip():
        raise SvgGeometryError(
            f"Choose an SVG file, then preview the import. {UNCHANGED}"
        )
    return raw


def _project_name(requested: str | None, svg_id: str | None) -> str:
    raw = requested.strip() if isinstance(requested, str) and requested.strip() else (svg_id or "Imported vector")
    cleaned = " ".join(raw.split())
    if not cleaned or any(char.isascii() and ord(char) < 32 for char in cleaned):
        raise SvgGeometryError(
            f"The project name must be visible text up to 80 characters. Rename it and import again. {UNCHANGED}"
        )
    if len(cleaned) > MAX_NAME:
        cleaned = cleaned[:MAX_NAME].rstrip()
    return cleaned


def _object_label(source_id: str | None, kind: str, count: int, region: int) -> str:
    base = _clean_token(source_id) if source_id else kind
    if region > 1 or (source_id and count):
        label = base if region == 1 and count == 1 else f"{base} {region if region > 1 else count}"
    else:
        label = base if source_id else f"{kind} {count}"
    return label[:MAX_NAME]


def _clean_token(value: str) -> str:
    cleaned = " ".join(value.split())
    return cleaned[:MAX_NAME] if cleaned else "Group"


def _ensure_inside(points: Iterable[dict], frame: Frame) -> None:
    for point in points:
        if (
            point["x"] < -1e-6
            or point["y"] < -1e-6
            or point["x"] > frame.width_mm + 1e-6
            or point["y"] > frame.height_mm + 1e-6
        ):
            raise SvgGeometryError(
                f"A point at ({point['x']:.2f}, {point['y']:.2f}) mm falls outside the "
                f"{frame.width_mm:.2f} by {frame.height_mm:.2f} mm artboard. Move the "
                f"artwork inside the artboard and import again. {UNCHANGED}"
            )


def _closed(points: list[dict]) -> dict:
    return {"points": points, "closed": True, "curved": False}


def _dedupe(points: list[dict]) -> list[dict]:
    kept: list[dict] = []
    for point in points:
        if kept and _near(_xy(kept[-1]), _xy(point), 1e-6):
            continue
        kept.append(point)
    return kept


def _pack(point: tuple[float, float]) -> dict:
    return {"x": _q(point[0]), "y": _q(point[1])}


def _xy(point: dict) -> tuple[float, float]:
    return point["x"], point["y"]


def _q(value: float) -> float:
    rounded = round(float(value) + 0.0, 6)
    if rounded == 0:
        return 0.0
    return rounded


def _near(a: tuple[float, float], b: tuple[float, float], eps: float = 1e-6) -> bool:
    return math.hypot(a[0] - b[0], a[1] - b[1]) <= eps


def _mid(a: tuple[float, float], b: tuple[float, float]) -> tuple[float, float]:
    return ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0)


def _optional_width(value: object) -> float | None:
    if value is None or value == "":
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise SvgGeometryError(
            f"Artboard width must be a number of millimetres. Enter it and import again. {UNCHANGED}"
        )
    return float(value)


def _optional_tolerance(value: object) -> float:
    if value is None or value == "":
        return DEFAULT_TOLERANCE_MM
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise SvgGeometryError(
            f"The flattening tolerance must be a number of millimetres. Enter it and import again. {UNCHANGED}"
        )
    return float(value)


def _dump(value: dict) -> str:
    return json.dumps(value, indent=2, allow_nan=False) + "\n"


def _emit(*, ok: bool, error: str | None = None, **payload: object) -> None:
    body: dict[str, object] = {"ok": ok}
    if error is not None:
        body["error"] = error
    body.update(payload)
    json.dump(body, sys.stdout, allow_nan=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    raise SystemExit(main())
