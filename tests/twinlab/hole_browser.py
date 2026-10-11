"""Served-browser check for one editable fill with a hole.

Requires the native TwinLab server. The browser does not generate stitches.
"""
from __future__ import annotations

import hashlib
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
DESIGN = ROOT / "fixtures" / "fill-holes" / "designs" / "twin-square-hole.json"


def snap(page):
    return page.evaluate("() => window.__twinlab.snapshot()")


def wait_compiled(page, previous):
    page.wait_for_function(
        """(previous) => {
          const shot = window.__twinlab.snapshot();
          return !shot.dirty && shot.artifact_id && shot.provenance
            && shot.provenance.dst_sha256 !== previous;
        }""",
        arg=previous,
        timeout=30000,
    )
    return snap(page)


def edit_hole_x(page, value):
    page.locator("#ring").select_option("1")
    page.locator("#point-x").fill(str(value))
    page.locator("#point-x").dispatch_event("change")


def main() -> None:
    url = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4175/")
    out = Path(os.environ.get("HOLE_OUTPUT", ROOT / "evidence" / "m2-g1"))
    out.mkdir(parents=True, exist_ok=True)
    errors = []
    console = []
    with sync_playwright() as playwright:
        options = {"headless": True, "args": ["--no-sandbox", "--disable-dev-shm-usage"]}
        if os.environ.get("BROWSER_EXECUTABLE"):
            options["executable_path"] = os.environ["BROWSER_EXECUTABLE"]
        browser = playwright.chromium.launch(**options)
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, device_scale_factor=1, accept_downloads=True)
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on("console", lambda message: console.append(message.type + ":" + message.text) if message.type == "error" else None)
        page.goto(url, wait_until="networkidle")
        page.wait_for_function("() => window.__twinlab && window.__twinlab.snapshot().artifact_id", timeout=30000)
        alpine = snap(page)
        assert alpine["mode"] == "native-rust", alpine["mode"]
        assert alpine["design"]["name"] == "Alpine mark"
        assert "holes" not in alpine["design"]["objects"][0]
        alpine_hash = alpine["provenance"]["dst_sha256"]

        page.locator("#file-input").set_input_files(DESIGN)
        holed = wait_compiled(page, alpine_hash)
        assert holed["design"]["objects"][0]["holes"][0][0]["x"] == -6
        assert holed["provenance"]["dst_sha256"] != alpine_hash
        hole_hash = holed["provenance"]["dst_sha256"]

        with page.expect_download() as saved_info:
            page.locator("#save").click()
        saved_path = out / "square-hole.json"
        saved_info.value.save_as(saved_path)
        page.locator("#file-input").set_input_files(saved_path)
        reopened = wait_compiled(page, "")
        assert reopened["design"]["objects"][0]["holes"][0][0]["x"] == -6
        assert reopened["provenance"]["dst_sha256"] == hole_hash

        page.locator("#attach-reference").click()
        page.wait_for_function("() => window.__twinlab.snapshot().compare.original_sha256", timeout=10000)
        page.locator("#scale-confirm").check()
        page.locator("#proof-generate").click()
        page.wait_for_function("() => window.__twinlab.snapshot().compare.binding_current", timeout=30000)
        page.locator("#proof-reopen").click()
        page.wait_for_function('() => document.getElementById("message").textContent.includes("Reopened")', timeout=10000)
        bound = snap(page)
        assert bound["compare"]["release_allowed"] is False
        assert bound["compare"]["approval_allowed"] is False

        edit_hole_x(page, -4)
        page.wait_for_timeout(50)
        stale = snap(page)
        assert stale["dirty"] is True
        assert stale["compare"]["binding_current"] is False
        assert page.locator("#download").is_disabled()
        assert page.locator("#proof-download").is_disabled()
        assert "not current" in page.locator("#compare-status").inner_text()
        revised = wait_compiled(page, hole_hash)
        assert revised["design"]["objects"][0]["holes"][0][0]["x"] == -4
        revised_hash = revised["provenance"]["dst_sha256"]
        with page.expect_download() as dst_info:
            page.locator("#download").click()
        dst_path = out / "square-hole.dst"
        dst_info.value.save_as(dst_path)
        assert hashlib.sha256(dst_path.read_bytes()).hexdigest() == revised_hash

        page.locator("#scale-confirm").check()
        page.locator("#proof-generate").click()
        page.wait_for_function("() => window.__twinlab.snapshot().compare.binding_current", timeout=30000)
        page.locator("#proof-reopen").click()
        page.wait_for_function('() => document.getElementById("message").textContent.includes("Reopened")', timeout=10000)
        with page.expect_download() as pdf_info:
            page.locator("#proof-download").click()
        pdf_path = out / "square-hole-proof.pdf"
        pdf_info.value.save_as(pdf_path)
        assert hashlib.sha256(pdf_path.read_bytes()).hexdigest() == snap(page)["compare"]["proof_sha256"]

        page.locator("#undo").click()
        restored = wait_compiled(page, revised_hash)
        assert restored["design"]["objects"][0]["holes"][0][0]["x"] == -6
        assert restored["provenance"]["dst_sha256"] == hole_hash

        edit_hole_x(page, -20)
        page.wait_for_function(
            '() => { const text = document.getElementById("message").textContent.toLowerCase(); return text.includes("hole") && text.includes("again"); }',
            timeout=15000,
        )
        rejected = snap(page)
        assert rejected["dirty"] is True
        assert rejected["artifact_id"] is None
        assert page.locator("#download").is_disabled()
        assert "not current" in page.locator("#compare-status").inner_text().lower() or "not compiled" in page.locator("#compare-status").inner_text().lower()
        page.locator("#undo").click()
        recovered = wait_compiled(page, "")
        assert recovered["design"]["objects"][0]["holes"][0][0]["x"] == -6
        assert recovered["dirty"] is False

        page.screenshot(path=str(out / "desktop.png"), full_page=True)
        page.set_viewport_size({"width": 768, "height": 1024})
        page.wait_for_timeout(100)
        edit_hole_x(page, -5)
        tablet = wait_compiled(page, hole_hash)
        assert tablet["design"]["objects"][0]["holes"][0][0]["x"] == -5
        page.screenshot(path=str(out / "tablet.png"), full_page=True)
        page.locator("#undo").click()
        wait_compiled(page, tablet["provenance"]["dst_sha256"])

        page.set_viewport_size({"width": 390, "height": 844})
        page.wait_for_timeout(100)
        assert page.evaluate("() => document.documentElement.scrollWidth <= window.innerWidth + 1")
        edit_hole_x(page, -3)
        mobile = wait_compiled(page, hole_hash)
        assert mobile["design"]["objects"][0]["holes"][0][0]["x"] == -3
        assert page.locator("#add-hole").is_visible()
        assert page.locator("#download").is_enabled()
        page.screenshot(path=str(out / "mobile.png"), full_page=True)
        assert page.locator("h1").inner_text() == "See every stitch. Understand every move."
        assert not errors, errors
        unexpected = [line for line in console if "422" not in line]
        assert not unexpected, unexpected
        assert console, "the rejected hole should surface as a failed compile response"
        browser.close()
    print("hole browser checks passed")
    print("dst", hashlib.sha256(dst_path.read_bytes()).hexdigest())
    print("pdf", hashlib.sha256(pdf_path.read_bytes()).hexdigest())


if __name__ == "__main__":
    main()
