"""Browser check for independently selectable satin underlay in the studio.

Constant-width and tapered columns use the same four selections. Legacy
Alpine objects stay in the plan. A rejected column does not replace it.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "satin-underlay"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)


def generate(page):
    with page.expect_response("**/api/generate") as generated:
        page.locator("#generate").click()
    response = generated.value
    payload = response.json()
    assert response.status == 200, payload
    page.wait_for_function(
        """() => document.querySelector('#errors').textContent === ''
          && !document.querySelector('#revision').textContent.includes('not generated')
          && document.querySelector('#phases').textContent.includes('Top satin')""",
        timeout=30000,
    )
    return payload


def top_sequence(payload):
    return payload["plan"]["metadata"]["satin_top_sequence"]


def phase_text(page):
    return page.locator("#phases").inner_text()


def main() -> None:
    url = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
    errors: list[str] = []
    console: list[str] = []
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000})
        page.on("dialog", lambda dialog: dialog.accept())
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on(
            "console",
            lambda message: console.append(message.type + ":" + message.text)
            if message.type == "error"
            else None,
        )
        page.goto(url, wait_until="domcontentloaded")
        page.wait_for_function(
            """() => document.querySelector('#project-title').textContent === 'Alpine Club'
              && document.querySelector('#stitch-count').textContent === '4701'""",
            timeout=30000,
        )
        page.locator("#add-object").click()
        page.locator("#fields label", has_text="Stitch type").locator("select").select_option("satin")
        page.wait_for_selector("#centre-run", timeout=5000)
        name = page.locator("#fields label", has_text="Object name").locator("input")
        name.fill("Column eight")
        name.dispatch_event("change")
        points = page.locator("#geometry textarea")
        points.fill("30, 40\n70, 40")
        points.dispatch_event("change")
        assert page.locator("#centre-run").is_checked() is False
        assert page.locator("#edge-run").is_checked() is False
        assert page.locator("#centre-run-length").count() == 0
        none = generate(page)
        none_revision = page.locator("#revision").inner_text()
        page.locator("#centre-run").check()
        page.wait_for_selector("#centre-run-length", timeout=5000)
        shown = page.locator("#centre-run-length").input_value()
        page.locator("#centre-run-length").dispatch_event("change")
        page.locator("#undo").click()
        page.wait_for_function(
            """(revision) => document.querySelector('#revision').textContent === revision
              && !document.querySelector('#centre-run').checked""",
            arg=none_revision,
            timeout=5000,
        )
        page.locator("#redo").click()
        page.wait_for_function(
            """() => document.querySelector('#centre-run').checked
              && document.querySelector('#revision').textContent.includes('not generated')""",
            timeout=5000,
        )
        centre = generate(page)
        page.locator("#edge-run").check()
        page.wait_for_selector("#edge-run-length", timeout=5000)
        both = generate(page)
        page.locator("#centre-run").uncheck()
        edge = generate(page)
        sequences = [top_sequence(item) for item in (none, centre, both, edge)]
        assert len(set(sequences)) == 1, sequences
        hashes = [
            hashlib.sha256(bytes.fromhex(item["dst_hex"])).hexdigest()
            for item in (none, centre, both, edge)
        ]
        assert len(set(hashes)) == 4, hashes
        assert "Centre-run underlay 0" in phase_text(page)
        assert "Edge-run underlay 0" not in phase_text(page)
        assert "Top satin" in phase_text(page)
        assert "Positioning and inter-phase connectors" in phase_text(page)
        repeated = generate(page)
        assert top_sequence(repeated) == sequences[0]
        assert hashlib.sha256(bytes.fromhex(repeated["dst_hex"])).hexdigest() == hashes[3]
        page.locator("#centre-run").check()
        page.locator("#centre-run-length").fill("4")
        page.locator("#centre-run-length").dispatch_event("change")
        page.locator("#edge-run-length").fill("2")
        page.locator("#edge-run-length").dispatch_event("change")
        unequal = generate(page)
        assert top_sequence(unequal) == sequences[0]
        widths = page.locator("#column-widths")
        widths.fill("2\n8")
        widths.dispatch_event("change")
        tapered_both = generate(page)
        assert "variable satin" in " ".join(tapered_both["readiness"]["notes"])
        assert top_sequence(tapered_both) != sequences[0]
        page.locator("#centre-run").uncheck()
        page.locator("#edge-run").uncheck()
        tapered_none = generate(page)
        assert top_sequence(tapered_none) == top_sequence(tapered_both)
        assert "Centre-run underlay 0" in phase_text(page)
        assert "Edge-run underlay 0" in phase_text(page)
        page.locator("#edge-run").check()
        page.locator("#edge-run-length").fill("2")
        page.locator("#edge-run-length").dispatch_event("change")
        tapered_edge = generate(page)
        assert top_sequence(tapered_edge) == top_sequence(tapered_both)
        assert page.locator("#edge-run-length").input_value() == "2"
        assert "Centre-run underlay 0" in phase_text(page)
        assert page.locator("#inspector-name").inner_text() == "Column eight"
        kept = page.locator("#stitch-count").inner_text()
        with page.expect_download() as project_download:
            page.locator("#save-project").click()
        project_path = EVIDENCE / "studio-project.json"
        project_download.value.save_as(project_path)
        saved = json.loads(project_path.read_text())
        column = saved["objects"][-1]
        assert column["label"] == "Column eight"
        assert column["underlay"] == ["edge_run"]
        assert column["edge_run_max_stitch_mm"] == 2
        assert "centre_run_max_stitch_mm" not in column
        assert column["column_widths_mm"] == [2, 8]
        page.locator("#open-project").set_input_files({
            "name": "notes.json",
            "mimeType": "application/json",
            "buffer": b"{",
        })
        page.wait_for_function(
            "() => document.querySelector('#errors').textContent.includes('not JSON')",
            timeout=5000,
        )
        assert page.locator("#stitch-count").inner_text() == kept
        assert page.locator("#project-title").inner_text() == "Alpine Club"
        with page.expect_response("**/api/generate") as reopened:
            page.locator("#open-project").set_input_files(project_path)
        reopened_payload = reopened.value.json()
        assert reopened.value.status == 200, reopened_payload
        page.locator("#objects button", has_text="Column eight").click()
        page.wait_for_function(
            """() => document.querySelector('#inspector-name').textContent === 'Column eight'
              && document.querySelector('#edge-run').checked
              && !document.querySelector('#centre-run').checked
              && document.querySelector('#column-widths').value === '2\\n8'
              && document.querySelector('#edge-run-length').value === '2'
              && document.querySelector('#phases').textContent.includes('Top satin')""",
            timeout=30000,
        )
        assert top_sequence(reopened_payload) == top_sequence(tapered_edge)
        with page.expect_download() as dst_download:
            page.locator("#export").click()
        dst_path = EVIDENCE / "studio-taper-edge.dst"
        dst_download.value.save_as(dst_path)
        dst_sha = hashlib.sha256(dst_path.read_bytes()).hexdigest()
        assert dst_sha == hashlib.sha256(bytes.fromhex(reopened_payload["dst_hex"])).hexdigest()
        assert reopened_payload["readiness"]["machine_release_allowed"] is False
        revision = page.locator("#revision").inner_text()
        page.locator("#centre-run").check()
        page.wait_for_function(
            """() => document.querySelector('#revision').textContent.includes('not generated')
              && document.querySelector('#export').disabled
              && document.querySelector('#phases').textContent.includes('Generate again')
              && document.querySelector('#inspector-name').textContent === 'Column eight'""",
            timeout=5000,
        )
        page.locator("#undo").click()
        page.wait_for_function(
            """(revision) => document.querySelector('#revision').textContent === revision
              && document.querySelector('#inspector-name').textContent === 'Column eight'
              && document.querySelector('#edge-run').checked
              && !document.querySelector('#centre-run').checked""",
            arg=revision,
            timeout=5000,
        )
        page.locator("#redo").click()
        page.wait_for_function(
            """() => document.querySelector('#revision').textContent.includes('not generated')
              && document.querySelector('#centre-run').checked
              && document.querySelector('#inspector-name').textContent === 'Column eight'""",
            timeout=5000,
        )
        page.locator("#undo").click()
        page.locator("#generate").click()
        page.wait_for_function(
            "() => !document.querySelector('#revision').textContent.includes('not generated')",
            timeout=30000,
        )
        kept = page.locator("#stitch-count").inner_text()
        points.fill("20, 30\n50, 30\n20, 30")
        points.dispatch_event("change")
        widths.fill("4\n4\n4")
        widths.dispatch_event("change")
        page.locator("#generate").click()
        page.wait_for_function(
            """() => document.querySelector('#errors').textContent.includes('reverses')
              && document.querySelector('#errors').textContent.includes('not changed')
              && document.querySelector('#revision').textContent.includes('not generated')""",
            timeout=30000,
        )
        assert page.locator("#stitch-count").inner_text() == kept
        assert page.locator("#edge-run").is_checked()
        for name, width, height, dark in (
            ("studio-desktop", 1440, 1000, False),
            ("studio-tablet", 768, 1024, False),
            ("studio-mobile", 390, 844, True),
        ):
            page.set_viewport_size({"width": width, "height": height})
            page.evaluate(
                "(dark) => { document.documentElement.dataset.appearance = dark ? 'dark' : 'light'; }",
                dark,
            )
            page.locator("#centre-run").scroll_into_view_if_needed()
            overflow = page.evaluate(
                "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
            )
            assert overflow, (name, page.evaluate(
                "() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"
            ))
            page.screenshot(path=str(EVIDENCE / f"{name}.png"))
        print("studio_url", url)
        print("studio_constant_sequences_equal", True)
        print("studio_mode_hashes", " ".join(hashes))
        print("studio_taper_edge_dst", dst_sha)
        print("studio_taper_revision", reopened_payload["revision"])
        print("studio_release", False)
        print("studio_unchanged_length_display", shown)
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
