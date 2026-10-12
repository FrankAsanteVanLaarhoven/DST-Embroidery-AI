"""Browser check for Block 01 version 2 and the preserved name-drop binding.

Version 1 stays the badge pin until the font control selects version 2.
Cafe with U+00E9 fails on version 1. The same entered table then sews on
version 2. Duplicate names, canonical equivalents, and the two apostrophes
stay separate rows. Evidence is written under evidence/lettering-v2.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "lettering-v2"
BADGE = ROOT / "fixtures" / "name-drop" / "badge.json"
CSV = ROOT / "fixtures" / "name-drop" / "lettering-v2-names.csv"
FOUR = ROOT / "fixtures" / "name-drop" / "four-names.csv"
FONT_V1 = "aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd"
FONT_V2 = "78e8506438960cc575d896ba338d65d711144bb5acc956205dbb28c081074a89"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)
STUDIO = os.environ.get("STUDIO_URL", "http://127.0.0.1:4176/")
TWIN = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4181/")
NAMES = [
    ("N-01", "Alpine 08"),
    ("N-02", "Alpine 09"),
    ("N-03", "Café"),
    ("N-04", "Cafe\u0301"),
    ("N-05", "François"),
    ("N-06", "Franc\u0327ois"),
    ("N-07", "José"),
    ("N-08", "Jose\u0301"),
    ("N-09", "Zoë"),
    ("N-10", "Zoe\u0308"),
    ("N-11", "Jürgen"),
    ("N-12", "Ju\u0308rgen"),
    ("N-13", "García"),
    ("N-14", "Garci\u0301a"),
    ("N-15", "Muñoz"),
    ("N-16", "Mun\u0303oz"),
    ("N-17", "O\u2019Neil"),
    ("N-18", "O'Neil"),
    ("N-19", "Van Laarhoven"),
    ("N-20", "Café"),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def snap(page):
    return page.evaluate("() => window.__nameDrop.snapshot()")


def row(page, item_id):
    found = [item for item in snap(page)["rows"] if item["item_id"] == item_id]
    assert len(found) == 1, item_id
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


def overflow(page) -> int:
    return page.evaluate(
        "() => document.documentElement.scrollWidth - document.documentElement.clientWidth"
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
          const select = document.querySelector('#name-drop-font-version');
          const font = document.querySelector('#name-drop-font');
          return select && select.options.length === 2 && select.value === '1'
            && font && font.textContent.includes('version 1')
            && font.textContent.includes('%s');
        }"""
        % FONT_V1,
        timeout=15000,
    )


def set_placement(page):
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


def arm_example(page):
    page.locator("#name-drop-example").click()
    page.locator("#name-drop-csv").set_input_files(str(FOUR))
    page.wait_for_function(
        """() => window.__nameDrop.snapshot().rows.length === 4
          && window.__nameDrop.snapshot().rows[0].text === 'ALPINE 08'
          && window.__nameDrop.snapshot().rows[3].text === 'ALPINE 08'""",
        timeout=5000,
    )
    set_placement(page)


def generate(page, count):
    page.locator("#name-drop-generate").focus()
    page.locator("#name-drop-generate").press("Enter")
    page.wait_for_function(
        """(count) => {
          const snap = window.__nameDrop.snapshot();
          return snap.running === false && snap.rows.length === count
            && document.querySelector('#name-drop-generate').getAttribute('aria-busy') === 'false';
        }""",
        arg=count,
        timeout=120000,
    )


def show_template(page):
    page.locator("#name-drop-template").click()
    page.wait_for_function(
        "() => window.__studioDesign.snapshot().inspection === null",
        timeout=15000,
    )


