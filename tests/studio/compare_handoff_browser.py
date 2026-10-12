"""Studio Compare opens the proved name-drop item on the permitted local service.

The sidebar and Production links must not navigate to a sample design.
Port 4175 stays unused. The selected studio project stays in place.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "compare-handoff"
BADGE = ROOT / "fixtures" / "name-drop" / "badge.json"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)
STUDIO = os.environ.get("STUDIO_URL", "http://127.0.0.1:4186/")
COMPARE = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4181/")


def studio_identity(page):
    return page.evaluate(
        """() => ({
          title: document.title,
          h1: document.querySelector('#project-title').textContent,
          epoch: window.__studioDesign.snapshot().epoch,
          name: window.__studioDesign.snapshot().project.name,
          object_count: window.__studioDesign.snapshot().project.objects.length,
        })"""
    )


def twin_drop(popup):
    return popup.evaluate(
        """() => {
          const snap = window.__twinlab.snapshot();
          return {
            name_drop: snap.name_drop,
            design_name: snap.design.name,
            schema_version: snap.design.schema_version,
            release_allowed: snap.compare.release_allowed,
            approval_allowed: snap.compare.approval_allowed,
            compare_status: document.querySelector('#compare-status').textContent,
            name_drop_status: document.querySelector('#name-drop-status').textContent,
            hash_text: document.querySelector('#hash').textContent,
            integrity: document.querySelector('#integrity').textContent,
            project_field: document.querySelector('#name').value,
          };
        }"""
    )


def overflow(page):
    return page.evaluate(
        "() => document.documentElement.scrollWidth - document.documentElement.clientWidth"
    )


def main():
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    csv_path = EVIDENCE / "one-item.csv"
    csv_path.write_text("item_id,text\nCMP-001,ALPINE 08\n", encoding="utf-8")
    errors = []
    console = []
    popups = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            executable_path=BROWSER,
            headless=True,
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000})
        page.on("dialog", lambda dialog: dialog.accept())
        def watch(target):
            target.on("pageerror", lambda error: errors.append(str(error)))
            target.on(
                "console",
                lambda message: console.append(message.type + ":" + message.text)
                if message.type == "error" and "ERR_FAILED" not in message.text
                else None,
            )

        page.on("popup", lambda popup: popups.append(popup) or watch(popup))
        watch(page)
        page.goto(STUDIO, wait_until="domcontentloaded")
        page.wait_for_function(
            """() => document.querySelector('#project-title').textContent === 'Alpine Club'
              && document.querySelector('#stitch-count').textContent === '4701'""",
            timeout=30000,
        )
        page.locator("#open-project").set_input_files(str(BADGE))
        page.wait_for_function(
            """() => document.querySelector('#project-title').textContent === 'Name drop badge'
              && document.title === 'Name drop badge — EmbroideryOS'""",
            timeout=15000,
        )
        page.locator("#objects button", has_text="Lettering").click()
        page.wait_for_function(
            """() => document.querySelector('#name-drop h2')
              && document.querySelector('#text-content').value === 'ALPINE 08'""",
            timeout=15000,
        )
        assert page.locator("a[href='http://127.0.0.1:4175/']").count() == 0
        assert page.locator("a[href='http://127.0.0.1:4181/']").count() == 2
        before_proof = len(popups)
        page.locator("nav[aria-label='Existing tools'] a[data-compare-open]").click()
        page.wait_for_function(
            """() => document.querySelector('#compare-status').textContent
              .includes('Prove the selected item before opening Compare')
              && document.querySelector('#errors').textContent
              .includes('No sample was opened')""",
            timeout=5000,
        )
        assert len(popups) == before_proof

        page.locator("#name-drop-csv").set_input_files(str(csv_path))
        page.wait_for_function(
            """() => window.__nameDrop.snapshot().rows.length === 1
              && window.__nameDrop.snapshot().rows[0].item_id === 'CMP-001'
              && window.__nameDrop.snapshot().rows[0].text === 'ALPINE 08'""",
            timeout=5000,
        )
        for key, value in (("min-x", "14"), ("min-y", "22"), ("max-x", "110"), ("max-y", "48")):
            field = page.locator("#name-drop-" + key)
            field.fill(value)
            field.dispatch_event("change")
        page.locator("#name-drop-generate").click()
        page.wait_for_function(
            """() => {
              const row = window.__nameDrop.snapshot().rows[0];
              return row && row.status === 'completed' && row.artifactCurrent
                && window.__nameDrop.snapshot().running === false;
            }""",
            timeout=60000,
        )
        page.locator("#name-drop-write").click()
        page.wait_for_function(
            """() => {
              const row = window.__nameDrop.snapshot().rows[0];
              return row && row.proofCurrent && row.item_id === 'CMP-001';
            }""",
            timeout=60000,
        )
        handoff = page.evaluate("() => window.__nameDrop.currentHandoff()")
        assert handoff["item_id"] == "CMP-001"
        assert handoff["binding"]["machine_release_allowed"] is False
        assert handoff["binding"]["physical_qualification"] is False
        assert len(handoff["pdf_sha256"]) == 64
        assert len(handoff["dst_sha256"]) == 64
        kept = studio_identity(page)

        page.evaluate(
            """() => document.querySelector("meta[name='embroideryos-compare']")
              .setAttribute('content', 'http://127.0.0.1:4175/')"""
        )
        page.locator("nav[aria-label='Existing tools'] a[data-compare-open]").click()
        page.wait_for_function(
            """() => document.querySelector('#compare-status').textContent
              .includes('not the permitted local service')
              && document.querySelector('#compare-status').textContent.includes('Nothing was sent')""",
            timeout=5000,
        )
        assert len(popups) == before_proof
        assert studio_identity(page) == kept
        page.evaluate(
            """() => document.querySelector("meta[name='embroideryos-compare']")
              .setAttribute('content', 'https://127.0.0.1:4181/')"""
        )
        page.locator("nav[aria-label='Existing tools'] a[data-compare-open]").click()
        page.wait_for_function(
            """() => document.querySelector('#compare-status').textContent
              .includes('Nothing was sent')""",
            timeout=5000,
        )
        assert len(popups) == before_proof
        page.evaluate(
            """() => document.querySelector("meta[name='embroideryos-compare']")
              .setAttribute('content', 'http://127.0.0.1:4181/')"""
        )

        def abort(route):
            route.abort()

        page.route("http://127.0.0.1:4181/**", abort)
        page.locator("nav[aria-label='Existing tools'] a[data-compare-open]").click()
        page.wait_for_function(
            """() => {
              const text = document.querySelector('#compare-status').textContent;
              const recovery = document.querySelector('#compare-recovery');
              return text.includes('Compare is unavailable at http://127.0.0.1:4181/')
                && text.includes('This design was kept')
                && recovery && !recovery.hidden;
            }""",
            timeout=10000,
        )
        assert len(popups) == before_proof
        assert studio_identity(page) == kept
        page.unroute("http://127.0.0.1:4181/**")

        with page.expect_popup() as first:
            page.locator("#compare-retry").click()
        popup = first.value
        popup.wait_for_function(
            """() => window.__twinlab && window.__twinlab.snapshot().name_drop
              && window.__twinlab.snapshot().name_drop.proof_current === true""",
            timeout=30000,
        )
        loaded = twin_drop(popup)
        assert loaded["design_name"] == "Alpine mark"
        assert loaded["schema_version"] == 1
        assert loaded["project_field"] == "Alpine mark"
        assert loaded["release_allowed"] is False
        assert loaded["approval_allowed"] is False
        assert loaded["name_drop"]["item_id"] == "CMP-001"
        assert loaded["name_drop"]["dst_sha256"] == handoff["dst_sha256"]
        assert loaded["name_drop"]["pdf_sha256"] == handoff["pdf_sha256"]
        assert loaded["name_drop"]["project_revision"] == handoff["project_revision"]
        assert loaded["name_drop"]["snapshot_id"] == handoff["binding"]["snapshot_id"]
        assert loaded["name_drop"]["design_replaced"] is False
        assert "CMP-001" in loaded["compare_status"]
        assert "was not replaced" in loaded["compare_status"]
        assert "Alpine mark" in loaded["compare_status"]
        assert "Imported review" in loaded["compare_status"]
        assert "Name drop badge" in loaded["compare_status"]
        assert "were not generated by" in loaded["compare_status"]
        assert "were not generated by" in loaded["name_drop_status"]
        assert loaded["hash_text"] == handoff["dst_sha256"]
        assert "Not generated by the open Alpine mark design" in loaded["integrity"]
        assert popup.url.startswith(COMPARE)
        assert studio_identity(page) == kept
        status = page.locator("#compare-status").inner_text()
        assert "CMP-001" in status
        assert handoff["project_revision"] in status
        assert handoff["dst_sha256"][:12] in status
        assert "This studio design was kept" in status
        assert "Machine release remains unavailable" in status
        assert page.evaluate(
            """() => document.querySelector('#compare-recovery').hidden
              && getComputedStyle(document.querySelector('#compare-recovery')).display === 'none'"""
        ) is True
        assert page.url.startswith(STUDIO)

        stale = page.evaluate(
            """() => {
              const handoff = window.__nameDrop.currentHandoff();
              const replay = JSON.parse(JSON.stringify(handoff.replay));
              const binding = JSON.parse(JSON.stringify(handoff.binding));
              binding.snapshot_id = 'stale-snapshot-id';
              return { replay: replay, binding: binding };
            }"""
        )
        popup.evaluate(
            """(payload) => {
              window.postMessage({
                type: 'embroideryos-compare-handoff',
                replay: payload.replay,
                binding: payload.binding,
              }, window.location.origin);
            }""",
            stale,
        )
        popup.wait_for_function(
            """() => document.querySelector('#message').textContent
              .includes('was not applied')
              && document.querySelector('#message').textContent
              .includes('The open design was not replaced')""",
            timeout=5000,
        )
        after_stale = twin_drop(popup)
        assert after_stale["name_drop"] == loaded["name_drop"]
        assert after_stale["design_name"] == "Alpine mark"
        assert studio_identity(page) == kept
        popup.close()

        page.locator("button[data-view='production']").first.click()
        page.wait_for_function(
            """() => [...document.querySelectorAll('h2')].some((node) => node.textContent === 'Production' && !node.closest('[hidden]'))""",
            timeout=5000,
        )
        link = page.locator("[data-page='production'] a[data-compare-open]")
        link.focus()
        with page.expect_popup() as second:
            link.press("Enter")
        again = second.value
        again.wait_for_function(
            """() => window.__twinlab && window.__twinlab.snapshot().name_drop
              && window.__twinlab.snapshot().name_drop.proof_current === true
              && window.__twinlab.snapshot().name_drop.item_id === 'CMP-001'""",
            timeout=30000,
        )
        second_drop = twin_drop(again)
        assert second_drop["name_drop"]["dst_sha256"] == handoff["dst_sha256"]
        assert second_drop["name_drop"]["pdf_sha256"] == handoff["pdf_sha256"]
        assert second_drop["name_drop"]["project_revision"] == handoff["project_revision"]
        assert second_drop["design_name"] == "Alpine mark"
        assert second_drop["release_allowed"] is False
        assert studio_identity(page) == kept
        assert page.evaluate(
            """() => getComputedStyle(document.querySelector('#compare-recovery')).display === 'none'"""
        ) is True
        page.screenshot(path=str(EVIDENCE / "production-desktop.png"))
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 768, "height": 1024})
        page.locator("[data-page='production'] a[data-compare-open]").scroll_into_view_if_needed()
        page.screenshot(path=str(EVIDENCE / "production-tablet.png"))
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 390, "height": 844})
        mobile_link = page.locator("[data-page='production'] a[data-compare-open]")
        mobile_link.scroll_into_view_if_needed()
        page.screenshot(path=str(EVIDENCE / "production-mobile.png"))
        assert overflow(page) <= 1
        assert mobile_link.is_visible()
        box = mobile_link.bounding_box()
        assert box and box["y"] >= 0 and box["y"] < 844 and box["width"] >= 40
        again.close()
        browser.close()

    assert not errors, errors
    assert not console, console
    record = {
        "item_id": handoff["item_id"],
        "snapshot_id": handoff["binding"]["snapshot_id"],
        "project_revision": handoff["project_revision"],
        "dst_sha256": handoff["dst_sha256"],
        "pdf_sha256": handoff["pdf_sha256"],
        "replay_sha256": handoff["binding"]["replay_sha256"],
        "machine_release_allowed": False,
        "physical_qualification": False,
        "studio_project": kept["name"],
        "compare_design_kept": "Alpine mark",
        "bad_destination_rejected": True,
        "unavailable_rejected": True,
        "stale_handoff_rejected": True,
        "sidebar_and_production_opened": True,
    }
    (EVIDENCE / "result.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    main()
