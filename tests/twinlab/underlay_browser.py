"""Browser check for selectable satin underlay in TwinLab Compare.

The historical Shield border stays a both-pass underlay_mm design until a
control changes. New satin is checked as constant width, then as a taper.
The downloaded DST, decoded replay, and reopened PDF belong to one snapshot.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "evidence" / "satin-underlay"
BROWSER = os.environ.get(
    "BROWSER_EXECUTABLE",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
)


def snap(page):
    return page.evaluate("() => window.__twinlab.snapshot()")


def settle(page, previous):
    page.wait_for_function(
        """(old) => {
          const snap = window.__twinlab.snapshot();
          return snap.dirty === false && snap.artifact_id && snap.artifact_id !== old
            && document.querySelector('#phases').dataset.topSequence;
        }""",
        arg=previous,
        timeout=60000,
    )
    return snap(page)


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
        revision = opened["design"]["revision"]
        page.locator(".object", has_text="Shield border").click()
        assert snap(page)["design"]["revision"] == revision
        shield = next(item for item in snap(page)["design"]["objects"] if item["label"] == "Shield border")
        assert "underlay" not in shield
        assert "centre_run_mm" not in shield
        assert shield["underlay_mm"] == 2.5
        assert page.locator("#centre-run").is_checked()
        assert page.locator("#edge-run").is_checked()
        page.locator(".object", has_text="Mountain field").click()
        assert page.locator("#underlay").count() == 1
        assert page.locator("#centre-run").count() == 0
        page.locator("#add-satin").click()
        page.wait_for_function(
            """() => {
              const snap = window.__twinlab.snapshot();
              return snap.dirty === false && document.querySelector('#centre-run')
                && !document.querySelector('#centre-run').checked
                && !document.querySelector('#edge-run').checked;
            }""",
            timeout=60000,
        )
        none = snap(page)
        none_top = page.locator("#phases").get_attribute("data-top-sequence")
        assert "Edge-run underlay 0" in page.locator("#phases").inner_text() or "Centre-run underlay" in page.locator("#phases").inner_text()
        before = none["design"]["revision"]
        page.locator("#centre-run").check()
        assert snap(page)["design"]["revision"] == before + 1
        page.locator("#centre-run-length").wait_for()
        page.locator("#centre-run-length").dispatch_event("change")
        page.wait_for_timeout(200)
        assert snap(page)["design"]["revision"] == before + 1
        centre = settle(page, none["artifact_id"])
        centre_top = page.locator("#phases").get_attribute("data-top-sequence")
        centre_text = page.locator("#phases").inner_text()
        assert "Centre-run underlay 0" not in centre_text
        assert "Edge-run underlay 0" in centre_text
        assert "Top satin" in centre_text
        assert centre["provenance"]["dst_sha256"] != none["provenance"]["dst_sha256"]
        page.locator("#edge-run").check()
        both = settle(page, centre["artifact_id"])
        both_top = page.locator("#phases").get_attribute("data-top-sequence")
        page.locator("#centre-run").uncheck()
        edge = settle(page, both["artifact_id"])
        edge_top = page.locator("#phases").get_attribute("data-top-sequence")
        assert none_top and len({none_top, centre_top, both_top, edge_top}) == 1
        assert "Centre-run underlay 0" in page.locator("#phases").inner_text()
        with page.expect_response("**/api/compile") as repeated:
            page.locator("#compile").click()
        repeated_payload = repeated.value.json()
        assert repeated.value.ok, repeated_payload
        assert repeated_payload["provenance"]["dst_sha256"] == edge["provenance"]["dst_sha256"]
        with page.expect_download() as saved:
            page.locator("#save").click()
        design_path = EVIDENCE / "twin-constant-edge.json"
        saved.value.save_as(design_path)
        stored = json.loads(design_path.read_text())
        added = next(item for item in stored["objects"] if item["label"] == "New satin")
        assert added["underlay"] == ["edge_run"]
        assert "centre_run_mm" not in added
        with page.expect_response("**/api/compile") as reopened:
            page.locator("#file-input").set_input_files(design_path)
        assert reopened.value.ok
        page.wait_for_function(
            "() => window.__twinlab.snapshot().dirty === false",
            timeout=60000,
        )
        page.locator(".object", has_text="New satin").click()
        assert page.locator("#edge-run").is_checked()
        assert page.locator("#centre-run").is_checked() is False
        page.locator(".object.selected").wait_for()
        assert "New satin" in page.locator(".object.selected").inner_text()
        with page.expect_download() as dst_info:
            page.locator("#download").click()
        constant_path = EVIDENCE / "twin-constant-edge.dst"
        dst_info.value.save_as(constant_path)
        constant = snap(page)
        assert hashlib.sha256(constant_path.read_bytes()).hexdigest() == constant["provenance"]["dst_sha256"]
        page.locator("#tab-machine").click()
        assert "Replay uses every record." in page.locator("#machine-window").inner_text()
        assert "raw" in page.locator("#machine-window").inner_text()
        page.locator("#tab-twin").click()
        page.locator("#scrub").evaluate(
            "(el)=>{el.value=0;el.dispatchEvent(new Event('input',{bubbles:true}));}"
        )
        page.wait_for_timeout(80)
        assert page.locator("#design-canvas").get_attribute("data-event-index") == "0"
        assert page.locator("#machine-canvas").get_attribute("data-event-index") == "0"
        assert "record 1 /" in page.locator("#twin-event").inner_text()
        page.locator("#column-widths").fill("2\n8")
        page.locator("#column-widths").dispatch_event("change")
        tapered_edge = settle(page, constant["artifact_id"])
        assert "variable satin" in page.locator("#phases").inner_text()
        tapered_edge_top = page.locator("#phases").get_attribute("data-top-sequence")
        assert tapered_edge_top != edge_top
        page.locator("#centre-run").check()
        page.locator("#centre-run-length").fill("4")
        page.locator("#centre-run-length").dispatch_event("change")
        mixed = settle(page, tapered_edge["artifact_id"])
        page.locator("#edge-run-length").fill("2.5")
        page.locator("#edge-run-length").dispatch_event("change")
        mixed_lengths = settle(page, mixed["artifact_id"])
        assert page.locator("#phases").get_attribute("data-top-sequence") == tapered_edge_top
        assert "Centre-run underlay 0" not in page.locator("#phases").inner_text()
        assert "Edge-run underlay 0" not in page.locator("#phases").inner_text()
        satin = next(item for item in mixed_lengths["design"]["objects"] if item["label"] == "New satin")
        assert satin["underlay"] == ["centre_run", "edge_run"]
        assert satin["widths_mm"] == [2, 8]
        assert satin["edge_run_mm"] == 2.5
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
        assert bound["compare"]["binding_current"] is True
        assert bound["compare"]["release_allowed"] is False
        assert bound["provenance"]["dst_sha256"] == mixed_lengths["provenance"]["dst_sha256"]
        page.locator("#proof-reopen").click()
        page.wait_for_function("() => document.querySelector('#proof-frame') && !document.querySelector('#proof-frame').hidden", timeout=10000)
        with page.expect_download() as proof_info:
            page.locator("#proof-download").click()
        proof_path = EVIDENCE / "twin-taper-proof.pdf"
        proof_info.value.save_as(proof_path)
        assert hashlib.sha256(proof_path.read_bytes()).hexdigest() == bound["compare"]["proof_sha256"]
        with page.expect_download() as taper_dst:
            page.locator("#download").click()
        taper_path = EVIDENCE / "twin-taper.dst"
        taper_dst.value.save_as(taper_path)
        assert hashlib.sha256(taper_path.read_bytes()).hexdigest() == bound["provenance"]["dst_sha256"]
        page.locator("#edge-run").uncheck()
        page.wait_for_function(
            """() => {
              const snap = window.__twinlab.snapshot();
              return snap.dirty === true && snap.compare.binding_current === false
                && document.querySelector('#download').disabled
                && document.querySelector('#proof-download').disabled;
            }""",
            timeout=5000,
        )
        stale = snap(page)
        assert stale["design"]["revision"] == bound["design"]["revision"] + 1
        assert "New satin" in page.locator(".object.selected").inner_text()
        centre_only = settle(page, bound["artifact_id"])
        assert centre_only["compare"]["binding_current"] is False
        page.locator("#undo").click()
        page.wait_for_function(
            """() => document.querySelector('#edge-run').checked
              && document.querySelector('#centre-run').checked
              && document.querySelector('.object.selected').textContent.includes('New satin')""",
            timeout=5000,
        )
        restored = settle(page, centre_only["artifact_id"])
        assert restored["provenance"]["dst_sha256"] == bound["provenance"]["dst_sha256"]
        assert restored["compare"]["binding_current"] is False
        page.locator("#redo").click()
        page.wait_for_function(
            """() => !document.querySelector('#edge-run').checked
              && document.querySelector('.object.selected').textContent.includes('New satin')""",
            timeout=5000,
        )
        for name, width, height in (
            ("twin-desktop", 1440, 1000),
            ("twin-tablet", 768, 1024),
            ("twin-mobile", 390, 844),
        ):
            page.set_viewport_size({"width": width, "height": height})
            page.locator("#centre-run").scroll_into_view_if_needed()
            overflow = page.evaluate(
                "() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1"
            )
            assert overflow, (name, page.evaluate(
                "() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]"
            ))
            page.screenshot(path=str(EVIDENCE / f"{name}.png"))
        manifest = {
            "url": url,
            "mode": restored["mode"],
            "engine_sha256": bound["provenance"]["engine_sha256"],
            "constant_edge_dst_sha256": constant["provenance"]["dst_sha256"],
            "taper_design_sha256": bound["provenance"]["design_sha256"],
            "taper_dst_sha256": bound["provenance"]["dst_sha256"],
            "taper_proof_sha256": bound["compare"]["proof_sha256"],
            "taper_artifact_id": bound["artifact_id"],
            "machine_release_allowed": False,
            "top_sequence_constant_modes_equal": True,
            "top_sequence_taper_modes_equal": True,
        }
        (EVIDENCE / "twin-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print("twin_url", url)
        print("twin_engine", bound["provenance"]["engine_sha256"])
        print("twin_constant_edge_dst", constant["provenance"]["dst_sha256"])
        print("twin_taper_dst", bound["provenance"]["dst_sha256"])
        print("twin_taper_design", bound["provenance"]["design_sha256"])
        print("twin_proof", bound["compare"]["proof_sha256"])
        print("twin_artifact", bound["artifact_id"])
        browser.close()
    assert not errors, errors
    unexpected = [item for item in console if "404" not in item and "422" not in item]
    assert not unexpected, unexpected


if __name__ == "__main__":
    main()
