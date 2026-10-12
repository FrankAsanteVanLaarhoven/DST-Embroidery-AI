"""Browser check for explicit horizontal lines on one text object.

Enter inserts a line. Command or Control Enter generates. A quoted CSV keeps
one item per record. Evidence is written under evidence/lettering-l4.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "lettering-l4"
BADGE = ROOT / "fixtures" / "name-drop" / "badge.json"
CSV = ROOT / "fixtures" / "name-drop" / "lettering-l4-names.csv"
FONT_V1 = "aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd"
FONT_V2 = "78e8506438960cc575d896ba338d65d711144bb5acc956205dbb28c081074a89"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)
STUDIO = os.environ.get("STUDIO_URL", "http://127.0.0.1:4186/")
TWIN = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4187/")
DEMO = "Café\nAlpine 08"
NAMES = [
    ("ML-001", "Café\nAlpine 08"),
    ("ML-002", "Café\nAlpine 09"),
    ("ML-003", "Cafe\u0301\nAlpine 08"),
    ("ML-004", "Café\nAlpine 08"),
    ("ML-005", "Jürgen\nÉquipe"),
    ("ML-006", "O'Neil\nA"),
    ("ML-007", "O\u2019Neil\nA"),
    ("ML-008", "Alpha\n\nBeta"),
    ("ML-009", "A\n  \nB"),
    ("ML-010", "North\nRidge\nClub\n08"),
    ("ML-011", "François\n08"),
    ("ML-012", "Franc\u0327ois\n08"),
    ("ML-013", "Zoë\nTeam"),
    ("ML-014", "Zoe\u0308\nTeam"),
    ("ML-015", "García\n09"),
    ("ML-016", "Garci\u0301a\n09"),
    ("ML-017", "Muñoz\n10"),
    ("ML-018", "José\n11"),
    ("ML-019", "One"),
    ("ML-020", "Alpine\nClub"),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def snap(page):
    return page.evaluate("() => window.__nameDrop.snapshot()")


def row(page, item_id):
    found = [item for item in snap(page)["rows"] if item["item_id"] == item_id]
    assert len(found) == 1, item_id
    return found[0]


def design_text(page):
    return page.evaluate(
        """() => {
          const project = window.__studioDesign.snapshot().project;
          const object = project.objects.find((entry) => entry.kind === 'text');
          return object.text;
        }"""
    )


def attach(page, errors, console):
    page.on("dialog", lambda dialog: dialog.accept())
    page.on("pageerror", lambda error: errors.append(getattr(error, "stack", None) or str(error)))
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
          && document.querySelectorAll('h1').length === 1
          && !document.querySelector('#generate').disabled""",
        timeout=20000,
    )
    page.locator("#objects button", has_text="Lettering").click()
    page.wait_for_selector("#text-content", timeout=10000)


def set_number(page, selector, value):
    field = page.locator(selector)
    field.fill(value)
    field.dispatch_event("change")


def configure_lines(page):
    page.wait_for_selector("#name-drop-font-version", timeout=10000)
    page.locator("#name-drop-font-version").select_option("2")
    page.wait_for_function(
        """() => document.querySelector('#name-drop-font').textContent.includes('version 2')
          && document.querySelector('#name-drop-font').textContent.includes('%s')"""
        % FONT_V2,
        timeout=10000,
    )
    move_y = page.get_by_label("Move Y · mm")
    move_y.fill("8")
    move_y.dispatch_event("change")
    page.locator("#line-layout").check()
    page.wait_for_selector("#line-alignment", timeout=5000)
    page.locator("#line-alignment").select_option("centre")
    set_number(page, "#line-block-width", "100")
    set_number(page, "#line-spacing", "18")
    text = page.locator("#text-content")
    text.fill(DEMO)
    text.dispatch_event("change")
    page.wait_for_function(
        """() => {
          const text = window.__studioDesign.snapshot().project.objects.find((entry) => entry.kind === 'text').text;
          return text.content === 'Café\\nAlpine 08'
            && text.layout && text.layout.alignment === 'centre'
            && text.layout.policy === 'embroideryos-line-layout/1'
            && text.font_version === '2';
        }""",
        timeout=5000,
    )


def set_placement(page):
    for key, value in (("min-x", "2"), ("min-y", "2"), ("max-x", "118"), ("max-y", "78")):
        set_number(page, "#name-drop-" + key, value)
    page.wait_for_function(
        """() => document.querySelector('#name-drop-min-y').value === '2'
          && document.querySelector('#name-drop-max-y').value === '78'""",
        timeout=5000,
    )


