"""Browser check for preview-first SVG import.

Covers a rejected file, a cancelled preview, a stale apply, and an accepted
import that is generated and rehearsed on the studio TwinLab route.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
BADGE = ROOT / "fixtures" / "svg-geometry" / "badge-hole.svg"
BADGE_SHA = "166a2fd6b55fa61fd4324a9e7b4a30dd5c2dd4fa2f4a236364d8916419211c4c"
DST_SHA = "62e97ba0a9548c8e8ace1504d5f3d41c69929829d175b1e0f214f20f6a859060"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)


def main() -> None:
    url = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
    out = ROOT / "evidence" / "svg-import"
    out.mkdir(parents=True, exist_ok=True)
    rejected = Path(tempfile.mkdtemp(prefix="embroidery-svg-")) / "rejected-text.svg"
    rejected.write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40">'
        '<text x="4" y="20">Hi</text></svg>',
        encoding="utf-8",
    )
    errors: list[str] = []
    console: list[str] = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, device_scale_factor=1)
        watch(page, errors, console)
        page.goto(url, wait_until="domcontentloaded")
        wait_alpine(page)
        assert_no_overflow(page, "1440")
        reject_text(page, rejected)
        cancel_preview(page)
        stale_apply(page)
        accept_and_generate(page)
        page.screenshot(path=out / "desktop-generated.png")
        page.set_viewport_size({"width": 768, "height": 1024})
        assert_no_overflow(page, "768")
        page.screenshot(path=out / "tablet.png")
        page.set_viewport_size({"width": 390, "height": 844})
        page.evaluate("() => { document.documentElement.dataset.appearance = 'dark'; }")
        assert_no_overflow(page, "390")
        page.screenshot(path=out / "mobile-dark.png", full_page=True)
        page.evaluate("() => { document.documentElement.dataset.appearance = 'light'; }")
        page.set_viewport_size({"width": 1440, "height": 1000})
        page.locator("button[data-view='production']").click()
        page.wait_for_function(
            """() => {
              const section = document.querySelector('[data-page="production"]');
              return section && !section.hidden && !document.querySelector('#rehearse').disabled;
            }""",
            timeout=5000,
        )
        page.locator("#rehearse").click()
        page.wait_for_url("**/twinlab", timeout=15000)
        page.wait_for_function(
            """() => {
              const facts = document.querySelector('#facts');
              const qualification = document.querySelector('#qualification');
              if (!facts || !qualification) return false;
              const text = facts.textContent;
              return text.includes('100 × 100 mm')
                && text.includes('1380')
                && text.includes('%s')
                && qualification.textContent.includes('Physical quality not validated')
                && qualification.textContent.includes('No hardware connected');
            }"""
            % DST_SHA,
            timeout=30000,
        )
        page.locator("#facts").scroll_into_view_if_needed()
        page.screenshot(path=out / "twinlab.png")

        mobile = browser.new_page(viewport={"width": 390, "height": 844}, device_scale_factor=1)
        watch(mobile, errors, console)
        mobile.goto(url, wait_until="domcontentloaded")
        mobile.evaluate("() => { document.documentElement.dataset.appearance = 'dark'; }")
        wait_alpine(mobile)
        mobile.locator("#svg-file").set_input_files(str(BADGE))
        mobile.locator("#svg-import").click()
        mobile.wait_for_function(
            """() => !document.querySelector('#svg-preview').hidden
              && document.querySelector('#project-title').textContent === 'Alpine Club'""",
            timeout=20000,
        )
        mobile.locator("#svg-cancel").click()
        mobile.wait_for_function(
            "() => document.querySelector('#svg-status').textContent.includes('Import cancelled')",
            timeout=5000,
        )
        assert_no_overflow(mobile, "390 dark task")
        mobile.locator("#svg-import").focus()
        assert mobile.locator("#svg-import").evaluate("node => node === document.activeElement")
        mobile.screenshot(path=out / "mobile-cancel.png", full_page=True)
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item]
    assert not unexpected, unexpected


def watch(page, errors: list[str], console: list[str]) -> None:
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.on(
        "console",
        lambda message: console.append(message.type + ":" + message.text)
        if message.type == "error"
        else None,
    )


def wait_alpine(page) -> None:
    page.wait_for_function(
        """() => document.querySelector('#project-title').textContent === 'Alpine Club'
          && document.querySelector('#stitch-count').textContent === '4701'
          && document.querySelectorAll('#objects li').length === 7
          && document.querySelector('#svg-apply').disabled""",
        timeout=30000,
    )


def assert_no_overflow(page, label: str) -> None:
    overflow = page.evaluate(
        "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
    )
    assert overflow, (label, page.evaluate(
        "() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"
    ))


def reject_text(page, rejected: Path) -> None:
    page.locator("#svg-file").set_input_files(str(rejected))
    page.locator("#svg-import").click()
    page.wait_for_function(
        """() => {
          const status = document.querySelector('#svg-status').textContent;
          return status.includes('text') && status.includes('not changed');
        }""",
        timeout=20000,
    )
    assert page.locator("#project-title").inner_text() == "Alpine Club"
    assert page.locator("#stitch-count").inner_text() == "4701"
    assert page.locator("#objects li").count() == 7
    assert page.locator("#svg-preview").is_hidden()


def cancel_preview(page) -> None:
    page.locator("#svg-file").set_input_files(str(BADGE))
    page.locator("#svg-import").click()
    page.wait_for_function(
        """() => !document.querySelector('#svg-preview').hidden
          && document.querySelector('#svg-summary').textContent.includes('No stitches were generated')
          && document.querySelector('#svg-summary').textContent.includes('does not approve itself')
          && document.querySelector('#project-title').textContent === 'Alpine Club'
          && document.querySelector('#stitch-count').textContent === '4701'""",
        timeout=20000,
    )
    assert BADGE_SHA in page.locator("#svg-source").inner_text()
    losses = page.locator("#svg-losses").inner_text()
    assert "Machine release" in losses
    assert "1.2 mm" in losses
    assert "generic-flat-120-v1" in losses
    assert "#1e4d3c" in page.locator("#svg-palette").inner_text()
    page.locator("#svg-cancel").click()
    page.wait_for_function(
        """() => document.querySelector('#svg-preview').hidden
          && document.querySelector('#svg-status').textContent.includes('Import cancelled')
          && document.querySelector('#project-title').textContent === 'Alpine Club'
          && document.querySelector('#stitch-count').textContent === '4701'""",
        timeout=5000,
    )


def stale_apply(page) -> None:
    page.locator("#svg-file").set_input_files(str(BADGE))
    page.locator("#svg-import").click()
    page.wait_for_function(
        "() => !document.querySelector('#svg-preview').hidden && !document.querySelector('#svg-apply').disabled",
        timeout=20000,
    )
    spacing = page.locator("#fields label", has_text="Row spacing").locator("input")
    spacing.fill("2.4")
    spacing.dispatch_event("change")
    page.wait_for_function(
        "() => document.querySelector('#revision').textContent.includes('not generated')",
        timeout=5000,
    )
    page.locator("#svg-apply").click()
    page.wait_for_function(
        """() => document.querySelector('#svg-status').textContent
            .includes('The design changed after this preview')
          && document.querySelector('#project-title').textContent === 'Alpine Club'""",
        timeout=5000,
    )
    assert page.locator("#svg-preview").is_hidden()
    page.locator("#undo").click()
    page.wait_for_function(
        """() => document.querySelector('#stitch-count').textContent === '4701'
          && document.querySelector('#project-title').textContent === 'Alpine Club'
          && !document.querySelector('#revision').textContent.includes('Edits')""",
        timeout=5000,
    )


def accept_and_generate(page) -> None:
    page.locator("#svg-import").click()
    page.wait_for_function(
        """() => !document.querySelector('#svg-preview').hidden
          && document.querySelector('#project-title').textContent === 'Alpine Club'
          && document.querySelector('#svg-source').textContent.includes('%s')"""
        % BADGE_SHA,
        timeout=20000,
    )
    page.locator("#svg-apply").click()
    page.wait_for_function(
        """() => document.querySelector('#project-title').textContent === 'badge-hole'
          && document.querySelectorAll('#objects li').length === 1
          && document.querySelector('#preview-note').textContent.includes('not in the stitch plan')""",
        timeout=5000,
    )
    assert "30" in page.locator("#object-holes").input_value()
    page.locator("#generate").click()
    page.wait_for_function(
        """() => document.querySelector('#stitch-count').textContent === '1380'
          && document.querySelector('#revision').textContent.startsWith('665f912eb3ce')
          && document.querySelector('#preview-note').textContent.includes('Machine release remains unavailable')
          && document.querySelector('#svg-status').textContent.includes('Machine release remains unavailable')""",
        timeout=30000,
    )
    assert page.locator("#rehearse").is_enabled()


if __name__ == "__main__":
    main()
