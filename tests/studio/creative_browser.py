"""Creative Director saves a raster artwork master without touching the embroidery project.

The local test provider returns real PNG bytes. It is not a live xAI illustration.
Port 4175 stays unused.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "creative-c1a"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)
STUDIO = os.environ.get("STUDIO_URL", "http://127.0.0.1:4192/")
ROBOT = (
    "Create an original friendly exploration robot, illustrated as a colourful cartoon character. "
    "Use a clean background and keep the silhouette readable."
)


def identity(page):
    return page.evaluate(
        """() => {
          const snap = window.__studioDesign.snapshot();
          return {
            title: document.querySelector('#project-title').textContent,
            epoch: snap.epoch,
            name: snap.project.name,
            objects: snap.project.objects.length,
            project: JSON.stringify(snap.project),
          };
        }"""
    )


def main() -> None:
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    reference = EVIDENCE / "reference.png"
    if not reference.exists():
        from PIL import Image

        Image.new("RGB", (8, 8), (24, 48, 72)).save(reference)
    errors = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            executable_path=BROWSER,
            headless=True,
        )
        page = browser.new_page(viewport={"width": 390, "height": 844})
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on(
            "console",
            lambda message: errors.append(message.text)
            if message.type == "error"
            else None,
        )
        page.goto(STUDIO, wait_until="domcontentloaded")
        page.wait_for_function(
            "() => window.__studioDesign && window.__studioDesign.snapshot().project.name",
            timeout=15000,
        )
        before = identity(page)
        page.locator("#open-creative").scroll_into_view_if_needed()
        page.locator("#open-creative").click()
        page.wait_for_function(
            """() => document.title === 'Creative Director — EmbroideryOS'
              && document.querySelectorAll('h1').length === 1
              && document.querySelector('#creative-heading').textContent === 'Creative Director'""",
            timeout=5000,
        )
        page.locator("#creative-request").fill(ROBOT)
        page.locator("#creative-profile").select_option("cartoon")
        page.locator("#creative-width").fill("64")
        page.locator("#creative-height").fill("64")
        page.locator("#creative-generate").click()
        page.wait_for_function(
            """() => {
              const image = document.querySelector('#creative-image');
              const caption = document.querySelector('#creative-caption').textContent;
              return image.naturalWidth > 0
                && caption.includes('Local test raster')
                && caption.includes('Not a live provider image')
                && caption.includes('No alpha channel')
                && image.alt.startsWith('Raster artwork candidate:');
            }""",
            timeout=15000,
        )
        assert identity(page)["project"] == before["project"]
        page.locator("#creative-select").click()
        page.wait_for_function(
            """() => document.querySelector('#creative-status').textContent.includes('embroidery design was not changed')
              && !document.querySelector('#creative-download').hidden""",
            timeout=10000,
        )
        stored = page.evaluate(
            """async () => {
              const link = document.querySelector('#creative-download');
              const response = await fetch(link.href, { cache: 'no-store' });
              const bytes = new Uint8Array(await response.arrayBuffer());
              const digest = await crypto.subtle.digest('SHA-256', bytes);
              const sha = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
              return {
                href: link.href,
                type: response.headers.get('content-type'),
                sha,
                length: bytes.length,
                png: bytes[0] === 0x89 && bytes[1] === 0x50,
              };
            }"""
        )
        assert stored["png"] is True
        assert stored["type"].startswith("image/png")
        assert stored["sha"] in page.locator("#creative-caption").inner_text()
        page.locator("#creative-reopen").click()
        page.wait_for_function(
            """(sha) => document.querySelector('#creative-status').textContent.includes('SHA-256 matches ' + sha)""",
            arg=stored["sha"],
            timeout=10000,
        )
        assert identity(page)["project"] == before["project"]
        page.locator("#creative-image").scroll_into_view_if_needed()
        mobile_box = page.locator("#creative-select").bounding_box()
        assert mobile_box and mobile_box["width"] >= 40
        overflow = page.evaluate(
            """() => {
              const panel = document.querySelector('#creative-panel');
              const card = panel.querySelector('.page-card');
              return panel.scrollWidth <= panel.clientWidth + 1
                && card.scrollWidth <= card.clientWidth + 1
                && document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1;
            }"""
        )
        assert overflow is True
        page.screenshot(path=EVIDENCE / "creative-mobile.png")
        page.set_viewport_size({"width": 768, "height": 1024})
        page.locator("#creative-image").scroll_into_view_if_needed()
        page.screenshot(path=EVIDENCE / "creative-tablet.png")
        page.set_viewport_size({"width": 1440, "height": 1000})
        page.locator("#creative-image").scroll_into_view_if_needed()
        page.screenshot(path=EVIDENCE / "creative-desktop.png")

        page.locator("#creative-profile").select_option("portrait")
        page.locator("#creative-request").fill("Portrait treatment of a supplied person")
        page.locator("#creative-reference").set_input_files([])
        page.locator("#creative-generate").click()
        page.wait_for_function(
            """() => document.querySelector('#creative-error').textContent.includes('No likeness was invented')""",
            timeout=10000,
        )
        page.locator("#creative-reference").set_input_files(str(reference))
        page.locator("#creative-generate").click()
        page.wait_for_function(
            """() => {
              const text = document.querySelector('#creative-error').textContent;
              return text.includes('Reference editing was not called')
                && text.includes('No likeness was invented');
            }""",
            timeout=10000,
        )
        assert identity(page)["project"] == before["project"]
        reopened = page.evaluate(
            """async (href) => {
              const response = await fetch(href, { cache: 'no-store' });
              const bytes = new Uint8Array(await response.arrayBuffer());
              const digest = await crypto.subtle.digest('SHA-256', bytes);
              return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
            }""",
            stored["href"],
        )
        assert reopened == stored["sha"]
        policy = page.evaluate(
            """async () => {
              const response = await fetch('/api/creative/policy', { cache: 'no-store' });
              return response.json();
            }"""
        )
        page.locator("#creative-close").click()
        page.wait_for_function(
            "() => document.querySelector('#creative-panel').hidden",
            timeout=5000,
        )
        assert identity(page)["project"] == before["project"]
        browser.close()
    refusals = [item for item in errors if "status of 422" in item]
    unexpected = [item for item in errors if "status of 422" not in item]
    if unexpected or len(refusals) != 2:
        raise SystemExit({"unexpected": unexpected, "refusals": refusals})
    record = {
        "studio": STUDIO,
        "project_name": before["name"],
        "project_unchanged": True,
        "provider": "test_image",
        "live": False,
        "sha256": stored["sha"],
        "byte_length": stored["length"],
        "artifact_href": stored["href"],
        "portrait_without_reference": "blocked",
        "portrait_with_reference": "blocked_generation_does_not_edit",
        "expected_portrait_422_console_lines": len(refusals),
        "policy": {
            "provider": policy.get("provider"),
            "live_ready": policy.get("live_ready"),
            "key_configured": policy.get("key_configured"),
            "remaining_calls": policy.get("remaining_calls"),
            "transmit_permitted": policy.get("transmit_permitted"),
            "live_process_enabled": policy.get("live_process_enabled"),
        },
        "machine_release_allowed": False,
    }
    (EVIDENCE / "browser.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({"sha256": stored["sha"], "bytes": stored["length"], "project": before["name"]}))


if __name__ == "__main__":
    main()