def wait_generated(page):
    page.wait_for_function(
        """() => !document.querySelector('#generate').disabled
          && document.querySelector('#generate').textContent.includes('Generate stitches')
          && document.querySelector('#errors').textContent === ''
          && document.querySelector('#stitch-count').textContent !== 'None yet'""",
        timeout=30000,
    )


def select_item(page, item_id):
    page.locator(f'[aria-label="Show {item_id}"]').click()
    page.wait_for_function(
        """(itemId) => {
          const snap = window.__nameDrop.snapshot();
          const row = snap.rows.find((entry) => entry.item_id === itemId);
          const field = document.querySelector('#text-content');
          return snap.selected === itemId && row && row.artifactCurrent && field
            && field.value === row.text && !document.querySelector('#export').disabled
            && document.querySelector('#stitch-count').textContent !== 'None yet';
        }""",
        arg=item_id,
        timeout=60000,
    )


def save_download(page, selector, destination: Path) -> Path:
    with page.expect_download() as download_info:
        page.locator(selector).click()
    download = download_info.value
    destination.parent.mkdir(parents=True, exist_ok=True)
    download.save_as(destination)
    return destination


def replay_ink(pdf_path: Path):
    import fitz

    doc = fitz.open(pdf_path)
    try:
        page = doc[0]
        text = "".join(item.get_text() for item in doc)
        width, height = page.rect.width, page.rect.height
        panel_width = (width - 88) / 2
        panel_height = 288
        reportlab_y = height - 386
        clip = fitz.Rect(
            52 + panel_width,
            height - (reportlab_y + panel_height),
            52 + panel_width * 2,
            height - reportlab_y,
        )
        pix = page.get_pixmap(matrix=fitz.Matrix(2, 2), clip=clip, alpha=False)
        ink = 0
        samples = pix.samples
        for index in range(0, len(samples), 3):
            if samples[index] < 230 or samples[index + 1] < 230 or samples[index + 2] < 230:
                ink += 1
        return text, ink, pix
    finally:
        doc.close()


