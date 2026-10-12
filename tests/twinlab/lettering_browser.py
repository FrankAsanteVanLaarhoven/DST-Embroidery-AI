"""Browser check for native lettering in TwinLab Compare.

The example design stays on schema 1. Added text is EmbroideryOS Block 01.
ALPINE 08 is compiled, proved, changed to ALPINE 09, and restored with Undo.
The restored DST bytes do not make the previous PDF current. O'NEIL-08 checks
spacing punctuation. A curly apostrophe is rejected and the text stays put.
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


def snap(page):
    return page.evaluate("() => window.__twinlab.snapshot()")


def lettering(snapshot):
    found = [item for item in snapshot["design"]["objects"] if item.get("kind") == "text"]
    assert len(found) == 1, found
    return found[0]


def settle(page, previous):
    page.wait_for_function(
        """(old) => {
          const snap = window.__twinlab.snapshot();
          return snap.dirty === false && snap.artifact_id && snap.artifact_id !== old
            && document.querySelector('#phases').dataset.topSequence
            && document.querySelector('#phases').dataset.letteringText;
        }""",
        arg=previous,
        timeout=60000,
    )
    return snap(page)


def compile_after(page, previous, action):
    with page.expect_response("**/api/compile") as compiled:
        action()
    response = compiled.value
    payload = response.json()
    assert response.ok, payload
    current = settle(page, previous)
    assert current["provenance"]["dst_sha256"] == payload["provenance"]["dst_sha256"]
    return current, payload


def set_text(page, previous, value):
    def action():
        field = page.locator("#text-content")
        field.fill(value)
        field.dispatch_event("change")

    return compile_after(page, previous, action)


def main() -> None:
    url = os.environ.get("TWINLAB_URL", "http://127.0.0.1:4181/")
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
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on(
            "console",
            lambda message: console.append(message.type + ":" + message.text)
            if message.type == "error"
            else None,
        )
        page.goto(url, wait_until="domcontentloaded")
        page.wait_for_function(
            "() => window.__twinlab && window.__twinlab.snapshot().artifact_id && window.__twinlab.snapshot().dirty === false",
            timeout=60000,
        )
        opened = snap(page)
        assert opened["mode"] == "native-rust", opened["mode"]
        assert opened["compare"]["release_allowed"] is False
        assert opened["design"]["schema_version"] == 1
        none, none_payload = compile_after(page, opened["artifact_id"], lambda: page.locator("#add-text").click())
        none_text = lettering(none)
        assert none_text["text"]["content"] == "ALPINE 08"
        assert none_text["points"] == []
        assert none_text["text"]["font_id"] == "embroideryos-block-01"
        assert none_text["text"]["font_sha256"] == none_payload["generation"]["lettering_font"]["sha256"]
        assert none["design"]["schema_version"] == 1
        assert "U+0041" in none_payload["generation"]["lettering_trace"]
        assert "U+0020 space" in none_payload["generation"]["lettering_trace"]
        assert "U+0038" in none_payload["generation"]["lettering_trace"]
        assert "ALPINE 08" in none_payload["generation"]["lettering_text"]
        none_top = none_payload["generation"]["top_sequence"]
        assert page.locator("#centre-run").is_checked() is False
        assert page.locator("#edge-run").is_checked() is False
        centre, centre_payload = compile_after(page, none["artifact_id"], lambda: page.locator("#centre-run").check())
        both, both_payload = compile_after(page, centre["artifact_id"], lambda: page.locator("#edge-run").check())
        edge, edge_payload = compile_after(page, both["artifact_id"], lambda: page.locator("#centre-run").uncheck())
        cleared, cleared_payload = compile_after(page, edge["artifact_id"], lambda: page.locator("#edge-run").uncheck())
        centre_again, _centre_again_payload = compile_after(
            page, cleared["artifact_id"], lambda: page.locator("#centre-run").check()
        )
        both_again, both_again_payload = compile_after(
            page, centre_again["artifact_id"], lambda: page.locator("#edge-run").check()
        )
        tops = [
            item["generation"]["top_sequence"]
            for item in (none_payload, centre_payload, both_payload, edge_payload, cleared_payload, both_again_payload)
        ]
        assert len(set(tops)) == 1, tops
        assert both_again["provenance"]["dst_sha256"] == both["provenance"]["dst_sha256"]
        assert none_top
        alpine = both_again
        alpine_payload = both_again_payload
        alpine_hash = alpine["provenance"]["dst_sha256"]
        alpine_revision = alpine["design"]["revision"]
        page.locator("#tab-original").click()
        page.locator("#attach-reference").click()
        page.wait_for_function(
            "() => window.__twinlab.snapshot().compare.original_sha256",
            timeout=10000,
        )
        page.locator("#source-width").fill("14")
        page.locator("#source-x").fill("-2")
        page.locator("#source-y").fill("-2")
        page.locator("#scale-confirm").check()
        page.locator("#proof-generate").click()
        page.wait_for_function(
            "() => window.__twinlab.snapshot().compare.binding_current",
            timeout=30000,
        )
        bound = snap(page)
        first_proof = bound["compare"]["proof_id"]
        assert bound["compare"]["binding_current"] is True
        assert bound["compare"]["release_allowed"] is False
        assert bound["provenance"]["dst_sha256"] == alpine_hash
        changed, changed_payload = set_text(page, alpine["artifact_id"], "ALPINE 09")
        assert lettering(changed)["text"]["content"] == "ALPINE 09"
        assert changed["design"]["revision"] != alpine_revision
        assert changed["provenance"]["dst_sha256"] != alpine_hash
        assert changed["compare"]["binding_current"] is False
        assert "ALPINE 09" in changed_payload["generation"]["lettering_text"]
        with page.expect_download() as changed_dst:
            page.locator("#download").click()
        changed_path = EVIDENCE / "twin-alpine09.dst"
        changed_dst.value.save_as(changed_path)
        assert hashlib.sha256(changed_path.read_bytes()).hexdigest() == changed["provenance"]["dst_sha256"]
        restored, restored_payload = compile_after(page, changed["artifact_id"], lambda: page.locator("#undo").click())
        assert lettering(restored)["text"]["content"] == "ALPINE 08"
        assert restored["design"]["revision"] != alpine_revision
        assert restored["provenance"]["dst_sha256"] == alpine_hash
        assert restored["compare"]["binding_current"] is False
        assert "Lettering" in page.locator(".object.selected").inner_text()
        assert "ALPINE 08" in restored_payload["generation"]["lettering_text"]
        page.locator("#scale-confirm").check()
        page.locator("#proof-generate").click()
        page.wait_for_function(
            "() => window.__twinlab.snapshot().compare.binding_current",
            timeout=30000,
        )
        rebound = snap(page)
        assert rebound["compare"]["proof_id"] != first_proof
        assert rebound["provenance"]["dst_sha256"] == alpine_hash
        page.locator("#proof-reopen").click()
        page.wait_for_function(
            "() => document.querySelector('#proof-frame') && !document.querySelector('#proof-frame').hidden",
            timeout=10000,
        )
        with page.expect_download() as proof_info:
            page.locator("#proof-download").click()
        proof_path = EVIDENCE / "twin-alpine08-proof.pdf"
        proof_info.value.save_as(proof_path)
        assert hashlib.sha256(proof_path.read_bytes()).hexdigest() == rebound["compare"]["proof_sha256"]
        page.locator("#tab-machine").click()
        assert "Replay uses every record." in page.locator("#machine-window").inner_text()
        with page.expect_download() as dst_info:
            page.locator("#download").click()
        dst_path = EVIDENCE / "twin-alpine08.dst"
        dst_info.value.save_as(dst_path)
        assert hashlib.sha256(dst_path.read_bytes()).hexdigest() == alpine_hash
        with page.expect_download() as replay_info:
            page.locator("#evidence").click()
        replay_path = EVIDENCE / "twin-alpine08-replay.json"
        replay_info.value.save_as(replay_path)
        replay = json.loads(replay_path.read_text())
        assert replay["provenance"]["dst_sha256"] == alpine_hash
        assert replay["trace"]["events"]
        moved, _moved_payload = compile_after(
            page,
            rebound["artifact_id"],
            lambda: (
                page.locator("#text-origin-x").fill("-50"),
                page.locator("#text-origin-x").dispatch_event("change"),
            ),
        )
        assert lettering(moved)["text"]["origin_x_mm"] == -50
        assert moved["compare"]["binding_current"] is False
        assert "Lettering" in page.locator(".object.selected").inner_text()
        undone, _undone_payload = compile_after(page, moved["artifact_id"], lambda: page.locator("#undo").click())
        assert lettering(undone)["text"]["origin_x_mm"] == -52
        assert undone["provenance"]["dst_sha256"] == alpine_hash
        assert undone["compare"]["binding_current"] is False
        assert "Lettering" in page.locator(".object.selected").inner_text()
        page.set_viewport_size({"width": 390, "height": 844})
        page.locator("#text-content").scroll_into_view_if_needed()
        neil, neil_payload = set_text(page, undone["artifact_id"], "O'NEIL-08")
        assert lettering(neil)["text"]["content"] == "O'NEIL-08"
        assert "U+0027" in neil_payload["generation"]["lettering_trace"]
        assert "U+002D" in neil_payload["generation"]["lettering_trace"]
        assert lettering(neil)["text"]["font_sha256"] == none_text["text"]["font_sha256"]
        with page.expect_download() as saved:
            page.locator("#save").click()
        design_path = EVIDENCE / "twin-oneil.json"
        saved.value.save_as(design_path)
        stored = json.loads(design_path.read_text())
        stored_text = next(item for item in stored["objects"] if item["kind"] == "text")
        assert stored["schema_version"] == 1
        assert stored_text["text"]["content"] == "O'NEIL-08"
        assert stored_text["text"]["font_sha256"] == none_text["text"]["font_sha256"]
        assert stored_text["points"] == []
        with page.expect_response("**/api/compile") as reopened_response:
            page.locator("#file-input").set_input_files(design_path)
        assert reopened_response.value.ok
        page.wait_for_function(
            "() => window.__twinlab.snapshot().dirty === false",
            timeout=60000,
        )
        page.locator(".object", has_text="Lettering").click()
        assert page.locator("#text-content").input_value() == "O'NEIL-08"
        assert none_text["text"]["font_sha256"][:12] in page.locator("#font-pin").inner_text()
        page.set_viewport_size({"width": 1440, "height": 1000})
        bad = "O\u2019NEIL"
        field = page.locator("#text-content")
        field.fill(bad)
        field.dispatch_event("change")
        page.wait_for_function(
            """(bad) => {
              const message = document.querySelector('#message').textContent;
              const snap = window.__twinlab.snapshot();
              const text = snap.design.objects.find((item) => item.kind === 'text');
              return message.includes('U+2019') && message.includes('left unchanged')
                && text.text.content === bad && snap.dirty === true
                && document.querySelector('#download').disabled;
            }""",
            arg=bad,
            timeout=30000,
        )
        assert lettering(snap(page))["text"]["content"] == bad
        for name, width, height in (
            ("twin-desktop", 1440, 1000),
            ("twin-tablet", 768, 1024),
            ("twin-mobile", 390, 844),
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
        binary = ROOT / "target" / "debug" / "embroidery-twin"
        manifest = {
            "url": url,
            "twin_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "engine_sha256": rebound["provenance"].get("engine_sha256"),
            "schema_version": 1,
            "alpine_08_revision": alpine_revision,
            "alpine_08_dst_sha256": alpine_hash,
            "alpine_08_artifact_id": rebound["artifact_id"],
            "alpine_09_revision": changed["design"]["revision"],
            "alpine_09_dst_sha256": changed["provenance"]["dst_sha256"],
            "restored_dst_matches_08": True,
            "restored_revision": restored["design"]["revision"],
            "first_proof_not_current_after_undo": True,
            "rebound_proof_id": rebound["compare"]["proof_id"],
            "rebound_proof_sha256": rebound["compare"]["proof_sha256"],
            "top_sequence_equal_across_underlay": True,
            "font_sha256": none_text["text"]["font_sha256"],
            "machine_release_allowed": False,
            "curly_apostrophe_rejected": "U+2019",
        }
        (EVIDENCE / "twin-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print("twin_url", url)
        print("twin_binary", manifest["twin_binary_sha256"])
        print("twin_engine", manifest["engine_sha256"])
        print("twin_alpine_08", alpine_hash, alpine_revision)
        print("twin_alpine_09", manifest["alpine_09_dst_sha256"], manifest["alpine_09_revision"])
        print("twin_proof", manifest["rebound_proof_sha256"])
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "422" not in item and "404" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
