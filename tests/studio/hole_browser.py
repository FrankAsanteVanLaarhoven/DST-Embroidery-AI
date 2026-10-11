"""Served-browser check for the studio hole editor.

Requires the new embroidery-studio binary. The browser does not generate stitches.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
HOLE = "30, 20\n50, 20\n50, 34\n30, 34\n"
TOUCHING = "15, 12\n40, 12\n40, 30\n15, 30\n"


def main() -> None:
    url = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
    out = Path(os.environ.get("STUDIO_HOLE_OUTPUT", ROOT / "evidence" / "m2-g1"))
    out.mkdir(parents=True, exist_ok=True)
    errors = []
    console = []
    generates = []
    with sync_playwright() as playwright:
        options = {"headless": True, "args": ["--no-sandbox", "--disable-dev-shm-usage"]}
        if os.environ.get("BROWSER_EXECUTABLE"):
            options["executable_path"] = os.environ["BROWSER_EXECUTABLE"]
        browser = playwright.chromium.launch(**options)
        page = browser.new_page(viewport={"width": 1440, "height": 900}, device_scale_factor=1)
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on(
            "console",
            lambda message: console.append(message.type + ":" + message.text)
            if message.type == "error"
            else None,
        )

        def keep(response):
            if response.request.method == "POST" and response.url.endswith("/api/generate"):
                generates.append(response)

        page.on("response", keep)
        page.goto(url, wait_until="domcontentloaded")
        page.wait_for_function("() => !document.querySelector('#generate').disabled", timeout=30000)
        page.locator("button[data-view='library']").click()
        page.locator("#open-sample").click()
        page.wait_for_function(
            "() => document.querySelector('#project-title').textContent === 'Club mark' && document.querySelector('#readiness').textContent === 'Geometry checked' && !document.querySelector('#generate').disabled",
            timeout=30000,
        )
        assert page.locator("h1").inner_text() == "Club mark"
        assert page.locator("#object-holes").input_value() == ""

        page.locator("#generate").click()
        page.wait_for_function(
            "() => document.querySelector('#stitch-count').textContent !== 'None yet' && document.querySelector('#readiness').textContent === 'Geometry checked'",
            timeout=30000,
        )
        solid = generates[-1]
        assert solid.status == 200
        solid_body = solid.json()
        assert solid_body["readiness"]["machine_release_allowed"] is False
        assert solid_body["readiness"]["engineering_export_allowed"] is True
        solid_stitches = len(solid_body["plan"]["stitches"])
        regions_before = page.locator("polygon.region").count()
        assert regions_before >= 1
        assert page.locator("#export").is_enabled()

        page.locator("#object-holes").fill(HOLE)
        page.locator("#object-holes").dispatch_event("change")
        page.wait_for_function(
            "() => document.querySelector('#revision').textContent === 'Edits are not generated yet'",
            timeout=5000,
        )
        assert page.locator("#export").is_disabled()
        assert page.locator("polygon.region").count() == regions_before + 1
        assert "not in the stitch plan" in page.locator("#preview-note").inner_text()

        page.locator("#generate").click()
        page.wait_for_function(
            "() => document.querySelector('#readiness').textContent === 'Geometry checked'",
            timeout=30000,
        )
        holed = generates[-1]
        assert holed.status == 200
        holed_body = holed.json()
        assert holed_body["readiness"]["machine_release_allowed"] is False
        kinds = [stitch["kind"] for stitch in holed_body["plan"]["stitches"]]
        assert "jump" in kinds
        assert any(kind in ("fill", "running") for kind in kinds)
        assert len(holed_body["plan"]["stitches"]) != solid_stitches
        dst = bytes.fromhex(holed_body["dst_hex"])
        digest = hashlib.sha256(dst).hexdigest()
        (out / "studio-club-hole.dst").write_bytes(dst)
        page.locator("#mode-stitches").click()
        assert page.locator("line.jump").count() > 0
        assert page.locator("line.stitch").count() > 0
        assert page.locator("#export").is_enabled()
        page.screenshot(path=out / "studio-desktop.png", full_page=True)

        page.locator("#undo").click()
        page.wait_for_function("() => document.querySelector('#object-holes').value === ''", timeout=5000)
        assert page.locator("#export").is_disabled()
        page.locator("#redo").click()
        page.wait_for_function(
            "() => document.querySelector('#object-holes').value.includes('30, 20')",
            timeout=5000,
        )

        page.locator("button", has_text="Right").click()
        page.wait_for_function(
            "() => document.querySelector('#object-holes').value.includes('31, 20')",
            timeout=5000,
        )
        assert page.locator("#revision").inner_text() == "Edits are not generated yet"

        page.locator("#object-holes").fill(TOUCHING)
        page.locator("#object-holes").dispatch_event("change")
        page.locator("#generate").click()
        page.wait_for_function(
            "() => document.querySelector('#errors').textContent.length > 0",
            timeout=30000,
        )
        message = page.locator("#errors").inner_text()
        assert "hole" in message.lower() or "plan was not changed" in message.lower()
        assert page.locator("#revision").inner_text() == "Edits are not generated yet"
        assert page.locator("#export").is_disabled()
        rejected = generates[-1]
        assert rejected.status >= 400

        page.set_viewport_size({"width": 768, "height": 1024})
        page.screenshot(path=out / "studio-tablet.png", full_page=True)
        page.set_viewport_size({"width": 390, "height": 844})
        overflow = page.evaluate(
            "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
        )
        assert overflow
        page.locator("#generate").scroll_into_view_if_needed()
        assert page.locator("#generate").is_enabled()
        page.screenshot(path=out / "studio-mobile.png", full_page=True)
        assert page.locator("h1").inner_text() == "Club mark"
        browser.close()

    unexpected = [line for line in console if "422" not in line and "400" not in line]
    assert not errors, errors
    assert not unexpected, unexpected
    print("studio hole browser checks passed")
    print("studio_hole_dst_sha256", digest)
    print("studio_hole_dst_bytes", len(dst))
    print("rejection", message)
    (out / "studio-hole-result.json").write_text(
        json.dumps(
            {
                "dst_sha256": digest,
                "dst_bytes": len(dst),
                "solid_stitches": solid_stitches,
                "holed_stitches": len(holed_body["plan"]["stitches"]),
                "machine_release_allowed": False,
                "rejection": message,
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
