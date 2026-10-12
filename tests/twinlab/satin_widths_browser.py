"""Variable-width satin through a fresh TwinLab/Compare instance.

Does not use port 4175. The design document is generic-flat-120-v1.
A native embroidery-project is not sent to this service.
"""

from __future__ import annotations

import hashlib
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "satin-variable"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)


def main() -> None:
    url = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4181/")
    errors: list[str] = []
    console: list[str] = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on(
            "console",
            lambda message: console.append(message.type + ":" + message.text)
            if message.type == "error"
            else None,
        )
        page.goto(url, wait_until="domcontentloaded")
        page.wait_for_function(
            "() => window.__twinlab && window.__twinlab.snapshot().artifact_id && window.__twinlab.snapshot().mode === 'native-rust'",
            timeout=30000,
        )
        session = page.evaluate("() => window.__twinlab.snapshot()")
        page.locator("#add-satin").click()
        page.wait_for_selector("#column-widths", timeout=5000)
        page.wait_for_function(
            """() => {
              const shot = window.__twinlab.snapshot();
              const satin = shot.design.objects.find((object) => object.label === 'New satin');
              return shot.artifact_id && !shot.dirty && satin && satin.kind === 'satin' && !satin.widths_mm;
            }""",
            timeout=30000,
        )
        constant = page.evaluate("() => window.__twinlab.snapshot()")
        widths = page.locator("#column-widths")
        widths.fill("2\n6")
        widths.dispatch_event("change")
        page.wait_for_function(
            """(previous) => {
              const shot = window.__twinlab.snapshot();
              const satin = shot.design.objects.find((object) => object.label === 'New satin');
              return shot.artifact_id && shot.artifact_id !== previous && !shot.dirty
                && satin && Array.isArray(satin.widths_mm) && satin.widths_mm[0] === 2 && satin.widths_mm[1] === 6;
            }""",
            arg=constant["artifact_id"],
            timeout=45000,
        )
        varied = page.evaluate("() => window.__twinlab.snapshot()")
        assert varied["provenance"]["dst_sha256"] != constant["provenance"]["dst_sha256"]
        assert varied["design"]["objects"][-1]["widths_mm"] == [2, 6]
        assert varied["compare"]["release_allowed"] is False
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        with page.expect_download() as saved:
            page.locator("#save").click()
        project_path = EVIDENCE / "twinlab-variable.json"
        saved.value.save_as(project_path)
        stored = project_path.read_text()
        assert '"widths_mm"' in stored
        page.locator("#file-input").set_input_files(project_path)
        page.wait_for_function(
            """() => {
              const shot = window.__twinlab.snapshot();
              const satin = shot.design.objects.find((object) => object.label === 'New satin');
              return shot.artifact_id && !shot.dirty && satin && satin.widths_mm && satin.widths_mm[0] === 2 && satin.widths_mm[1] === 6;
            }""",
            timeout=30000,
        )
        reopened = page.evaluate("() => window.__twinlab.snapshot()")
        assert reopened["provenance"]["dst_sha256"] == varied["provenance"]["dst_sha256"]
        page.locator(".object", has_text="New satin").click()
        page.wait_for_selector("#column-widths", timeout=5000)
        page.locator("#attach-reference").click()
        page.locator("#scale-confirm").check()
        page.locator("#proof-generate").click()
        page.wait_for_function(
            "() => window.__twinlab.snapshot().compare.binding_current",
            timeout=30000,
        )
        bound = page.evaluate("() => window.__twinlab.snapshot()")
        assert bound["compare"]["proof_sha256"]
        assert bound["compare"]["release_allowed"] is False
        page.locator("#tab-machine").click()
        assert "DST" in page.locator("#machine-summary").inner_text()
        window = page.locator("#machine-window").inner_text()
        assert "raw" in window and "byte_offset" not in window
        assert "to " in window
        page.locator("#tab-twin").click()
        total = bound["trace"]["duration_ms"]
        for fraction in (0, 0.5, 1):
            page.locator("#scrub").evaluate(
                "(el, value) => { el.value = String(value); el.dispatchEvent(new Event('input', {bubbles: true})); }",
                total * fraction,
            )
            page.wait_for_timeout(60)
            design_index = page.locator("#design-canvas").get_attribute("data-event-index")
            machine_index = page.locator("#machine-canvas").get_attribute("data-event-index")
            state = page.evaluate("() => window.__twinlab.snapshot().state")
            assert design_index == machine_index == str(state["event_index"])
            assert "Shared clock" in page.locator("#twin-event").inner_text()
        assert page.locator("#design-canvas").get_attribute("data-event-index") != ""
        page.locator("#proof-reopen").click()
        page.wait_for_function(
            "() => document.getElementById('message').textContent.includes('Reopened')",
            timeout=10000,
        )
        widths.fill("2\n8")
        widths.dispatch_event("change")
        page.wait_for_function(
            """() => {
              const shot = window.__twinlab.snapshot();
              return shot.dirty && shot.compare.binding_current === false;
            }""",
            timeout=5000,
        )
        assert page.locator("#download").is_disabled()
        assert "not current" in page.locator("#compare-status").inner_text()
        page.locator("#undo").click()
        page.wait_for_function(
            """() => {
              const shot = window.__twinlab.snapshot();
              const satin = shot.design.objects.find((object) => object.label === 'New satin');
              return document.querySelector('#column-widths')
                && document.querySelector('#column-widths').value === '2\\n6'
                && satin && satin.widths_mm[0] === 2 && satin.widths_mm[1] === 6;
            }""",
            timeout=5000,
        )
        page.locator("#redo").click()
        page.wait_for_function(
            "() => document.querySelector('#column-widths').value === '2\\n8'",
            timeout=5000,
        )
        page.wait_for_function("() => !window.__twinlab.snapshot().dirty", timeout=45000)
        with page.expect_download(timeout=15000) as dst_info:
            page.locator("#download").click()
        dst_path = EVIDENCE / "twinlab-variable.dst"
        dst_info.value.save_as(dst_path)
        current = page.evaluate("() => window.__twinlab.snapshot()")
        assert hashlib.sha256(dst_path.read_bytes()).hexdigest() == current["provenance"]["dst_sha256"]
        assert current["provenance"]["dst_sha256"] != varied["provenance"]["dst_sha256"]
        print("twin_mode", session["mode"])
        print("twin_profile", "generic-flat-120-v1")
        print("twin_design_sha256", current["provenance"]["design_sha256"])
        print("twin_dst_sha256", current["provenance"]["dst_sha256"])
        print("twin_engine_sha256", current["provenance"].get("engine_sha256"))
        print("twin_artifact", current["artifact_id"])
        print("constant_dst", constant["provenance"]["dst_sha256"])
        print("listed_dst", varied["provenance"]["dst_sha256"])
        page.screenshot(path=str(EVIDENCE / "twinlab-desktop.png"), full_page=True)
        page.set_viewport_size({"width": 768, "height": 1024})
        page.wait_for_timeout(80)
        assert page.evaluate("() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1")
        page.screenshot(path=str(EVIDENCE / "twinlab-tablet.png"), full_page=True)
        page.set_viewport_size({"width": 390, "height": 844})
        page.wait_for_timeout(80)
        assert page.evaluate("() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1")
        page.screenshot(path=str(EVIDENCE / "twinlab-mobile.png"), full_page=True)
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item and "404" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