def select_item(page, item_id):
    page.locator(f'[aria-label="Show {item_id}"]').click()
    page.wait_for_function(
        """(itemId) => {
          const snap = window.__nameDrop.snapshot();
          const row = snap.rows.find((entry) => entry.item_id === itemId);
          const field = document.querySelector('#text-content');
          const revision = document.querySelector('#revision').textContent;
          const stitches = document.querySelector('#stitch-count').textContent;
          return snap.selected === itemId && row && row.artifactCurrent && field
            && field.value === row.text && !document.querySelector('#export').disabled
            && stitches !== 'None yet'
            && revision === row.project_revision.slice(0, 12);
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
    return proved


def save_download(page, selector, destination: Path) -> Path:
    with page.expect_download() as download_info:
        page.locator(selector).click()
    download = download_info.value
    destination.parent.mkdir(parents=True, exist_ok=True)
    download.save_as(destination)
    return destination


def open_facts(page):
    return page.evaluate(
        """() => {
          const view = window.__studioDesign.snapshot();
          const outline = view.project.objects.find((object) => object.kind === 'running');
          const text = view.project.objects.find((object) => object.kind === 'text');
          return {
            outline: JSON.stringify(outline),
            text: text.text.content,
            font_version: text.text.font_version,
            font_sha256: text.text.font_sha256,
            stitches: document.querySelector('#stitch-count').textContent,
            lines: document.querySelectorAll('#canvas line').length,
            proof: document.querySelector('#name-drop-proof').textContent,
            message: document.querySelector('#name-drop-message').textContent,
          };
        }"""
    )


def font_of(page):
    view = page.evaluate("() => window.__studioDesign.snapshot()")
    text = next(item for item in view["project"]["objects"] if item["kind"] == "text")
    return text["text"]


def main() -> None:
    errors: list[str] = []
    console: list[str] = []
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    badge_before = sha256(BADGE)
    expected = {item_id: text for item_id, text in NAMES}
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(page, errors, console)
        open_badge(page)
        assert "not the full line height" in page.locator("#fields").inner_text()
        arm_example(page)
        generate(page, 4)
        state = snap(page)
        assert state["certifies"] is True
        assert state["rows"][0]["dst_sha256"] == state["rows"][3]["dst_sha256"]
        assert state["rows"][0]["project_revision"] == state["rows"][3]["project_revision"]
        assert state["rows"][0]["dst_sha256"] != state["rows"][1]["dst_sha256"]
        write_proof(page, "ITEM-001")
        write_proof(page, "ITEM-002")
        select_item(page, "ITEM-001")
        first = open_facts(page)
        replay_one = save_download(page, "#name-drop-replay", EVIDENCE / "ITEM-001.replay.json")
        select_item(page, "ITEM-002")
        second = open_facts(page)
        replay_two = save_download(page, "#name-drop-replay", EVIDENCE / "ITEM-002.replay.json")
        assert first["outline"] == second["outline"]
        assert first["text"] == "ALPINE 08"
        assert second["text"] == "ALPINE 09"
        assert first["font_sha256"] == FONT_V1
        assert second["font_sha256"] == FONT_V1
        assert first["stitches"] != second["stitches"]
        assert first["lines"] != second["lines"]
        assert "ITEM-001" in first["proof"] and "ITEM-002" not in first["proof"]
        assert "ITEM-002" in second["proof"] and "ITEM-001" not in second["proof"]
        assert "ITEM-002" in second["message"]
        one = json.loads(replay_one.read_text())
        two = json.loads(replay_two.read_text())
        assert one["batch"]["item_id"] == "ITEM-001" and one["batch"]["text"] == "ALPINE 08"
        assert two["batch"]["item_id"] == "ITEM-002" and two["batch"]["text"] == "ALPINE 09"
        assert one["batch"]["font_version"] == "1"
        assert one["provenance"]["dst_sha256"] != two["provenance"]["dst_sha256"]

        show_template(page)
        page.locator("#name-drop-csv").set_input_files(str(CSV))
        page.wait_for_function(
            """() => window.__nameDrop.snapshot().rows.length === 20
              && window.__nameDrop.snapshot().rows[2].text === 'Café'""",
            timeout=5000,
        )
        set_placement(page)
        generate(page, 20)
        failed = snap(page)
        assert failed["certifies"] is False
        assert failed["counts"]["failed"] == 20
        assert failed["counts"]["completed"] == 0
        assert [item["item_id"] for item in failed["rows"]] == [item_id for item_id, _ in NAMES]
        for item in failed["rows"]:
            assert item["text"] == expected[item["item_id"]]
            assert item["artifactCurrent"] is False
            assert item["status"] == "failed"
        cafe_error = row(page, "N-03")["error"]
        assert "U+0061" in cafe_error
        assert "left unchanged" in cafe_error
        assert "version 2" not in cafe_error
        assert row(page, "N-03")["text"] == "Café"
        assert row(page, "N-04")["text"] == "Cafe\u0301"
        assert row(page, "N-04")["text"] != row(page, "N-03")["text"]
        assert "U+2019" in row(page, "N-17")["error"]
        assert row(page, "N-17")["text"] == "O\u2019Neil"
        assert "U+0065" in row(page, "N-18")["error"]
        assert row(page, "N-18")["text"] == "O'Neil"
        assert page.locator('[aria-label="Text for N-03"]').input_value() == "Café"

        page.locator("#name-drop-font-version").select_option("2")
        page.wait_for_function(
            """() => {
              const font = document.querySelector('#name-drop-font').textContent;
              const snap = window.__nameDrop.snapshot();
              return font.includes('version 2') && font.includes('%s')
                && snap.stale === true && snap.certifies === false
                && snap.rows.length === 20
                && snap.rows.every((row) => row.artifactCurrent === false)
                && document.querySelector('#name-drop-message').textContent.includes('entered names were not changed');
            }"""
            % FONT_V2,
            timeout=15000,
        )
        chosen = font_of(page)
        assert chosen["content"] == "ALPINE 08"
        assert chosen["font_version"] == "2"
        assert chosen["font_sha256"] == FONT_V2
        for item_id, text in NAMES:
            assert row(page, item_id)["text"] == text
        assert sha256(BADGE) == badge_before

        untouched = browser.new_page(viewport={"width": 1440, "height": 900}, accept_downloads=True)
        attach(untouched, errors, console)
        open_badge(untouched)
        assert "version 1" in untouched.locator("#name-drop-font").inner_text()
        assert FONT_V1 in untouched.locator("#name-drop-font").inner_text()
        assert FONT_V2 not in untouched.locator("#name-drop-font").inner_text()
        untouched.close()

        page.locator("#undo").click()
        page.wait_for_function(
            """() => document.querySelector('#name-drop-font').textContent.includes('version 1')
              && document.querySelector('#name-drop-font').textContent.includes('%s')
              && window.__nameDrop.snapshot().certifies === false"""
            % FONT_V1,
            timeout=5000,
        )
        assert font_of(page)["font_version"] == "1"
        assert row(page, "N-03")["text"] == "Café"
        page.locator("#redo").click()
        page.wait_for_function(
            """() => document.querySelector('#name-drop-font').textContent.includes('version 2')
              && document.querySelector('#name-drop-font').textContent.includes('%s')"""
            % FONT_V2,
            timeout=5000,
        )
        assert font_of(page)["font_version"] == "2"
        assert row(page, "N-20")["text"] == "Café"
        set_placement(page)
        generate(page, 20)
        sewn = snap(page)
        assert sewn["certifies"] is True
        assert sewn["counts"]["completed"] == 20
        assert sewn["stale"] is False
        for item in sewn["rows"]:
            assert item["text"] == expected[item["item_id"]]
            assert item["status"] == "completed"
            assert item["font_version"] == "2"
            assert item["font_sha256"] == FONT_V2
            assert item["artifactCurrent"] is True
        composed = row(page, "N-03")
        decomposed = row(page, "N-04")
        again = row(page, "N-20")
        assert composed["text"] != decomposed["text"]
        assert composed["dst_sha256"] == decomposed["dst_sha256"]
        assert composed["project_revision"] != decomposed["project_revision"]
        assert "U+00E9" in composed["glyph_codes"]
        assert "U+0301" not in decomposed["glyph_codes"]
        assert composed["glyph_codes"] == decomposed["glyph_codes"]
        assert composed["dst_sha256"] == again["dst_sha256"]
        assert composed["project_revision"] == again["project_revision"]
        assert composed["item_id"] != again["item_id"]
        assert row(page, "N-17")["dst_sha256"] != row(page, "N-18")["dst_sha256"]
        assert "U+2019" in row(page, "N-17")["glyph_codes"]
        assert "U+0027" in row(page, "N-18")["glyph_codes"]
        assert row(page, "N-05")["dst_sha256"] == row(page, "N-06")["dst_sha256"]
        assert row(page, "N-05")["text"] != row(page, "N-06")["text"]

        for item_id, _text in NAMES:
            write_proof(page, item_id)
        assert page.locator("#name-drop-bundle").is_enabled()
        assert row(page, "N-03")["proof_id"] != row(page, "N-20")["proof_id"]
        assert row(page, "N-03")["proof_id"] != row(page, "N-04")["proof_id"]
        assert row(page, "N-03")["dst_sha256"] == row(page, "N-04")["dst_sha256"]

        select_item(page, "N-03")
        project_path = save_download(page, "#name-drop-project", EVIDENCE / "N-03.embroidery-project.json")
        dst_path = save_download(page, "#name-drop-dst", EVIDENCE / "N-03.dst")
        replay_path = save_download(page, "#name-drop-replay", EVIDENCE / "N-03.replay.json")
        pdf_path = save_download(page, "#name-drop-pdf", EVIDENCE / "N-03.proof.pdf")
        select_item(page, "N-04")
        dst_nfd = save_download(page, "#name-drop-dst", EVIDENCE / "N-04.dst")
        select_item(page, "N-17")
        dst_curly = save_download(page, "#name-drop-dst", EVIDENCE / "N-17.dst")
        select_item(page, "N-18")
        dst_straight = save_download(page, "#name-drop-dst", EVIDENCE / "N-18.dst")
        bundle_path = save_download(page, "#name-drop-bundle", EVIDENCE / "name-drop-batch.json")

        saved = json.loads(project_path.read_text())
        text = next(item for item in saved["objects"] if item["kind"] == "text")
        assert text["points"] == []
        assert text["text"]["content"] == "Café"
        assert text["text"]["font_version"] == "2"
        assert text["text"]["font_sha256"] == FONT_V2
        assert dst_path.read_bytes()[:3] == b"LA:"
        assert sha256(dst_path) == row(page, "N-03")["dst_sha256"]
        assert dst_path.read_bytes() == dst_nfd.read_bytes()
        assert dst_curly.read_bytes() != dst_straight.read_bytes()
        replay = json.loads(replay_path.read_text())
        assert replay["protocol"] == "embroidery-project-replay/1"
        assert replay["batch"]["item_id"] == "N-03"
        assert replay["batch"]["text"] == "Café"
        assert replay["batch"]["font_version"] == "2"
        assert replay["batch"]["font_sha256"] == FONT_V2
        assert replay["provenance"]["dst_sha256"] == sha256(dst_path)
        assert pdf_path.read_bytes().startswith(b"%PDF")
        bundle = json.loads(bundle_path.read_text())
        assert bundle["schema"] == "embroideryos-name-drop-batch/0.1"
        assert bundle["certifies_batch"] is True
        assert bundle["machine_release_allowed"] is False
        assert bundle["physical_qualification"] is False
        assert len(bundle["items"]) == 20
        assert [item["text"] for item in bundle["items"]] == [text for _item_id, text in NAMES]
        assert bundle["items"][2]["dst_sha256"] == bundle["items"][3]["dst_sha256"]
        assert bundle["items"][2]["proof_id"] != bundle["items"][3]["proof_id"]
        assert bundle["items"][2]["proof_id"] != bundle["items"][19]["proof_id"]
        assert bundle["items"][2]["binding"]["pdf_sha256"] == sha256(pdf_path)
        assert all(item["font_version"] == "2" for item in bundle["items"])
        assert all(item["binding"]["machine_release_allowed"] is False for item in bundle["items"])

        page.locator("#open-project").set_input_files(str(project_path))
        page.wait_for_function(
            """() => {
              const view = window.__studioDesign.snapshot();
              const text = view.project.objects.find((object) => object.kind === 'text');
              return text.text.content === 'Café'
                && text.text.font_version === '2'
                && text.text.font_sha256 === '%s'
                && window.__nameDrop.snapshot().stale === true
                && window.__nameDrop.snapshot().certifies === false;
            }"""
            % FONT_V2,
            timeout=15000,
        )
        assert sha256(BADGE) == badge_before

        page.set_viewport_size({"width": 1440, "height": 1000})
        page.locator("#objects button", has_text="Lettering").click()
        page.screenshot(path=str(EVIDENCE / "studio-desktop.png"), full_page=True)
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 768, "height": 1024})
        page.locator("#name-drop").scroll_into_view_if_needed()
        page.screenshot(path=str(EVIDENCE / "studio-tablet.png"), full_page=True)
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 390, "height": 844})
        page.locator("#name-drop-font-version").focus()
        assert page.locator("#name-drop-font-version").is_enabled()
        page.locator('[aria-label="Text for N-03"]').scroll_into_view_if_needed()
        assert page.locator('[aria-label="Text for N-03"]').input_value() == "Café"
        page.screenshot(path=str(EVIDENCE / "studio-mobile.png"), full_page=True)
        assert overflow(page) <= 1

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
              return snap.name_drop && snap.name_drop.item_id === 'N-03'
                && snap.name_drop.design_replaced === false
                && snap.design.name === 'Alpine mark'
                && snap.name_drop.proof_current === false;
            }""",
            timeout=15000,
        )
        binding = {
            "schema": "embroideryos-name-drop-binding/0.1",
            "snapshot_id": replay["batch"]["snapshot_id"],
            "item_id": "N-04",
            "project_revision": replay["batch"]["project_revision"],
            "dst_sha256": replay["provenance"]["dst_sha256"],
            "machine_release_allowed": False,
            "physical_qualification": False,
        }
        mismatch_path = EVIDENCE / "mismatch-binding.json"
        mismatch_path.write_text(json.dumps(binding))
        twin.locator("#name-drop-proof-input").set_input_files(str(mismatch_path))
        twin.wait_for_function(
            """() => window.__twinlab.snapshot().name_drop.proof_current === false
              && window.__twinlab.snapshot().design.name === 'Alpine mark'""",
            timeout=5000,
        )
        binding["item_id"] = "N-03"
        match_path = EVIDENCE / "match-binding.json"
        match_path.write_text(json.dumps(binding))
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
        twin.locator("#clear-name-drop").click()
        twin.wait_for_function(
            """() => window.__twinlab.snapshot().name_drop === null
              && window.__twinlab.snapshot().design.name === 'Alpine mark'""",
            timeout=5000,
        )
        twin.screenshot(path=str(EVIDENCE / "twin-mobile.png"), full_page=True)
        assert overflow(twin) <= 1
        browser.close()

    assert errors == [], errors
    assert console == [], console
    assert sha256(BADGE) == badge_before
    print("lettering v2 browser passed")
    print("dst", sha256(dst_path))
    print("pdf", sha256(pdf_path))
    print("font", FONT_V2)


if __name__ == "__main__":
    main()
