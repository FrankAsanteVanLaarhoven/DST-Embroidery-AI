"""Browser check for variable-width satin editing.

Adds one satin column to Alpine Club, generates it, then rejects a width
list that does not match the centreline.
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
    url = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
    errors: list[str] = []
    console: list[str] = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000})
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
        page.wait_for_function(
            "() => document.querySelector('#inspector-name').textContent === 'New region'",
            timeout=5000,
        )
        page.locator("#fields label", has_text="Stitch type").locator("select").select_option("satin")
        page.wait_for_selector("#column-widths", timeout=5000)
        points = page.locator("#geometry textarea")
        points.fill("30, 40\n70, 40")
        points.dispatch_event("change")
        widths = page.locator("#column-widths")
        widths.fill("2\n8")
        widths.dispatch_event("change")
        with page.expect_response("**/api/generate") as generated:
            page.locator("#generate").click()
        payload = generated.value.json()
        page.wait_for_function(
            """() => {
              const count = Number(document.querySelector('#stitch-count').textContent);
              const note = document.querySelector('#preview-note').textContent;
              return count > 4701
                && note.includes('Machine release remains unavailable')
                && !document.querySelector('#revision').textContent.includes('Edits');
            }""",
            timeout=30000,
        )
        assert page.locator("#errors").inner_text() == ""
        by_object = {}
        for stitch in payload["plan"]["stitches"]:
            bucket = by_object.setdefault(str(stitch["object_id"]), {"sewing": 0, "travel": 0})
            if stitch["kind"] in {"jump", "trim", "color_change"}:
                bucket["travel"] += 1
            else:
                bucket["sewing"] += 1
        dst = bytes.fromhex(payload["dst_hex"])
        print("studio_revision", payload["revision"])
        print("studio_dst_sha256", hashlib.sha256(dst).hexdigest())
        print("studio_objects", by_object)
        print("studio_release", payload["readiness"]["machine_release_allowed"])
        assert payload["readiness"]["machine_release_allowed"] is False
        policy = [note for note in payload["readiness"]["notes"] if "12 mm" in note]
        assert policy and "split pieces" in policy[0] and "omitted" in policy[0]
        satin_ids = [
            object_id
            for object_id, counts in by_object.items()
            if counts["sewing"] > 0 and object_id != "1"
        ]
        assert satin_ids, by_object
        assert len(payload["revision"]) == 64
        widths.fill("2")
        widths.dispatch_event("change")
        page.locator("#generate").click()
        page.wait_for_function(
            """() => document.querySelector('#errors').textContent.includes('one column width')
              && document.querySelector('#revision').textContent.includes('not generated')""",
            timeout=30000,
        )
        page.locator("#undo").click()
        page.wait_for_function(
            """() => !document.querySelector('#revision').textContent.includes('Edits')
              && Number(document.querySelector('#stitch-count').textContent) > 4701
              && document.querySelector('#column-widths').value === '2\\n8'""",
            timeout=5000,
        )
        page.locator("#redo").click()
        page.wait_for_function(
            """() => document.querySelector('#column-widths').value === '2'
              && document.querySelector('#revision').textContent.includes('not generated')""",
            timeout=5000,
        )
        page.locator("#undo").click()
        page.wait_for_function(
            "() => document.querySelector('#column-widths').value === '2\\n8'",
            timeout=5000,
        )
        points.fill("20, 30\n50, 30\n20, 30")
        points.dispatch_event("change")
        widths.fill("4\n4\n4")
        widths.dispatch_event("change")
        kept = page.locator("#stitch-count").inner_text()
        page.locator("#generate").click()
        page.wait_for_function(
            """() => document.querySelector('#errors').textContent.includes('reverses')
              && document.querySelector('#revision').textContent.includes('not generated')""",
            timeout=30000,
        )
        assert page.locator("#stitch-count").inner_text() == kept
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        page.screenshot(path=str(EVIDENCE / "desktop-generated.png"))
        for name, width, height, dark in (
            ("tablet", 768, 1024, False),
            ("mobile-dark", 390, 844, True),
        ):
            page.set_viewport_size({"width": width, "height": height})
            page.evaluate(
                "(dark) => { document.documentElement.dataset.appearance = dark ? 'dark' : 'light'; }",
                dark,
            )
            page.locator("#column-widths").scroll_into_view_if_needed()
            overflow = page.evaluate(
                "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
            )
            assert overflow, (name, page.evaluate(
                "() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"
            ))
            page.screenshot(path=str(EVIDENCE / f"{name}.png"))
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
