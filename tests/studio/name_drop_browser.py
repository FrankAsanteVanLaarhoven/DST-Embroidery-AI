"""Browser check for a bounded name drop.

One badge, one text object, and Block 01 version 1 produce four traceable
rows. ITEM-001 and ITEM-004 both keep ALPINE 08. A failed accent stays
visible. Cancellation and a template edit do not publish a complete batch.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "name-drop"
BADGE = ROOT / "fixtures" / "name-drop" / "badge.json"
CSV = ROOT / "fixtures" / "name-drop" / "four-names.csv"
FONT = "aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)
STUDIO = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
TWIN = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4181/")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def snap(page):
    return page.evaluate("() => window.__nameDrop.snapshot()")


def row(page, item_id):
    found = [item for item in snap(page)["rows"] if item["item_id"] == item_id]
    assert len(found) == 1, snap(page)
    return found[0]


def attach(page, errors, console):
    page.on("dialog", lambda dialog: dialog.accept())
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.on(
        "console",
        lambda message: console.append(message.type + ":" + message.text)
        if message.type == "error"
        else None,
    )


def open_badge(page):
    page.goto(STUDIO, wait_until="domcontentloaded")
    page.wait_for_function(
        """() => document.querySelector('#project-title').textContent === 'Alpine Club'
          && document.querySelector('#stitch-count').textContent === '4701'""",
        timeout=30000,
    )
    page.locator("#open-project").set_input_files(str(BADGE))
    page.wait_for_function(
        """() => document.querySelector('#project-title').textContent === 'Name drop badge'
          && document.title === 'Name drop badge — EmbroideryOS'
          && document.querySelectorAll('h1').length === 1""",
        timeout=15000,
    )
    page.locator("#objects button", has_text="Lettering").click()
    page.wait_for_function(
        """() => {
          const heading = document.querySelector('#name-drop h2');
          const text = document.querySelector('#text-content');
          return heading && heading.textContent === 'Name drop' && text && text.value === 'ALPINE 08'
            && document.querySelector('#name-drop-font').textContent.includes('embroideryos-block-01');
        }""",
        timeout=15000,
    )


def arm_table(page):
    page.locator("#name-drop-example").click()
    page.wait_for_function(
        """() => {
          const rows = window.__nameDrop.snapshot().rows;
          return rows.length === 4 && rows[0].text === 'ALPINE 08' && rows[3].text === 'ALPINE 08'
            && rows[0].item_id === 'ITEM-001' && rows[3].item_id === 'ITEM-004';
        }""",
        timeout=5000,
    )
    page.locator("#name-drop-csv").set_input_files(str(CSV))
    page.wait_for_function(
        """() => document.querySelector('#name-drop-message').textContent.includes('CSV imported')
          && window.__nameDrop.snapshot().rows.length === 4""",
        timeout=5000,
    )
    for key, value in (("min-x", "14"), ("min-y", "22"), ("max-x", "110"), ("max-y", "48")):
        field = page.locator("#name-drop-" + key)
        field.fill(value)
        field.dispatch_event("change")
    page.wait_for_function(
        """() => document.querySelector('#name-drop-min-x').value === '14'
          && document.querySelector('#name-drop-min-y').value === '22'
          && document.querySelector('#name-drop-max-x').value === '110'
          && document.querySelector('#name-drop-max-y').value === '48'""",
        timeout=5000,
    )


def generate_names(page):
    page.locator("#name-drop-generate").focus()
    page.locator("#name-drop-generate").press("Enter")
    page.wait_for_function(
        """() => {
          const snap = window.__nameDrop.snapshot();
          return snap.running === false && snap.rows.length === 4
            && snap.rows.every((row) => row.status === 'completed' && row.artifactCurrent)
            && document.querySelector('#name-drop-generate').getAttribute('aria-busy') === 'false';
        }""",
        timeout=60000,
    )


def select_item(page, item_id):
    page.locator(f'[aria-label="Show {item_id}"]').click()
    page.wait_for_function(
        """(itemId) => {
          const snap = window.__nameDrop.snapshot();
          const row = snap.rows.find((entry) => entry.item_id === itemId);
          const field = document.querySelector('#text-content');
          return snap.selected === itemId && row && row.artifactCurrent && field
            && field.value === row.text && !document.querySelector('#export').disabled;
        }""",
        arg=item_id,
        timeout=60000,
    )


def write_proof(page, item_id):
    select_item(page, item_id)
    before = row(page, item_id)["dst_sha256"]
    page.locator("#name-drop-write").click()
    page.wait_for_function(
        """(itemId) => {
          const snap = window.__nameDrop.snapshot();
          const row = snap.rows.find((entry) => entry.item_id === itemId);
          const proof = document.querySelector('#name-drop-proof').textContent;
          return row && row.proofCurrent && row.proof_id
            && proof.includes(itemId) && proof.includes(row.proof_id)
            && !document.querySelector('#name-drop-replay').disabled;
        }""",
        arg=item_id,
        timeout=60000,
    )
    proved = row(page, item_id)
    assert proved["dst_sha256"] == before
    assert proved["font_sha256"] == FONT
    return proved


def save_download(page, selector, destination: Path) -> Path:
    with page.expect_download() as download_info:
        page.locator(selector).click()
    download = download_info.value
    destination.parent.mkdir(parents=True, exist_ok=True)
    download.save_as(destination)
    return destination


def overflow(page) -> int:
    return page.evaluate(
        "() => document.documentElement.scrollWidth - document.documentElement.clientWidth"
    )


def hold_second(page):
    gate = {"route": None}

    def handle(route):
        payload = json.loads(route.request.post_data or "{}")
        if payload.get("item_id") == "ITEM-002" and gate["route"] is None:
            gate["route"] = route
            return
        route.continue_()

    page.route("**/api/batch/item", handle)
    return gate


def release(gate):
    route = gate.get("route")
    if not route:
        return
    gate["route"] = None
    route.continue_()


def main() -> None:
    errors: list[str] = []
    console: list[str] = []
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(page, errors, console)
        open_badge(page)
        assert "version 1" in page.locator("#name-drop-font").inner_text()
        assert FONT in page.locator("#name-drop-font").inner_text()
        assert "native-hoop-v1" in page.locator("#name-drop-font").inner_text()
        arm_table(page)
        generate_names(page)
        state = snap(page)
        assert state["certifies"] is True
        assert state["stale"] is False
        assert [item["item_id"] for item in state["rows"]] == [
            "ITEM-001",
            "ITEM-002",
            "ITEM-003",
            "ITEM-004",
        ]
        assert [item["text"] for item in state["rows"]] == [
            "ALPINE 08",
            "ALPINE 09",
            "O'NEIL-08",
            "ALPINE 08",
        ]
        assert state["counts"] == {
            "queued": 0,
            "running": 0,
            "completed": 4,
            "failed": 0,
            "cancelled": 0,
        }
        assert all(item["font_sha256"] == FONT for item in state["rows"])
        assert state["rows"][0]["dst_sha256"] == state["rows"][3]["dst_sha256"]
        assert state["rows"][0]["project_revision"] == state["rows"][3]["project_revision"]
        assert state["rows"][0]["dst_sha256"] != state["rows"][1]["dst_sha256"]
        assert "U+0027" in state["rows"][2]["glyph_codes"]
        assert "U+002D" in state["rows"][2]["glyph_codes"]
        assert page.locator("#name-drop-bundle").is_disabled()

        first = write_proof(page, "ITEM-001")
        select_item(page, "ITEM-002")
        assert row(page, "ITEM-001")["proofCurrent"] is True
        assert row(page, "ITEM-002")["proofCurrent"] is False
        assert "not current" in page.locator("#name-drop-proof").inner_text()
        assert "ITEM-001" not in page.locator("#name-drop-proof").inner_text()
        second = write_proof(page, "ITEM-002")
        assert page.locator("#name-drop-bundle").is_disabled()
        assert first["proof_id"] != second["proof_id"]
        write_proof(page, "ITEM-003")
        write_proof(page, "ITEM-004")
        assert page.locator("#name-drop-bundle").is_enabled()
        assert row(page, "ITEM-001")["proof_id"] != row(page, "ITEM-004")["proof_id"]
        assert row(page, "ITEM-001")["dst_sha256"] == row(page, "ITEM-004")["dst_sha256"]

        select_item(page, "ITEM-001")
        assert "ITEM-001" in page.locator("#name-drop-message").inner_text()
        assert "ITEM-004" not in page.locator("#name-drop-message").inner_text()
        project_path = save_download(page, "#name-drop-project", EVIDENCE / "ITEM-001.embroidery-project.json")
        dst_path = save_download(page, "#name-drop-dst", EVIDENCE / "ITEM-001.dst")
        replay_path = save_download(page, "#name-drop-replay", EVIDENCE / "ITEM-001.replay.json")
        pdf_path = save_download(page, "#name-drop-pdf", EVIDENCE / "ITEM-001.proof.pdf")
        bundle_path = save_download(page, "#name-drop-bundle", EVIDENCE / "name-drop-batch.json")

        saved = json.loads(project_path.read_text())
        text = next(item for item in saved["objects"] if item["kind"] == "text")
        assert saved["format_version"] == "2"
        assert text["points"] == []
        assert text["text"]["content"] == "ALPINE 08"
        assert text["text"]["font_id"] == "embroideryos-block-01"
        assert text["text"]["font_version"] == "1"
        assert text["text"]["font_sha256"] == FONT
        assert dst_path.read_bytes()[:2] == b"LA"
        assert sha256(dst_path) == row(page, "ITEM-001")["dst_sha256"]
        replay = json.loads(replay_path.read_text())
        assert replay["protocol"] == "embroidery-project-replay/1"
        assert replay["kernel"]["mode"] == "native-rust"
        assert replay["batch"]["item_id"] == "ITEM-001"
        assert replay["batch"]["text"] == "ALPINE 08"
        assert replay["provenance"]["dst_sha256"] == sha256(dst_path)
        assert pdf_path.read_bytes().startswith(b"%PDF")
        bundle = json.loads(bundle_path.read_text())
        assert bundle["schema"] == "embroideryos-name-drop-batch/0.1"
        assert bundle["certifies_batch"] is True
        assert bundle["machine_release_allowed"] is False
        assert bundle["physical_qualification"] is False
        assert len(bundle["items"]) == 4
        assert bundle["items"][0]["dst_sha256"] == bundle["items"][3]["dst_sha256"]
        assert bundle["items"][0]["proof_id"] != bundle["items"][3]["proof_id"]
        assert bundle["items"][0]["text"] == "ALPINE 08"
        assert bundle["items"][3]["text"] == "ALPINE 08"
        assert all(item["binding"]["machine_release_allowed"] is False for item in bundle["items"])
        assert all(item["font_sha256"] == FONT for item in bundle["items"])

        page.screenshot(path=str(EVIDENCE / "studio-desktop.png"), full_page=True)
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 768, "height": 1024})
        page.locator("#name-drop").scroll_into_view_if_needed()
        page.screenshot(path=str(EVIDENCE / "studio-tablet.png"), full_page=True)
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 390, "height": 844})
        select_item(page, "ITEM-002")
        assert page.locator("#text-content").input_value() == "ALPINE 09"
        assert "ITEM-002" in page.locator("#name-drop-proof").inner_text()
        assert "ITEM-001" not in page.locator("#name-drop-proof").inner_text()
        page.locator("#name-drop").scroll_into_view_if_needed()
        page.screenshot(path=str(EVIDENCE / "studio-mobile.png"), full_page=True)
        assert overflow(page) <= 1

        page.set_viewport_size({"width": 1440, "height": 1000})
        select_item(page, "ITEM-001")
        with page.expect_download() as before_info:
            page.locator("#export").click()
        before = before_info.value
        before_path = EVIDENCE / "studio-before-undo.dst"
        before.save_as(before_path)
        before_hash = sha256(before_path)
        field = page.locator("#text-content")
        field.fill("ALPINE 09")
        field.dispatch_event("change")
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === 'ALPINE 09'
              && document.querySelector('#export').disabled
              && window.__nameDrop.snapshot().rows.find((entry) => entry.item_id === 'ITEM-001').text === 'ALPINE 09'
              && window.__nameDrop.snapshot().rows.find((entry) => entry.item_id === 'ITEM-001').proofCurrent === false
              && window.__nameDrop.snapshot().certifies === false""",
            timeout=5000,
        )
        assert page.locator("#name-drop-bundle").is_disabled()
        page.locator("#undo").click()
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === 'ALPINE 08'
              && !document.querySelector('#export').disabled
              && window.__nameDrop.snapshot().rows.find((entry) => entry.item_id === 'ITEM-001').proofCurrent === false
              && window.__nameDrop.snapshot().certifies === false""",
            timeout=5000,
        )
        with page.expect_download() as after_info:
            page.locator("#export").click()
        after = after_info.value
        after_path = EVIDENCE / "studio-after-undo.dst"
        after.save_as(after_path)
        assert sha256(after_path) == before_hash
        assert "not current" in page.locator("#name-drop-proof").inner_text()
        assert page.locator("#name-drop-bundle").is_disabled()

        cafe = page.locator('[aria-label="Text for ITEM-003"]')
        cafe.fill("CAFÉ")
        cafe.dispatch_event("change")
        page.locator("#name-drop-generate").click()
        page.wait_for_function(
            """() => {
              const snap = window.__nameDrop.snapshot();
              const accent = snap.rows.find((entry) => entry.item_id === 'ITEM-003');
              return snap.running === false && accent && accent.text === 'CAFÉ' && accent.status === 'failed'
                && accent.artifactCurrent === false && snap.certifies === false
                && snap.counts.failed === 1 && snap.counts.completed === 3;
            }""",
            timeout=60000,
        )
        assert "U+00C9" in row(page, "ITEM-003")["error"]
        assert "not changed" in row(page, "ITEM-003")["error"]
        page.locator('[aria-label="Text for ITEM-003"]').scroll_into_view_if_needed()
        assert page.locator('[aria-label="Text for ITEM-003"]').input_value() == "CAFÉ"
        assert page.locator("#name-drop-bundle").is_disabled()
        page.locator("#name-drop-template").click()
        page.wait_for_function(
            """() => {
              const view = window.__studioDesign.snapshot();
              const text = view.project.objects.find((object) => object.kind === 'text');
              return view.inspection === null && text.text.content === 'ALPINE 08'
                && text.text.font_sha256 === '%s';
            }"""
            % FONT,
            timeout=15000,
        )

        page.locator("#open-project").set_input_files(str(project_path))
        page.wait_for_function(
            """() => {
              const view = window.__studioDesign.snapshot();
              const text = view.project.objects.find((object) => object.kind === 'text');
              return view.project.format_version === '2' && text.points.length === 0
                && text.text.content === 'ALPINE 08'
                && text.text.font_id === 'embroideryos-block-01'
                && text.text.font_version === '1'
                && text.text.font_sha256 === '%s'
                && window.__nameDrop.snapshot().stale === true
                && window.__nameDrop.snapshot().certifies === false;
            }"""
            % FONT,
            timeout=15000,
        )
        page.locator("#objects button", has_text="Lettering").click()
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === 'ALPINE 08'
              && document.querySelector('#name-drop-bundle').disabled""",
            timeout=5000,
        )

        stale = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(stale, errors, console)
        open_badge(stale)
        arm_table(stale)
        held = hold_second(stale)
        try:
            stale.locator("#name-drop-generate").click()
            stale.wait_for_function(
                """() => {
                  const snap = window.__nameDrop.snapshot();
                  const first = snap.rows.find((entry) => entry.item_id === 'ITEM-001');
                  const second = snap.rows.find((entry) => entry.item_id === 'ITEM-002');
                  return snap.running && first && first.artifactCurrent && second && second.status === 'running';
                }""",
                timeout=60000,
            )
            field = stale.locator("#text-content")
            field.fill("ALPINE 07")
            field.dispatch_event("change")
            stale.wait_for_function(
                """() => {
                  const snap = window.__nameDrop.snapshot();
                  return snap.stale && snap.running === false && snap.certifies === false
                    && document.querySelector('#name-drop-message').textContent.includes('Late results were not applied');
                }""",
                timeout=5000,
            )
            release(held)
            stale.wait_for_timeout(500)
            late = snap(stale)
            assert late["stale"] is True
            assert late["certifies"] is False
            assert all(item["artifactCurrent"] is False for item in late["rows"])
            assert all(item["proofCurrent"] is False for item in late["rows"])
            assert row(stale, "ITEM-002")["text"] == "ALPINE 09"
            assert row(stale, "ITEM-001")["text"] == "ALPINE 08"
            assert stale.locator("#name-drop-bundle").is_disabled()
        finally:
            release(held)

        cancelled = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(cancelled, errors, console)
        open_badge(cancelled)
        arm_table(cancelled)
        held = hold_second(cancelled)
        try:
            cancelled.locator("#name-drop-generate").click()
            cancelled.wait_for_function(
                """() => {
                  const snap = window.__nameDrop.snapshot();
                  const first = snap.rows.find((entry) => entry.item_id === 'ITEM-001');
                  return snap.running && first && first.status === 'completed' && first.artifactCurrent
                    && !document.querySelector('#name-drop-cancel').disabled;
                }""",
                timeout=60000,
            )
            cancelled.locator("#name-drop-cancel").click()
            cancelled.wait_for_function(
                """() => {
                  const snap = window.__nameDrop.snapshot();
                  const first = snap.rows.find((entry) => entry.item_id === 'ITEM-001');
                  return snap.running === false && snap.certifies === false && first.artifactCurrent
                    && first.status === 'completed'
                    && document.querySelector('#name-drop-message').textContent.includes('No completion manifest');
                }""",
                timeout=15000,
            )
            release(held)
            cancelled.wait_for_timeout(800)
            done = snap(cancelled)
            assert done["certifies"] is False
            assert done["stale"] is False
            assert row(cancelled, "ITEM-001")["status"] == "completed"
            assert row(cancelled, "ITEM-001")["artifactCurrent"] is True
            assert row(cancelled, "ITEM-001")["text"] == "ALPINE 08"
            for item_id in ("ITEM-002", "ITEM-003", "ITEM-004"):
                assert row(cancelled, item_id)["status"] == "cancelled", row(cancelled, item_id)
                assert row(cancelled, item_id)["artifactCurrent"] is False
            assert done["counts"]["completed"] == 1
            assert done["counts"]["cancelled"] == 3
            assert done["counts"]["failed"] == 0
            assert cancelled.locator("#name-drop-bundle").is_disabled()
            cancelled.locator('[aria-label="Show ITEM-001"]').click()
            cancelled.wait_for_function(
                """() => document.querySelector('#text-content').value === 'ALPINE 08'
                  && !document.querySelector('#name-drop-dst').disabled""",
                timeout=60000,
            )
        finally:
            release(held)

        twin = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(twin, errors, console)
        twin.goto(TWIN, wait_until="domcontentloaded")
        twin.wait_for_function(
            """() => document.querySelector('h1') && document.querySelector('h1').textContent.includes('See every stitch')
              && window.__twinlab && window.__twinlab.snapshot().design
              && window.__twinlab.snapshot().design.name === 'Alpine mark'""",
            timeout=30000,
        )
        twin.locator("#name-drop-input").set_input_files(str(replay_path))
        twin.wait_for_function(
            """() => {
              const snap = window.__twinlab.snapshot();
              return snap.name_drop && snap.name_drop.item_id === 'ITEM-001'
                && snap.name_drop.design_replaced === false
                && snap.design.name === 'Alpine mark'
                && snap.name_drop.proof_current === false
                && document.querySelector('#name-drop-status').textContent.includes('was not replaced');
            }""",
            timeout=10000,
        )
        mismatched = {
            "schema": "embroideryos-name-drop-binding/0.1",
            "snapshot_id": replay["batch"]["snapshot_id"],
            "item_id": "ITEM-999",
            "project_revision": replay["batch"]["project_revision"],
            "dst_sha256": replay["provenance"]["dst_sha256"],
            "machine_release_allowed": False,
            "physical_qualification": False,
        }
        mismatch_path = EVIDENCE / "mismatch-binding.json"
        mismatch_path.write_text(json.dumps(mismatched))
        twin.locator("#name-drop-proof-input").set_input_files(str(mismatch_path))
        twin.wait_for_function(
            """() => window.__twinlab.snapshot().name_drop.proof_current === false
              && document.querySelector('#name-drop-status').textContent.includes('not current')
              && window.__twinlab.snapshot().design.name === 'Alpine mark'""",
            timeout=5000,
        )
        matched = dict(mismatched)
        matched["item_id"] = "ITEM-001"
        match_path = EVIDENCE / "match-binding.json"
        match_path.write_text(json.dumps(matched))
        twin.locator("#name-drop-proof-input").set_input_files(str(match_path))
        twin.wait_for_function(
            """() => window.__twinlab.snapshot().name_drop.proof_current === true
              && window.__twinlab.snapshot().name_drop.design_replaced === false
              && window.__twinlab.snapshot().design.name === 'Alpine mark'
              && document.querySelector('#name-drop-status').textContent.includes('Machine release remains unavailable')""",
            timeout=5000,
        )
        twin.locator("section.compare").screenshot(path=str(EVIDENCE / "twin-desktop.png"))
        assert overflow(twin) <= 1
        twin.set_viewport_size({"width": 390, "height": 844})
        twin.locator("#clear-name-drop").scroll_into_view_if_needed()
        twin.locator("#clear-name-drop").click()
        twin.wait_for_function(
            """() => window.__twinlab.snapshot().name_drop === null
              && document.querySelector('#name-drop-status').textContent.includes('No name-drop bundle')
              && window.__twinlab.snapshot().design.name === 'Alpine mark'""",
            timeout=5000,
        )
        twin.screenshot(path=str(EVIDENCE / "twin-mobile.png"), full_page=True)
        assert overflow(twin) <= 1
        browser.close()

    unexplained = [line for line in console if "422" not in line]
    assert errors == [], errors
    assert unexplained == [], unexplained
    print("name-drop browser passed")
    print("dst", sha256(dst_path))
    print("proofs", row_ids(bundle))


def row_ids(bundle):
    return [(item["item_id"], item["proof_id"][:12], item["dst_sha256"][:12]) for item in bundle["items"]]


if __name__ == "__main__":
    main()
