"""Browser check for the design prompt.

An empty prompt and a missing studio key must leave Alpine Club unchanged.
This script does not call the model.
"""

from __future__ import annotations

import os
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    url = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
    out = Path(os.environ.get("STUDIO_HOLE_OUTPUT", ROOT / "evidence" / "m2-g1"))
    out.mkdir(parents=True, exist_ok=True)
    image = out / "prompt-sample.png"
    Image.new("RGB", (32, 32), (30, 77, 60)).save(image)
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
              && !document.querySelector('#ask-design').disabled""",
            timeout=30000,
        )
        before = page.locator("#stitch-count").inner_text()
        page.locator("#ask-design").click()
        page.wait_for_function(
            "() => document.querySelector('#prompt-status').textContent.includes('not changed')",
            timeout=5000,
        )
        assert page.locator("#project-title").inner_text() == "Alpine Club"
        page.locator("#design-prompt").fill("A small filled square named Trial square")
        page.locator("#design-prompt").evaluate(
            """(node) => {
              const event = new Event('paste', { bubbles: true });
              node.dispatchEvent(event);
              node.value = 'A pasted outline of a ridge';
            }"""
        )
        assert "pasted" in page.locator("#design-prompt").input_value()
        page.locator("#design-image").set_input_files(str(image))
        page.wait_for_function(
            "() => !document.querySelector('#prompt-preview').hidden && document.querySelector('#prompt-thumb').src.startsWith('data:image/jpeg')",
            timeout=5000,
        )
        page.locator("#voice-input").click()
        page.wait_for_function(
            """() => {
              const text = document.querySelector('#prompt-status').textContent;
              return text.includes('Audio is not sent') || text.includes('not available') || text.includes('Listening');
            }""",
            timeout=5000,
        )
        if page.locator("#voice-input").inner_text() == "Stop voice":
            page.locator("#voice-input").click()
        page.locator("#ask-design").click()
        page.wait_for_function(
            "() => document.querySelector('#prompt-status').textContent.includes('XAI_API_KEY')",
            timeout=10000,
        )
        assert page.locator("#project-title").inner_text() == "Alpine Club"
        assert page.locator("#stitch-count").inner_text() == before
        assert page.locator("#design-proposal").is_hidden()
        assert "not changed" in page.locator("#prompt-status").inner_text()
        page.screenshot(path=out / "prompt-desktop.png")
        page.evaluate("() => document.querySelector('#discard-design').click()")
        assert page.locator("#project-title").inner_text() == "Alpine Club"
        assert "not changed" in page.locator("#prompt-status").inner_text()
        page.evaluate("() => { document.documentElement.dataset.appearance = 'dark'; }")
        page.set_viewport_size({"width": 390, "height": 844})
        overflow = page.evaluate(
            "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
        )
        assert overflow, page.evaluate(
            "() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"
        )
        page.screenshot(path=out / "prompt-mobile-dark.png", full_page=True)
        page.evaluate("() => { document.documentElement.dataset.appearance = 'light'; }")
        page.screenshot(path=out / "prompt-mobile.png", full_page=True)
        page.locator("#design-prompt").focus()
        assert page.locator("#design-prompt").evaluate("node => node === document.activeElement")
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
