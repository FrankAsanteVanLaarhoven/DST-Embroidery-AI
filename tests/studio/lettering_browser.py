"""Browser check for native lettering in the studio.

ALPINE 08 stays a text object through Rust generation, a text edit, Undo,
save and reopen, and the exact DST download. ALPINE 09 is a later revision.
O'NEIL-08 exercises both supported punctuation marks. A lowercase string is
rejected without changing the entered characters or the previous plan.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "lettering"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)


def dst_sha(payload):
    return hashlib.sha256(bytes.fromhex(payload["dst_hex"])).hexdigest()


def text_object(project):
    found = [item for item in project["objects"] if item.get("kind") == "text"]
    assert len(found) == 1, found
    return found[0]


def phase_for(payload, object_id):
    summary = payload["plan"]["metadata"]["satin_phase_summary"]
    marker = f"Object {object_id}:"
    start = summary.find(marker)
    assert start >= 0, summary
    rest = summary[start:]
    nxt = rest.find(" Object ", len(marker))
    return rest if nxt < 0 else rest[:nxt]


def generate(page):
    with page.expect_response("**/api/generate") as generated:
        page.locator("#generate").click()
    response = generated.value
    payload = response.json()
    assert response.status == 200, payload
    page.wait_for_function(
        """() => document.querySelector('#errors').textContent === ''
          && !document.querySelector('#revision').textContent.includes('not generated')
          && document.querySelector('#phases').textContent.includes('EmbroideryOS Block 01')
          && document.querySelector('#text-content')""",
        timeout=30000,
    )
    return payload, json.loads(response.request.post_data)


def set_text(page, value):
    field = page.locator("#text-content")
    field.fill(value)
    field.dispatch_event("change")
    page.wait_for_function(
        """(value) => document.querySelector('#text-content').value === value
          && document.querySelector('#revision').textContent.includes('not generated')""",
        arg=value,
        timeout=5000,
    )


def assert_lettering(payload, project, content):
    item = text_object(project)
    assert project["format_version"] == "2"
    assert item["points"] == []
    assert item["text"]["content"] == content
    assert item["text"]["font_id"] == "embroideryos-block-01"
    assert item["text"]["font_version"] == "1"
    assert len(item["text"]["font_sha256"]) == 64
    meta = payload["plan"]["metadata"]
    assert content in meta["lettering_text"]
    assert "lettering" in payload["provenance"]["generators"]
    assert payload["readiness"]["machine_release_allowed"] is False
    assert payload["readiness"]["engineering_export_allowed"] is True
    return item


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
        page = browser.new_page(viewport={"width": 1440, "height": 1000}, accept_downloads=True)
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
        page.locator("#add-text").click()
        page.wait_for_function(
            """() => document.querySelector('#inspector-name').textContent === 'Lettering'
              && document.querySelector('#text-content').value === 'ALPINE 08'
              && document.querySelector('#text-geometry-note')
              && document.querySelector('#font-pin').textContent.includes('Pinned to the engine recipe')
              && document.querySelector('#cap-height').value === '12'
              && !document.querySelector('#centre-run').checked
              && !document.querySelector('#edge-run').checked""",
            timeout=5000,
        )
        assert "browser font" in page.locator("#text-geometry-note").inner_text()
        assert "Text · ALPINE 08" in page.locator("#objects button", has_text="Lettering").inner_text()
        page.locator("#centre-run").check()
        page.locator("#edge-run").check()
        alpine, alpine_project = generate(page)
        alpine_text = assert_lettering(alpine, alpine_project, "ALPINE 08")
        alpine_id = alpine_text["object_id"]
        alpine_phase = phase_for(alpine, alpine_id)
        alpine_top = alpine["plan"]["metadata"]["satin_top_sequence"]
        alpine_hash = dst_sha(alpine)
        alpine_revision = alpine["revision"]
        trace = alpine["plan"]["metadata"]["lettering_trace"]
        assert "U+0041" in trace and "U+0020 space" in trace and "U+0038" in trace
        assert "Centre-run underlay 0" not in alpine_phase
        assert "Edge-run underlay 0" not in alpine_phase
        assert "Top satin" in alpine_phase
        assert "Positioning and inter-phase connectors" in alpine_phase
        assert int(page.locator("#stitch-count").inner_text()) > 4701
        shown_revision = page.locator("#revision").inner_text()
        page.locator("#geometry button", has_text="Right").click()
        page.wait_for_function(
            """() => document.querySelector('#revision').textContent.includes('not generated')
              && document.querySelector('#export').disabled
              && document.querySelector('#inspector-name').textContent === 'Lettering'""",
            timeout=5000,
        )
        assert page.locator("#fields label", has_text="Move X").locator("input").input_value() == "19"
        page.locator("#undo").click()
        page.wait_for_function(
            """(revision) => document.querySelector('#revision').textContent === revision
              && document.querySelector('#text-content').value === 'ALPINE 08'
              && !document.querySelector('#export').disabled""",
            arg=shown_revision,
            timeout=5000,
        )
        spacing = page.locator("#letter-spacing")
        spacing.fill("1.2")
        spacing.dispatch_event("change")
        page.wait_for_function(
            """() => document.querySelector('#revision').textContent.includes('not generated')
              && document.querySelector('#letter-spacing').value === '1.2'
              && document.querySelector('#export').disabled""",
            timeout=5000,
        )
        page.locator("#undo").click()
        page.wait_for_function(
            """(revision) => document.querySelector('#revision').textContent === revision
              && document.querySelector('#letter-spacing').value === '0.8'""",
            arg=shown_revision,
            timeout=5000,
        )
        set_text(page, "ALPINE 09")
        page.locator("#undo").click()
        page.wait_for_function(
            "() => document.querySelector('#text-content').value === 'ALPINE 08'",
            timeout=5000,
        )
        page.locator("#redo").click()
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === 'ALPINE 09'
              && document.querySelector('#revision').textContent.includes('not generated')
              && document.querySelector('#inspector-name').textContent === 'Lettering'""",
            timeout=5000,
        )
        changed, _changed_project = generate(page)
        changed_hash = dst_sha(changed)
        assert changed["revision"] != alpine_revision
        assert changed_hash != alpine_hash
        assert "ALPINE 09" in changed["plan"]["metadata"]["lettering_text"]
        assert page.locator("#text-content").input_value() == "ALPINE 09"
        page.locator("#undo").click()
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === 'ALPINE 08'
              && document.querySelector('#revision').textContent.includes('not generated')
              && document.querySelector('#export').disabled
              && document.querySelector('#inspector-name').textContent === 'Lettering'""",
            timeout=5000,
        )
        restored, _restored_project = generate(page)
        assert restored["revision"] == alpine_revision
        assert dst_sha(restored) == alpine_hash
        assert restored["plan"]["metadata"]["satin_top_sequence"] == alpine_top
        page.locator("#centre-run").uncheck()
        edge, _edge_project = generate(page)
        edge_phase = phase_for(edge, alpine_id)
        assert edge["plan"]["metadata"]["satin_top_sequence"] == alpine_top
        assert dst_sha(edge) != alpine_hash
        assert "Centre-run underlay 0" in edge_phase
        assert "Edge-run underlay 0" not in edge_phase
        page.locator("#edge-run").uncheck()
        none, _none_project = generate(page)
        none_phase = phase_for(none, alpine_id)
        assert none["plan"]["metadata"]["satin_top_sequence"] == alpine_top
        assert "Centre-run underlay 0" in none_phase
        assert "Edge-run underlay 0" in none_phase
        page.locator("#centre-run").check()
        centre, _centre_project = generate(page)
        centre_phase = phase_for(centre, alpine_id)
        assert centre["plan"]["metadata"]["satin_top_sequence"] == alpine_top
        assert "Centre-run underlay 0" not in centre_phase
        assert "Edge-run underlay 0" in centre_phase
        page.locator("#edge-run").check()
        repeated, _repeated_project = generate(page)
        assert repeated["plan"]["metadata"]["satin_top_sequence"] == alpine_top
        assert dst_sha(repeated) == alpine_hash
        set_text(page, "O'NEIL-08")
        neil, neil_project = generate(page)
        neil_text = assert_lettering(neil, neil_project, "O'NEIL-08")
        neil_trace = neil["plan"]["metadata"]["lettering_trace"]
        assert "U+0027" in neil_trace and "U+002D" in neil_trace
        assert neil_text["text"]["font_sha256"] == alpine_text["text"]["font_sha256"]
        with page.expect_download() as project_download:
            page.locator("#save-project").click()
        project_path = EVIDENCE / "studio-project.json"
        project_download.value.save_as(project_path)
        saved = json.loads(project_path.read_text())
        saved_text = text_object(saved)
        assert saved["format_version"] == "2"
        assert saved_text["text"]["content"] == "O'NEIL-08"
        assert saved_text["text"]["font_sha256"] == alpine_text["text"]["font_sha256"]
        assert saved_text["points"] == []
        with page.expect_response("**/api/generate") as reopened:
            page.locator("#open-project").set_input_files(project_path)
        reopened_payload = reopened.value.json()
        assert reopened.value.status == 200, reopened_payload
        page.locator("#objects button", has_text="Lettering").click()
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === "O'NEIL-08"
              && document.querySelector('#font-pin').textContent.includes('Pinned to the engine recipe')
              && document.querySelector('#phases').textContent.includes('EmbroideryOS Block 01')""",
            timeout=30000,
        )
        assert dst_sha(reopened_payload) == dst_sha(neil)
        assert reopened_payload["readiness"]["machine_release_allowed"] is False
        with page.expect_download() as dst_download:
            page.locator("#export").click()
        dst_path = EVIDENCE / "studio-oneil.dst"
        dst_download.value.save_as(dst_path)
        assert hashlib.sha256(dst_path.read_bytes()).hexdigest() == dst_sha(reopened_payload)
        page.locator(".rail-nav [data-view='production']").click()
        page.locator("#verify").click()
        page.wait_for_function(
            "() => document.querySelector('#preview-note').textContent.includes('Read back')",
            timeout=30000,
        )
        page.locator(".rail-nav [data-view='studio']").click()
        kept = page.locator("#stitch-count").inner_text()
        set_text(page, "Alpine")
        with page.expect_response("**/api/generate") as rejected:
            page.locator("#generate").click()
        rejected_body = rejected.value.json()
        assert rejected.value.status == 422, rejected_body
        assert "U+006C" in rejected_body["error"]
        assert "left unchanged" in rejected_body["error"]
        page.wait_for_function(
            """() => document.querySelector('#errors').textContent.includes('U+006C')
              && document.querySelector('#errors').textContent.includes('left unchanged')
              && document.querySelector('#text-content').value === 'Alpine'
              && document.querySelector('#export').disabled
              && document.querySelector('#revision').textContent.includes('not generated')""",
            timeout=5000,
        )
        assert page.locator("#stitch-count").inner_text() == kept
        page.locator("#undo").click()
        page.wait_for_function(
            """() => document.querySelector('#text-content').value === "O'NEIL-08"
              && document.querySelector('#inspector-name').textContent === 'Lettering'""",
            timeout=5000,
        )
        set_text(page, "CAF\u00c9")
        with page.expect_response("**/api/generate") as accented:
            page.locator("#generate").click()
        accented_body = accented.value.json()
        assert accented.value.status == 422, accented_body
        assert "U+00C9" in accented_body["error"]
        assert "left unchanged" in accented_body["error"]
        assert page.locator("#text-content").input_value() == "CAF\u00c9"
        assert page.locator("#stitch-count").inner_text() == kept
        for name, width, height in (
            ("studio-desktop", 1440, 1000),
            ("studio-tablet", 768, 1024),
            ("studio-mobile", 390, 844),
        ):
            page.set_viewport_size({"width": width, "height": height})
            page.locator("#text-content").scroll_into_view_if_needed()
            overflow = page.evaluate(
                "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
            )
            assert overflow, (name, page.evaluate(
                "() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"
            ))
            page.screenshot(path=str(EVIDENCE / f"{name}.png"))
        binary = ROOT / "target" / "debug" / "embroidery-studio"
        manifest = {
            "url": url,
            "studio_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "alpine_08_revision": alpine_revision,
            "alpine_08_dst_sha256": alpine_hash,
            "alpine_09_revision": changed["revision"],
            "alpine_09_dst_sha256": changed_hash,
            "restored_08_matches_08": True,
            "top_sequence_equal_across_underlay": True,
            "oneil_revision": neil["revision"],
            "oneil_dst_sha256": dst_sha(neil),
            "font_id": alpine_text["text"]["font_id"],
            "font_sha256": alpine_text["text"]["font_sha256"],
            "machine_release_allowed": False,
            "lowercase_rejected": "U+006C",
            "accent_rejected": "U+00C9",
        }
        (EVIDENCE / "studio-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print("studio_url", url)
        print("studio_binary", manifest["studio_binary_sha256"])
        print("studio_alpine_08", alpine_hash, alpine_revision)
        print("studio_alpine_09", changed_hash, changed["revision"])
        print("studio_oneil", manifest["oneil_dst_sha256"])
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