def hold_second(page, item_id):
    gate = {"route": None}

    def handle(route):
        payload = json.loads(route.request.post_data or "{}")
        if payload.get("item_id") == item_id and gate["route"] is None:
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
    badge_before = sha256(BADGE)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            headless=True,
            executable_path=BROWSER,
            args=["--no-sandbox", "--disable-dev-shm-usage"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(page, errors, console)
        requests = []
        page.on("request", lambda request: requests.append(request.url))
        open_badge(page)
        requests.clear()
        before_enter = page.locator("#text-content").input_value()
        page.locator("#text-content").press("End")
        page.locator("#text-content").press("Enter")
        page.wait_for_timeout(500)
        after_enter = page.locator("#text-content").input_value()
        assert after_enter == before_enter + "\n"
        assert not any(url.endswith("/api/generate") for url in requests)
        configure_lines(page)
        requests.clear()
        page.locator("#text-content").press("Control+Enter")
        wait_generated(page)
        assert any(url.endswith("/api/generate") for url in requests)
        saved_text = design_text(page)
        assert saved_text["content"] == DEMO
        assert saved_text["layout"]["alignment"] == "centre"
        assert saved_text["layout"]["baseline_spacing_mm"] == 18
        assert saved_text["font_sha256"] == FONT_V2
        assert "layout" not in json.loads(BADGE.read_text())["objects"][1]["text"]
        project_path = save_download(page, "#save-project", EVIDENCE / "template.embroidery-project.json")
        opened = json.loads(project_path.read_text())
        opened_text = next(item["text"] for item in opened["objects"] if item["kind"] == "text")
        assert opened_text["content"] == DEMO
        assert opened_text["layout"]["policy"] == "embroideryos-line-layout/1"
        page.locator("#open-project").set_input_files(str(project_path))
        page.wait_for_function(
            """() => document.querySelector('#project-title').textContent === 'Name drop badge'
              && !document.querySelector('#generate').disabled
              && document.querySelector('#errors').textContent === ''""",
            timeout=30000,
        )
        page.locator("#objects button", has_text="Lettering").click()
        page.wait_for_function(
            """() => {
              const field = document.querySelector('#text-content');
              return field && field.value === 'Café\\nAlpine 08'
                && document.querySelector('#line-alignment').value === 'centre';
            }""",
            timeout=10000,
        )
        requests.clear()
        page.locator("#name-drop-add").click()
        table_text = page.locator("[data-text]").first
        table_text.click()
        table_text.press("Enter")
        page.wait_for_timeout(200)
        assert "\n" in table_text.input_value()
        assert not any("/api/batch/item" in url for url in requests)
        page.locator("#name-drop-csv").set_input_files(str(CSV))
        page.wait_for_function(
            """() => window.__nameDrop.snapshot().rows.length === 20
              && window.__nameDrop.snapshot().rows[0].text === 'Café\\nAlpine 08'
              && window.__nameDrop.snapshot().rows[2].text.includes('Alpine 08')""",
            timeout=5000,
        )
        assert row(page, "ML-003")["text"] == "Cafe\u0301\nAlpine 08"
        assert row(page, "ML-003")["text"] != row(page, "ML-001")["text"]
        assert row(page, "ML-008")["text"] == "Alpha\n\nBeta"
        assert row(page, "ML-009")["text"] == "A\n  \nB"
        assert len(snap(page)["rows"]) == 20
        set_placement(page)
        page.locator("#name-drop-generate").click()
        page.wait_for_function(
            """() => {
              const snap = window.__nameDrop.snapshot();
              return snap.running === false && snap.rows.length === 20
                && document.querySelector('#name-drop-generate').getAttribute('aria-busy') === 'false';
            }""",
            timeout=180000,
        )
        state = snap(page)
        assert state["certifies"] is True, json.dumps(
            [{"id": item["item_id"], "status": item["status"], "error": item["error"]} for item in state["rows"]],
            ensure_ascii=False,
        )
        assert [item["item_id"] for item in state["rows"]] == [item_id for item_id, _text in NAMES]
        for item_id, text in NAMES:
            item = row(page, item_id)
            assert item["text"] == text
            assert item["status"] == "completed"
            assert item["font_version"] == "2"
            assert item["font_sha256"] == FONT_V2
            assert item["artifactCurrent"] is True
            assert "U+000A" not in item["glyph_codes"]
        assert row(page, "ML-001")["dst_sha256"] == row(page, "ML-003")["dst_sha256"]
        assert row(page, "ML-001")["project_revision"] != row(page, "ML-003")["project_revision"]
        assert row(page, "ML-001")["dst_sha256"] == row(page, "ML-004")["dst_sha256"]
        assert row(page, "ML-001")["project_revision"] == row(page, "ML-004")["project_revision"]
        assert row(page, "ML-001")["item_id"] != row(page, "ML-004")["item_id"]
        assert row(page, "ML-006")["dst_sha256"] != row(page, "ML-007")["dst_sha256"]
        assert "U+0020" in row(page, "ML-009")["glyph_codes"]
        select_item(page, "ML-001")
        assert page.locator("#text-content").input_value() == DEMO
        item_project = save_download(page, "#name-drop-project", EVIDENCE / "ML-001.embroidery-project.json")
        item_dst = save_download(page, "#name-drop-dst", EVIDENCE / "ML-001.dst")
        page.locator("#name-drop-write").click()
        page.wait_for_function(
            """() => {
              const row = window.__nameDrop.snapshot().rows.find((entry) => entry.item_id === 'ML-001');
              return row && row.proofCurrent && !document.querySelector('#name-drop-replay').disabled;
            }""",
            timeout=30000,
        )
        replay_path = save_download(page, "#name-drop-replay", EVIDENCE / "ML-001.replay.json")
        pdf_path = save_download(page, "#name-drop-pdf", EVIDENCE / "ML-001.proof.pdf")
        stored = json.loads(item_project.read_text())
        stored_object = next(item for item in stored["objects"] if item["kind"] == "text")
        assert stored_object["text"]["content"] == DEMO
        assert stored_object["text"]["layout"]["alignment"] == "centre"
        assert stored_object["points"] == []
        assert item_dst.read_bytes()[:3] == b"LA:"
        assert sha256(item_dst) == row(page, "ML-001")["dst_sha256"]
        replay = json.loads(replay_path.read_text())
        assert replay["protocol"] == "embroidery-project-replay/1"
        assert replay["batch"]["item_id"] == "ML-001"
        assert replay["batch"]["text"] == DEMO
        assert replay["batch"]["font_sha256"] == FONT_V2
        labels, ink, pix = replay_ink(pdf_path)
        assert "DST replay view" in labels
        assert "Engineering review only" in labels
        assert ink > 20
        assert labels != DEMO
        pix.save(str(EVIDENCE / "ML-001-replay.png"))
        assert row(page, "ML-001")["proof_id"]
        assert pdf_path.read_bytes().startswith(b"%PDF")
        select_item(page, "ML-004")
        page.locator("#name-drop-write").click()
        page.wait_for_function(
            """() => {
              const snap = window.__nameDrop.snapshot();
              const first = snap.rows.find((entry) => entry.item_id === 'ML-001');
              const duplicate = snap.rows.find((entry) => entry.item_id === 'ML-004');
              return first.proofCurrent && duplicate.proofCurrent && first.proof_id !== duplicate.proof_id;
            }""",
            timeout=30000,
        )
        select_item(page, "ML-001")
        spacing = page.locator("#line-spacing")
        spacing.fill("24")
        spacing.dispatch_event("change")
        page.wait_for_function(
            """() => {
              const snap = window.__nameDrop.snapshot();
              const row = snap.rows.find((entry) => entry.item_id === 'ML-001');
              return row.proofCurrent === false && snap.certifies === false
                && document.querySelector('#name-drop-bundle').disabled
                && document.querySelector('#text-content').value === 'Café\\nAlpine 08';
            }""",
            timeout=5000,
        )
        page.locator("#undo").click()
        page.wait_for_function(
            """() => document.querySelector('#line-spacing').value === '18'
              && window.__nameDrop.snapshot().certifies === false
              && window.__nameDrop.snapshot().rows.find((entry) => entry.item_id === 'ML-001').proofCurrent === false
              && document.querySelector('#text-content').value === 'Café\\nAlpine 08'""",
            timeout=5000,
        )
        page.locator("#redo").click()
        page.wait_for_function(
            """() => document.querySelector('#line-spacing').value === '24'
              && window.__nameDrop.snapshot().certifies === false
              && window.__studioDesign.snapshot().selected""",
            timeout=5000,
        )
        page.screenshot(path=str(EVIDENCE / "studio-desktop.png"), full_page=True)
        assert overflow(page) <= 1
        page.set_viewport_size({"width": 768, "height": 1024})
        page.wait_for_timeout(200)
        assert overflow(page) <= 1
        assert page.locator("#text-content").input_value() == DEMO
        page.screenshot(path=str(EVIDENCE / "studio-tablet.png"), full_page=True)
        page.set_viewport_size({"width": 390, "height": 844})
        page.wait_for_timeout(200)
        assert overflow(page) <= 1
        page.locator("#text-content").focus()
        assert page.locator("#generate").is_enabled() or page.locator("#generate").is_visible()
        page.screenshot(path=str(EVIDENCE / "studio-mobile.png"), full_page=True)

        mobile = browser.new_page(viewport={"width": 390, "height": 844}, accept_downloads=True)
        attach(mobile, errors, console)
        open_badge(mobile)
        configure_lines(mobile)
        mobile.locator("#generate").click()
        wait_generated(mobile)
        assert design_text(mobile)["content"] == DEMO
        assert design_text(mobile)["layout"]["alignment"] == "centre"
        assert overflow(mobile) <= 1
        mobile.screenshot(path=str(EVIDENCE / "studio-mobile-generated.png"), full_page=True)
        mobile.close()

        cancel = browser.new_page(viewport={"width": 1440, "height": 900}, accept_downloads=True)
        attach(cancel, errors, console)
        open_badge(cancel)
        configure_lines(cancel)
        tiny = EVIDENCE / "two-items.csv"
        tiny.write_text('item_id,text\nML-001,"Café\nAlpine 08"\nML-002,"Café\nAlpine 09"\n', encoding="utf-8")
        cancel.locator("#name-drop-csv").set_input_files(str(tiny))
        cancel.wait_for_function(
            "() => window.__nameDrop.snapshot().rows.length === 2",
            timeout=5000,
        )
        set_placement(cancel)
        gate = hold_second(cancel, "ML-002")
        cancel.locator("#name-drop-generate").click()
        cancel.wait_for_function(
            """() => {
              const row = window.__nameDrop.snapshot().rows.find((entry) => entry.item_id === 'ML-001');
              return row && row.status === 'completed' && row.artifactCurrent === true;
            }""",
            timeout=60000,
        )
        cancel.locator("#name-drop-cancel").click()
        cancel.wait_for_function(
            """() => {
              const snap = window.__nameDrop.snapshot();
              const second = snap.rows.find((entry) => entry.item_id === 'ML-002');
              return snap.running === false && snap.certifies === false && second.status === 'cancelled';
            }""",
            timeout=10000,
        )
        release(gate)
        cancel.wait_for_timeout(500)
        late = snap(cancel)
        assert late["certifies"] is False
        assert late["rows"][0]["status"] == "completed"
        assert late["rows"][0]["proofCurrent"] is False
        assert late["rows"][1]["status"] == "cancelled"
        assert late["rows"][1]["proofCurrent"] is False
        assert late["rows"][1]["text"] == "Café\nAlpine 09"
        assert cancel.locator("#name-drop-bundle").is_disabled()
        cancel.close()

        twin = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
        attach(twin, errors, console)
        twin.goto(TWIN, wait_until="domcontentloaded")
        twin.wait_for_function(
            """() => document.querySelector('h1').textContent.includes('See every stitch')
              && /^[a-f0-9]{64}$/.test(document.querySelector('#hash').textContent)""",
            timeout=30000,
        )
        twin_design = {
            "schema_version": 1,
            "name": "Lined badge",
            "revision": 1,
            "objects": [{
                "id": 1,
                "label": "Lettering",
                "kind": "text",
                "points": [],
                "rgb": [30, 90, 160],
                "spacing_mm": 0.6,
                "angle_degrees": 0,
                "max_stitch_mm": 3,
                "width_mm": 1.6,
                "compensation_mm": 0,
                "underlay_mm": 0,
                "text": {
                    "content": DEMO,
                    "font_id": "embroideryos-block-01",
                    "font_version": "2",
                    "font_sha256": FONT_V2,
                    "origin_x_mm": -40,
                    "origin_y_mm": -20,
                    "cap_height_mm": 8,
                    "letter_spacing_mm": 0.8,
                    "word_spacing_mm": 3.6,
                    "layout": {
                        "policy": "embroideryos-line-layout/1",
                        "alignment": "centre",
                        "block_width_mm": 50,
                        "first_baseline_mm": -12.64,
                        "baseline_spacing_mm": 16,
                    },
                },
            }],
        }
        twin_path = EVIDENCE / "twin-open.json"
        twin_path.write_text(json.dumps(twin_design), encoding="utf-8")
        twin_requests = []
        twin.on("request", lambda request: twin_requests.append(request.url) if request.method == "POST" else None)
        twin.locator("#file-input").set_input_files(str(twin_path))
        twin.wait_for_function(
            """() => document.querySelector('#text-content')
              && document.querySelector('#text-content').value === 'Café\\nAlpine 08'
              && document.querySelector('#line-alignment').value === 'centre'
              && /^[a-f0-9]{64}$/.test(document.querySelector('#hash').textContent)
              && !document.querySelector('#compile').disabled
              && !document.querySelector('#message').classList.contains('error')""",
            timeout=30000,
        )
        twin_requests.clear()
        twin.locator("#text-content").press("End")
        twin.locator("#text-content").press("Enter")
        twin.wait_for_timeout(700)
        assert twin.locator("#text-content").input_value().endswith("\n")
        assert not any(url.endswith("/api/compile") for url in twin_requests)
        twin.locator("#text-content").fill(DEMO)
        twin.locator("#text-content").dispatch_event("change")
        baseline = twin.locator("#line-baseline").input_value()
        set_number(twin, "#text-origin-y", "-18")
        assert twin.locator("#line-baseline").input_value() == baseline
        twin.locator("#compile").click()
        twin.wait_for_function(
            """() => /^[a-f0-9]{64}$/.test(document.querySelector('#hash').textContent)
              && !document.querySelector('#compile').disabled
              && !document.querySelector('#message').classList.contains('error')""",
            timeout=30000,
        )
        first_hash = twin.locator("#hash").inner_text()
        set_number(twin, "#line-spacing", "20")
        twin.locator("#compile").click()
        twin.wait_for_function(
            """(previous) => document.querySelector('#hash').textContent !== previous
              && /^[a-f0-9]{64}$/.test(document.querySelector('#hash').textContent)
              && !document.querySelector('#message').classList.contains('error')""",
            arg=first_hash,
            timeout=30000,
        )
        twin.locator("#undo").click()
        twin.locator("#compile").click()
        twin.wait_for_function(
            """(previous) => document.querySelector('#hash').textContent === previous
              && !document.querySelector('#message').classList.contains('error')""",
            arg=first_hash,
            timeout=30000,
        )
        twin.locator("#name-drop-input").set_input_files(str(replay_path))
        twin.wait_for_function(
            """() => document.querySelector('#message').textContent.includes('ML-001')
              && document.querySelector('#name').value === 'Lined badge'""",
            timeout=5000,
        )
        twin.set_viewport_size({"width": 390, "height": 844})
        twin.wait_for_timeout(200)
        assert overflow(twin) <= 1
        twin.screenshot(path=str(EVIDENCE / "twin-mobile.png"), full_page=True)
        twin.set_viewport_size({"width": 1440, "height": 1000})
        twin.screenshot(path=str(EVIDENCE / "twin-desktop.png"), full_page=True)
        browser.close()

    assert sha256(BADGE) == badge_before
    assert not errors, errors
    assert not console, console
    print("lettering l4 browser passed")


if __name__ == "__main__":
    main()
