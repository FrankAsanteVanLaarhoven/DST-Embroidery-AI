"""Geometry checks for the bounded SVG intake.

These tests do not generate stitches and do not accept A08, G02, or G03.
"""

from __future__ import annotations

import hashlib
import json
import math
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python"))

from embroidery_ai.svg_geometry import SvgGeometryError, convert_svg

FIXTURES = ROOT / "fixtures" / "svg-geometry"
PYTHON = ROOT / ".venv" / "bin" / "python"
CLI = ROOT / "tools" / "import_svg_geometry.py"


def svg(body: str, **attrs: str) -> str:
    extra = " ".join(f'{key}="{value}"' for key, value in attrs.items())
    return (
        '<?xml version="1.0" encoding="UTF-8"?>'
        f'<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" '
        f'viewBox="0 0 100 100" {extra}>{body}</svg>'
    )


def convert(body: str, **kwargs):
    return convert_svg(body, **kwargs)


def point(obj, index):
    return obj["points"][index]["x"], obj["points"][index]["y"]


class SvgGeometryTests(unittest.TestCase):
    def test_project_shape_matches_native_v1(self):
        result = convert(svg('<rect id="field" x="10" y="10" width="20" height="15"/>'))
        project = result["project"]
        self.assertEqual(project["format"], "embroidery-project")
        self.assertEqual(project["format_version"], "1")
        self.assertIsNone(project["review"])
        obj = project["objects"][0]
        self.assertEqual(
            set(obj),
            {
                "object_id",
                "label",
                "kind",
                "points",
                "angle_degrees",
                "density_mm",
                "column_width_mm",
                "max_stitch_mm",
                "underlay_spacing_mm",
                "underlay",
                "pull_compensation_mm",
                "color_index",
            },
        )
        self.assertNotIn("holes", obj)
        self.assertNotIn("rgb", obj)
        self.assertNotIn("groups", obj)
        self.assertNotIn("stitches", project)
        self.assertNotIn("dst", project)

    def test_source_sha256_matches_exact_bytes(self):
        body = svg('<rect x="10" y="10" width="20" height="20"/>')
        raw = body.encode("utf-8")
        result = convert_svg(raw)
        self.assertEqual(result["manifest"]["source_sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(result["manifest"]["source_byte_length"], len(raw))

    def test_physical_millimetre_artboard(self):
        result = convert(
            '<svg xmlns="http://www.w3.org/2000/svg" width="10cm" height="8cm" viewBox="0 0 100 80">'
            '<rect x="0" y="0" width="100" height="80" fill="#000000"/>'
            "</svg>"
        )
        self.assertEqual(result["project"]["hoop_width_mm"], 100)
        self.assertEqual(result["project"]["hoop_height_mm"], 80)
        self.assertEqual(result["manifest"]["artboard"]["width_source"], "svg-absolute")
        self.assertEqual(point(result["project"]["objects"][0], 1), (100, 0))

    def test_pixel_artboard_requires_width(self):
        body = '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="10" height="10"/></svg>'
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("pixels", str(caught.exception))
        self.assertIn("not changed", str(caught.exception))

    def test_pixel_artboard_uses_supplied_width_and_aspect(self):
        body = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">'
            '<rect x="0" y="0" width="200" height="100"/>'
            "</svg>"
        )
        result = convert(body, width_mm=50)
        self.assertEqual(result["project"]["hoop_width_mm"], 50)
        self.assertEqual(result["project"]["hoop_height_mm"], 25)
        self.assertEqual(result["manifest"]["artboard"]["width_source"], "width-mm-argument")
        self.assertEqual(point(result["project"]["objects"][0], 2), (50, 25))

    def test_viewbox_origin_maps_into_the_hoop(self):
        body = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="10 20 100 100">'
            '<rect x="10" y="20" width="40" height="30"/>'
            "</svg>"
        )
        result = convert(body)
        self.assertEqual(point(result["project"]["objects"][0], 0), (0, 0))
        self.assertEqual(point(result["project"]["objects"][0], 2), (40, 30))

    def test_meet_letterboxes_a_wide_viewbox(self):
        body = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" '
            'viewBox="0 0 200 100" preserveAspectRatio="xMidYMid meet">'
            '<rect x="0" y="0" width="200" height="100"/>'
            "</svg>"
        )
        result = convert(body)
        self.assertEqual(point(result["project"]["objects"][0], 0), (0, 25))
        self.assertEqual(point(result["project"]["objects"][0], 2), (100, 75))

    def test_xmin_ymin_meet_aligns_to_the_origin(self):
        body = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" '
            'viewBox="0 0 200 100" preserveAspectRatio="xMinYMin meet">'
            '<rect x="0" y="0" width="200" height="100"/>'
            "</svg>"
        )
        result = convert(body)
        self.assertEqual(point(result["project"]["objects"][0], 0), (0, 0))
        self.assertEqual(point(result["project"]["objects"][0], 2), (100, 50))

    def test_none_stretches_each_axis(self):
        body = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" '
            'viewBox="0 0 200 100" preserveAspectRatio="none">'
            '<rect x="0" y="0" width="200" height="100"/>'
            "</svg>"
        )
        result = convert(body)
        self.assertEqual(point(result["project"]["objects"][0], 1), (100, 0))
        self.assertEqual(point(result["project"]["objects"][0], 2), (100, 100))

    def test_slice_is_rejected(self):
        body = (
            '<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" '
            'viewBox="0 0 200 100" preserveAspectRatio="xMidYMid slice">'
            '<rect width="10" height="10"/>'
            "</svg>"
        )
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("slice", str(caught.exception))

    def test_transform_list_applies_the_rightmost_function_first(self):
        result = convert(svg('<rect x="0" y="0" width="10" height="10" transform="translate(20 10) rotate(90)"/>'))
        self.assertEqual(point(result["project"]["objects"][0], 0), (20, 10))
        self.assertEqual(point(result["project"]["objects"][0], 1), (20, 20))
        self.assertEqual(point(result["project"]["objects"][0], 2), (10, 20))

    def test_parent_transform_is_applied_after_the_child(self):
        result = convert(
            svg(
                '<g transform="translate(10 0)">'
                '<rect x="0" y="0" width="10" height="10" transform="translate(5 0)"/>'
                "</g>"
            )
        )
        self.assertEqual(point(result["project"]["objects"][0], 0), (15, 0))
        self.assertEqual(point(result["project"]["objects"][0], 1), (25, 0))

    def test_rotation_uses_the_pivot(self):
        result = convert(svg('<rect x="0" y="0" width="10" height="10" transform="rotate(90 5 5)"/>'))
        self.assertEqual(point(result["project"]["objects"][0], 0), (10, 0))
        self.assertEqual(point(result["project"]["objects"][0], 1), (10, 10))
        self.assertEqual(point(result["project"]["objects"][0], 2), (0, 10))
        self.assertEqual(point(result["project"]["objects"][0], 3), (0, 0))

    def test_reflection_across_a_vertical_axis(self):
        result = convert(
            svg(
                '<rect x="60" y="10" width="10" height="10" '
                'transform="translate(50 0) scale(-1 1) translate(-50 0)"/>'
            )
        )
        xs = sorted(point(result["project"]["objects"][0], index)[0] for index in range(4))
        self.assertEqual(xs, [30, 30, 40, 40])

    def test_skew_x_shears_the_rectangle(self):
        result = convert(svg('<rect x="10" y="10" width="10" height="10" transform="skewX(45)"/>'))
        self.assertEqual(point(result["project"]["objects"][0], 0), (20, 10))
        self.assertEqual(point(result["project"]["objects"][0], 1), (30, 10))
        self.assertEqual(point(result["project"]["objects"][0], 2), (40, 20))

    def test_shapes_and_explicit_running_centreline(self):
        body = svg(
            '<circle cx="20" cy="20" r="8" fill="#112233"/>'
            '<ellipse cx="50" cy="20" rx="8" ry="4" fill="#112233"/>'
            '<polygon points="10,40 20,40 15,50" fill="#112233"/>'
            '<line x1="5" y1="70" x2="25" y2="70" fill="none" stroke="#445566"/>'
            '<polyline points="40,70 50,80 60,70" fill="none" stroke="#445566"/>'
        )
        with self.assertRaises(SvgGeometryError):
            convert(body)
        result = convert(body, running_strokes=True)
        kinds = [obj["kind"] for obj in result["project"]["objects"]]
        self.assertEqual(kinds, ["fill", "fill", "fill", "running", "running"])
        self.assertGreater(len(result["project"]["objects"][0]["points"]), 8)
        self.assertEqual(result["project"]["objects"][3]["points"][0]["y"], 70)
        self.assertTrue(any("centreline" in loss.lower() or "centerline" in loss.lower() for loss in result["losses"]))

    def test_stroke_only_path_requires_the_centreline_flag(self):
        body = svg('<path d="M10 10 L40 10" fill="none" stroke="#000000"/>')
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("centreline", str(caught.exception).lower())
        result = convert(body, running_strokes=True)
        self.assertEqual(result["project"]["objects"][0]["kind"], "running")
        self.assertEqual(len(result["project"]["objects"][0]["points"]), 2)

    def test_filled_shape_records_stroke_as_a_loss(self):
        result = convert(svg('<rect x="10" y="10" width="20" height="20" fill="#000000" stroke="#ff0000" stroke-width="3"/>'))
        self.assertEqual(result["project"]["objects"][0]["kind"], "fill")
        self.assertTrue(any("stroke" in loss.lower() for loss in result["losses"]))

    def test_cubic_samples_stay_within_the_tolerance(self):
        tolerance = 0.05
        body = svg('<path fill="#000" d="M10 40 C10 10 90 10 90 40 C90 70 10 70 10 40 Z"/>')
        result = convert(body, flatten_tolerance_mm=tolerance)
        points = result["project"]["objects"][0]["points"]
        self.assertGreater(len(points), 4)
        deviation = _max_curve_deviation(
            [(10, 40), (10, 10), (90, 10), (90, 40)],
            points,
        )
        self.assertLessEqual(deviation, tolerance + 1e-4)
        self.assertTrue(any("flattened" in loss.lower() for loss in result["losses"]))
        self.assertTrue(result["manifest"]["objects"][0]["flattened"])

    def test_quadratic_samples_stay_within_the_tolerance(self):
        tolerance = 0.05
        body = svg('<path fill="#000" d="M10 60 Q50 10 90 60 L10 60 Z"/>')
        result = convert(body, flatten_tolerance_mm=tolerance)
        points = result["project"]["objects"][0]["points"]
        deviation = _max_quadratic_deviation((10, 60), (50, 10), (90, 60), points)
        self.assertLessEqual(deviation, tolerance + 1e-4)

    def test_smooth_cubic_matches_the_explicit_control(self):
        smooth = svg('<path fill="#000" d="M10 30 C10 10 30 10 40 30 S70 50 80 30 L80 70 L10 70 Z"/>')
        explicit = svg('<path fill="#000" d="M10 30 C10 10 30 10 40 30 C50 50 70 50 80 30 L80 70 L10 70 Z"/>')
        left = convert(smooth)["project"]["objects"][0]["points"]
        right = convert(explicit)["project"]["objects"][0]["points"]
        self.assertEqual(left, right)

    def test_smooth_quadratic_matches_the_explicit_control(self):
        smooth = svg('<path fill="#000" d="M10 60 Q20 30 40 60 T70 60 L70 80 L10 80 Z"/>')
        explicit = svg('<path fill="#000" d="M10 60 Q20 30 40 60 Q60 90 70 60 L70 80 L10 80 Z"/>')
        self.assertEqual(
            convert(smooth)["project"]["objects"][0]["points"],
            convert(explicit)["project"]["objects"][0]["points"],
        )

    def test_evenodd_compound_path_keeps_one_hole(self):
        result = convert((FIXTURES / "badge-hole.svg").read_text(encoding="utf-8"))
        obj = result["project"]["objects"][0]
        self.assertEqual(obj["kind"], "fill")
        self.assertEqual(len(result["project"]["objects"]), 1)
        self.assertEqual(len(obj["holes"]), 1)
        self.assertEqual(obj["points"][0], {"x": 10, "y": 10})
        self.assertEqual(obj["holes"][0][0], {"x": 30, "y": 30})
        self.assertEqual(result["manifest"]["objects"][0]["fill_rule"], "evenodd")

    def test_nonzero_opposite_winding_is_a_hole(self):
        body = svg(
            '<path fill="#000" fill-rule="nonzero" '
            'd="M10 10 H90 V90 H10 Z M30 30 V70 H70 V30 Z"/>'
        )
        result = convert(body)
        self.assertEqual(len(result["project"]["objects"][0]["holes"]), 1)

    def test_nonzero_same_winding_is_rejected(self):
        body = svg(
            '<path fill="#000" fill-rule="nonzero" '
            'd="M10 10 H90 V90 H10 Z M30 30 H70 V70 H30 Z"/>'
        )
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("winding", str(caught.exception).lower())

    def test_disjoint_outers_become_separate_objects(self):
        body = svg('<path fill="#000" d="M5 5 H20 V20 H5 Z M25 5 H40 V20 H25 Z"/>')
        result = convert(body)
        self.assertEqual(len(result["project"]["objects"]), 2)
        self.assertNotIn("holes", result["project"]["objects"][0])
        self.assertNotIn("holes", result["project"]["objects"][1])

    def test_touching_rings_are_rejected(self):
        body = svg('<path fill="#000" fill-rule="evenodd" d="M10 10 H50 V50 H10 Z M30 10 H40 V30 H30 Z"/>')
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("touch", str(caught.exception).lower())

    def test_overlapping_rings_are_rejected(self):
        body = svg('<path fill="#000" fill-rule="evenodd" d="M10 10 H40 V40 H10 Z M30 30 H60 V60 H30 Z"/>')
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("touch", str(caught.exception).lower())

    def test_deeper_nesting_is_rejected(self):
        body = svg(
            '<path fill="#000" fill-rule="evenodd" '
            'd="M10 10 H90 V90 H10 Z M20 20 H80 V80 H20 Z M30 30 H70 V70 H30 Z"/>'
        )
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("nested", str(caught.exception).lower())

    def test_paint_inheritance_and_palette_sidecar(self):
        body = svg(
            '<g fill="#ff0000">'
            '<rect x="10" y="10" width="10" height="10"/>'
            '<rect x="30" y="10" width="10" height="10" fill="#00ff00"/>'
            "</g>"
        )
        result = convert(body)
        self.assertEqual(
            result["manifest"]["palette"],
            [
                {"color_index": 0, "source": "#ff0000"},
                {"color_index": 1, "source": "#00ff00"},
            ],
        )
        self.assertEqual(result["project"]["objects"][0]["color_index"], 0)
        self.assertEqual(result["project"]["objects"][1]["color_index"], 1)
        self.assertNotIn("source", result["project"]["objects"][0])

    def test_default_fill_is_black(self):
        result = convert(svg('<rect x="10" y="10" width="10" height="10"/>'))
        self.assertEqual(result["manifest"]["palette"][0]["source"], "#000000")

    def test_review_and_machine_authority_are_absent(self):
        result = convert(svg('<rect x="10" y="10" width="10" height="10"/>'))
        manifest = result["manifest"]
        self.assertIsNone(result["project"]["review"])
        self.assertIsNone(manifest["review"])
        self.assertFalse(manifest["machine_release_allowed"])
        self.assertFalse(manifest["approval_allowed"])
        self.assertIsNone(manifest["timing_model"])
        self.assertEqual(manifest["coordinate_frame"], "native-hoop-nonnegative-v1")
        self.assertIn("generic-flat-120-v1", manifest["not_interchangeable_with"])
        self.assertTrue(any("Machine release" in loss for loss in result["losses"]))
        self.assertTrue(any("1.2 mm" in loss for loss in result["losses"]))

    def test_malformed_xml_is_rejected(self):
        with self.assertRaises(SvgGeometryError) as caught:
            convert("<svg><rect></svg>")
        self.assertIn("well-formed", str(caught.exception))

    def test_text_image_gradient_clip_and_arc_are_rejected(self):
        cases = [
            (svg("<text>Hello</text>"), "text"),
            (svg('<image href="mark.png"/>'), "image"),
            (svg("<defs><linearGradient id='g'/></defs><rect width='10' height='10' fill='url(#g)'/>"), "gradient"),
            (svg("<clipPath id='c'><rect width='10' height='10'/></clipPath><rect width='10' height='10'/>"), "clip"),
            (svg('<path d="M10 10 A10 10 0 0 1 20 20 Z"/>'), "arc"),
        ]
        for body, needle in cases:
            with self.subTest(needle=needle):
                with self.assertRaises(SvgGeometryError) as caught:
                    convert(body)
                self.assertIn(needle, str(caught.exception).lower())
                self.assertIn("not changed", str(caught.exception))

    def test_external_reference_is_rejected(self):
        with self.assertRaises(SvgGeometryError) as caught:
            convert(svg('<rect x="0" y="0" width="10" height="10" href="https://example.invalid/a.svg"/>'))
        self.assertIn("reference", str(caught.exception).lower())

    def test_doctype_and_entity_are_rejected(self):
        body = '<!DOCTYPE svg [<!ENTITY x "10">]><svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm">&x;</svg>'
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("entities", str(caught.exception).lower())

    def test_processing_instruction_is_rejected(self):
        body = '<?xml-stylesheet type="text/css" href="a.css"?>' + svg('<rect width="10" height="10"/>')
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("processing", str(caught.exception).lower())

    def test_cdata_is_rejected(self):
        body = svg("<rect width='10' height='10'/><![CDATA[hidden]]>")
        with self.assertRaises(SvgGeometryError) as caught:
            convert(body)
        self.assertIn("CDATA", str(caught.exception))

    def test_document_budgets(self):
        with self.assertRaises(SvgGeometryError) as caught:
            convert_svg(b"<svg>" + b" " * 400_001)
        self.assertIn("larger than", str(caught.exception))
        deep = "<svg xmlns='http://www.w3.org/2000/svg' width='10mm' height='10mm'>" + "<g>" * 20 + "<rect width='1' height='1'/>" + "</g>" * 20 + "</svg>"
        with self.assertRaises(SvgGeometryError) as caught:
            convert(deep)
        self.assertIn("16", str(caught.exception))

    def test_geometry_is_deterministic(self):
        body = (FIXTURES / "cubic-leaf.svg").read_text(encoding="utf-8")
        self.assertEqual(convert(body), convert(body))

    def test_anonymous_group_identity_is_shared_by_siblings(self):
        body = svg(
            "<g>"
            '<rect id="a" x="5" y="5" width="10" height="10"/>'
            '<rect id="b" x="20" y="5" width="10" height="10"/>'
            "</g>"
            "<g>"
            '<rect id="c" x="40" y="5" width="10" height="10"/>'
            "</g>"
        )
        records = convert(body)["manifest"]["objects"]
        self.assertEqual(records[0]["groups"], ["anonymous-group-1"])
        self.assertEqual(records[1]["groups"], ["anonymous-group-1"])
        self.assertEqual(records[2]["groups"], ["anonymous-group-2"])

    def test_named_group_ancestry_is_retained(self):
        body = svg(
            '<g id="outer">'
            '<g id="inner">'
            '<rect id="mark" x="10" y="10" width="10" height="10"/>'
            "</g></g>"
        )
        record = convert(body)["manifest"]["objects"][0]
        self.assertEqual(record["groups"], ["outer", "inner"])
        self.assertEqual(record["source_id"], "mark")
        self.assertNotIn("groups", convert(body)["project"]["objects"][0])

    def test_leaf_fixture_is_one_closed_fill(self):
        result = convert((FIXTURES / "cubic-leaf.svg").read_text(encoding="utf-8"))
        obj = result["project"]["objects"][0]
        self.assertEqual(len(result["project"]["objects"]), 1)
        self.assertEqual(obj["kind"], "fill")
        self.assertNotIn("holes", obj)
        self.assertGreaterEqual(len(obj["points"]), 8)
        self.assertEqual(result["project"]["name"], "cubic-leaf")

    def test_opacity_style_and_class_are_rejected(self):
        for body, needle in (
            (svg('<rect x="10" y="10" width="10" height="10" opacity="0.5"/>'), "opacity"),
            (svg('<rect x="10" y="10" width="10" height="10" style="fill:#000"/>'), "style"),
            (svg('<rect class="mark" x="10" y="10" width="10" height="10"/>'), "class"),
        ):
            with self.subTest(needle=needle):
                with self.assertRaises(SvgGeometryError) as caught:
                    convert(body)
                self.assertIn(needle, str(caught.exception).lower())

    def test_cli_writes_exact_source_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            dest = root / "bundle"
            completed = subprocess.run(
                [str(PYTHON), str(CLI), str(FIXTURES / "badge-hole.svg"), str(dest)],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            source = (FIXTURES / "badge-hole.svg").read_bytes()
            self.assertEqual((dest / "source.svg").read_bytes(), source)
            project = json.loads((dest / "project.json").read_text(encoding="utf-8"))
            manifest = json.loads((dest / "import-manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(project["format"], "embroidery-project")
            self.assertIsNone(project["review"])
            self.assertEqual(manifest["source_sha256"], hashlib.sha256(source).hexdigest())
            self.assertFalse(manifest["machine_release_allowed"])
            self.assertNotIn("dst", project)
            again = subprocess.run(
                [str(PYTHON), str(CLI), str(FIXTURES / "badge-hole.svg"), str(dest)],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(again.returncode, 2)
            self.assertIn("not empty", again.stderr)
            self.assertEqual((dest / "source.svg").read_bytes(), source)

    def test_cli_failure_writes_no_bundle(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "text.svg"
            dest = Path(temp) / "out"
            source.write_text(svg("<text>Hello</text>"), encoding="utf-8")
            completed = subprocess.run(
                [str(PYTHON), str(CLI), str(source), str(dest)],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(dest.exists())

    def test_points_outside_the_artboard_are_rejected(self):
        with self.assertRaises(SvgGeometryError) as caught:
            convert(svg('<rect x="-5" y="10" width="10" height="10"/>'))
        self.assertIn("outside", str(caught.exception))

    def test_rounded_rectangles_are_rejected(self):
        with self.assertRaises(SvgGeometryError) as caught:
            convert(svg('<rect x="10" y="10" width="20" height="20" rx="2"/>'))
        self.assertIn("Rounded", str(caught.exception))


def _max_curve_deviation(cubic, polyline) -> float:
    worst = 0.0
    for step in range(1, 201):
        sample = _cubic(cubic, step / 200)
        worst = max(worst, _distance_to_polyline(sample, polyline))
    return worst


def _max_quadratic_deviation(p0, p1, p2, polyline) -> float:
    worst = 0.0
    for step in range(1, 201):
        t = step / 200
        sample = (
            (1 - t) * (1 - t) * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0],
            (1 - t) * (1 - t) * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1],
        )
        worst = max(worst, _distance_to_polyline(sample, polyline))
    return worst


def _cubic(points, t: float):
    p0, p1, p2, p3 = points
    u = 1 - t
    return (
        u * u * u * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t * t * t * p3[0],
        u * u * u * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t * t * t * p3[1],
    )


def _distance_to_polyline(point, polyline) -> float:
    best = math.inf
    for index in range(len(polyline) - 1):
        best = min(best, _distance_to_segment(point, polyline[index], polyline[index + 1]))
    return best


def _distance_to_segment(point, start, end) -> float:
    ax, ay = start["x"], start["y"]
    bx, by = end["x"], end["y"]
    dx, dy = bx - ax, by - ay
    length = dx * dx + dy * dy
    if length <= 1e-18:
        return math.hypot(point[0] - ax, point[1] - ay)
    t = min(1.0, max(0.0, ((point[0] - ax) * dx + (point[1] - ay) * dy) / length))
    return math.hypot(point[0] - (ax + t * dx), point[1] - (ay + t * dy))


if __name__ == "__main__":
    unittest.main()
