"""Served-browser check for the Alpine Club studio shell.

Requires the studio binary that serves /api/library. The browser does not generate stitches.
"""
from __future__ import annotations

import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
VIEWS = {
    "overview": "Overview",
    "library": "Design library",
    "studio": "Objects",
    "validation": "Validation",
    "approvals": "Approvals",
    "production": "Production",
    "materials": "Materials",
    "insights": "Insights",
    "settings": "Settings",
}


def main() -> None:
    url = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
    out = Path(os.environ.get("STUDIO_HOLE_OUTPUT", ROOT / "evidence" / "m2-g1"))
    out.mkdir(parents=True, exist_ok=True)
    errors = []
    console = []
    with sync_playwright() as playwright:
        options = {"headless": True, "args": ["--no-sandbox", "--disable-dev-shm-usage"]}
        if os.environ.get("BROWSER_EXECUTABLE"):
            options["executable_path"] = os.environ["BROWSER_EXECUTABLE"]
        browser = playwright.chromium.launch(**options)
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, device_scale_factor=1)
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on(
            "console",
            lambda message: console.append(message.type + ":" + message.text) if message.type == "error" else None,
        )
        page.goto(url, wait_until="domcontentloaded")
        page.wait_for_function(
            """() => document.querySelector('#project-title').textContent === 'Alpine Club'
              && document.querySelector('#readiness').textContent === 'Geometry checked'
              && !document.querySelector('#generate').disabled""",
            timeout=30000,
        )
        stitches = int(page.locator("#stitch-count").inner_text())
        travels = int(page.locator("#travel-count").inner_text())
        assert stitches > 0
        assert travels > 0
        assert page.get_by_text("Not performed").count() >= 1
        assert "Geometry checked" == page.locator("#readiness").inner_text()
        assert page.get_by_text("Machine release remains unavailable.").count() >= 1
        assert int(page.locator("#library-count").inner_text()) >= 13
        assert page.locator("a[href='/twinlab']").count() >= 1
        assert page.locator("a[data-compare-open]").count() >= 2
        assert page.locator("a[href='http://127.0.0.1:4181/']").count() >= 2
        assert page.locator("a[href='http://127.0.0.1:4175/']").count() == 0
        page.locator("#zoom-in").click()
        assert page.locator("#zoom-label").inner_text() == "125%"
        page.locator("#zoom-out").click()
        assert page.locator("#zoom-label").inner_text() == "100%"
        for view, heading in VIEWS.items():
            page.locator(f"button[data-view='{view}']").first.click()
            page.wait_for_function(
                "(heading) => [...document.querySelectorAll('h2')].some((node) => node.textContent === heading && node.offsetParent !== null)",
                arg=heading,
                timeout=5000,
            )
        page.locator("button[data-view='studio']").first.click()
        page.locator("#mode-artwork").click()
        page.screenshot(path=out / "alpine-studio.png")
        page.set_viewport_size({"width": 768, "height": 1024})
        page.screenshot(path=out / "alpine-studio-tablet.png", full_page=True)
        page.set_viewport_size({"width": 390, "height": 844})
        studio_overflow = page.evaluate("() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1")
        assert studio_overflow, page.evaluate("() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]")
        page.screenshot(path=out / "alpine-studio-mobile.png", full_page=True)
        page.set_viewport_size({"width": 1440, "height": 1000})
        page.locator("#explore-proposal").click()
        assert page.locator("#proposal").inner_text() != ""
        spacing = page.get_by_label("Row spacing · mm")
        spacing.fill("2.4")
        spacing.dispatch_event("change")
        page.wait_for_function(
            "() => document.querySelector('#revision').textContent === 'Edits are not generated yet'",
            timeout=5000,
        )
        assert page.locator("#export").is_disabled()
        page.locator("#generate").click()
        page.wait_for_function(
            "() => document.querySelector('#readiness').textContent === 'Geometry checked'",
            timeout=30000,
        )
        assert int(page.locator("#stitch-count").inner_text()) > 0
        assert int(page.locator("#stitch-count").inner_text()) != stitches
        page.locator("#mode-travel").click()
        page.locator("#mode-density").click()
        page.locator("#mode-stitches").click()
        page.locator("#mode-artwork").click()
        page.locator("button[data-view='approvals']").first.click()
        page.locator("#reviewer").fill("Local geometry")
        page.locator("#review").click()
        page.wait_for_function(
            "() => document.querySelector('#readiness').textContent === 'Reviewed'",
            timeout=30000,
        )
        page.screenshot(path=out / "alpine-desktop.png", full_page=True)
        page.set_viewport_size({"width": 768, "height": 1024})
        page.screenshot(path=out / "alpine-tablet.png", full_page=True)
        page.set_viewport_size({"width": 390, "height": 844})
        overflow = page.evaluate("() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1")
        assert overflow, page.evaluate("() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]")
        page.locator("#generate").scroll_into_view_if_needed()
        assert page.locator("#generate").is_enabled()
        page.screenshot(path=out / "alpine-mobile.png", full_page=True)
        browser.close()
    assert not errors, errors
    assert not console, console
    print("alpine studio shell passed")
    print("initial_stitches", stitches)
    print("initial_travel", travels)


if __name__ == "__main__":
    main()
