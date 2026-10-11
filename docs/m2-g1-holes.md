# M2-G1 — one outer ring and explicit holes

Local increment on commit `4ea98f9ad29c18351a9b6b47257fc654f2acce94`. The working tree is uncommitted. This is not a machine release, a Boolean-geometry claim, or acceptance of a register row.

Rust remains the only stitch and DST authority. The browser edits rings and displays the decoded export. `machine_release_allowed` stays false.

## Supported topology

One simple, non-self-intersecting outer ring, plus zero or more simple holes. Each hole lies strictly inside the outer ring. Holes do not touch the outer ring, touch one another, overlap, or nest. A repeated closing point is dropped. Perpendicular underlay, when enabled, uses the same region.

Needle-down fill and underlay may end on the permitted boundary. An open segment must not enter a hole or leave the outer ring. Samples within 0.0001 mm of a boundary count as on that boundary. Admission treats 0.000001 mm as on a ring. Travel across a hole, including a row that lies on a hole edge, is a jump.

An empty hole list calls the existing `generate_fill` path. Projects that omit `holes` stay readable. Format version stays `"1"`, and the TwinLab schema version stays 1. Empty hole lists are omitted from canonical JSON, so a no-hole revision does not change. The locked running-project revision remains `daaf1d0fed513d354da2cc305c3eafeffeeb7a6028f379b5e18074d128542580`.

Core limits: 32 holes, 4,096 points on one ring, 8,192 points in the region, and 200,000 commands in one holed fill. TwinLab’s editor is tighter: holes only on a fill, at most 8 holes, 3–64 points each, 256 points together, and coordinates in [−60, 60]. Rejection names the problem and does not commit a partial plan.

## Register rows this does not accept

These rows were identified before the change. Their acceptance cells stay `NOT_RUN`. The workbook was not edited.

| ID | Feature | Status in the register | What this increment is |
|---|---|---|---|
| ED21 | Hole Sewing Setting | PLANNED | One fill with explicit holes. Not the whole editing setting. |
| S02 | Fill | LIBRARY_SUBSET | The existing scanline fill, extended to this topology. Not every fill strategy. |
| G02 | Robust geometry and explicit units | PLANNED | This topology only. Not Boolean clipping or offsets. |
| Q10 | Underlay and pull compensation (continuity) | LIBRARY_SUBSET | Perpendicular underlay jumps the same holes. Not a full underlay catalogue. |

ED02, ED17, S03–S14, Q01, and the other fill families were not exercised as acceptance. Source conflicts CF01–CF06 stay unresolved.

Workbook SHA-256 `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54,782 bytes). Inherited snapshot and `contracts/compatibility-coverage.json` both remain `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32,594 bytes).

## Source identity

`HEAD` alone does not name this implementation. The tests below ran on the uncommitted tree.

- Commit: `4ea98f9ad29c18351a9b6b47257fc654f2acce94`
- Engine binaries after the workspace test, SHA-256: `embroidery-twin` `0be474b5c0dcd5baa85edf6982a1ac50df4e7c8bc711793f795d99bf41439069`, `embroidery-studio` `c7918dda587fd989bddbf25e0b7b229d0e5323d292a276f0fd7893244846f61a`
- Tracked diff of `README.md`, `apps/twinlab/public`, `crates/core`, `crates/twin`, `docs/engine.md`, and `web`, taken before this file and the M2-G1 quality-gate appendix: SHA-256 `59fdc60b4a26f59a94a2cce82d83d5ba65323629c6a08fca8aa56ae50d5a5c5d`. That README diff also contains the earlier uncommitted baseline 0.3 section.
- The studio on port 4173 was already running and was not restarted. It is the older binary. The hole studio check used a second process on port 4176. TwinLab on port 4175 was the process that served the browser export below.

## Commands and results

Run from `/Users/favl/embroidery-engine` on 11 October 2026. rustc 1.90.0. System Chrome. Python 3.11.4 in `.venv`.

- `cargo fmt --all -- --check` — exit 0
- `cargo clippy --workspace --all-targets --offline -- -D warnings` — exit 0
- `cargo test --workspace --offline` — exit 0. embroidery-core 28, embroidery-dst 16 plus fill-hole lock 2 plus pyembroidery oracle 5, embroidery-ffi 4, embroidery-motion 10, embroidery-simulate 3, embroidery-studio workflow 5, embroidery-twin 18.
- `cargo test -p embroidery-dst --test fill_holes --offline -- --nocapture` — no-hole DST SHA-256 `5d74efde4d1d1d74210bab965640c72291136761a9fc5d14f867182f0865a171`, 560 bytes. Holed square SHA-256 `e04210e8f220fbf61e22d232ab26ae3b66014f3bbb6c50e0c792ff47f4c610c7`, 569 bytes, repeated and different from the solid square.
- `node --test tests/twinlab/unit.mjs` — 57 passed, 0 failed. This does not execute the hole browser.
- TwinLab browser: `BROWSER_EXECUTABLE="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" TWINLAB_URL=http://127.0.0.1:4175/ HOLE_OUTPUT=evidence/m2-g1 .venv/bin/python tests/twinlab/hole_browser.py` — passed. Edited DST SHA-256 `65fcab89dfa0167b9d4cfbad18c040a3fa25bf8cce6150d712beb1ffca7ad0e3`. PDF SHA-256 `8fc6c6b9f4d5479da62899b2b93376b61fe7208f7d7d36531ee955ae7ef4e081`.
- Studio browser: `STUDIO_URL=http://127.0.0.1:4176/ STUDIO_HOLE_OUTPUT=evidence/m2-g1` with the same Chrome, `tests/studio/hole_browser.py` — passed. Club-mark hole DST SHA-256 `cb74dd0aaf96dc97c4296a2e34602d47d8ef72af67d94bcbb1f7c2325c848737`, 851 bytes. `machine_release_allowed` was false.

`scripts/check-twinlab-m1.sh` was not run for this milestone. The earlier Compare Alpine spacing result, DST `c895401797b8ce29c217a75b5d808e530505594422ed70fbea4dad4b0269998e`, stays a historical Compare result. It is not this hole export.

## What the browser runs did

TwinLab at `http://127.0.0.1:4175/` loaded Alpine mark with no `holes` field, then opened `fixtures/fill-holes/designs/twin-square-hole.json`. Save and reopen kept the hole and the same DST hash. Moving the hole marked the export and PDF not current, then Rust compiled a new DST. The downloaded DST matched the replay hash. The proof binding kept release and approval false. Undo restored the hole. A hole point placed on the outer boundary was rejected, left the artifact unset, and undo recovered a clean compile. Viewports were 1440×1000, 768×1024, and 390×844. Screenshots: `evidence/m2-g1/desktop.png`, `tablet.png`, `mobile.png`.

Studio at `http://127.0.0.1:4176/` loaded Club mark, generated the no-hole sample, then added one hole on the Block fill. Export was disabled until Rust regenerated. The plan contains jumps across the hole and no needle-down sample in the hole interior. Undo cleared the hole, redo restored it, and Right moved the hole with the outline. A hole that touched the outer edge was rejected with: “Hole 1 is not strictly inside the outer boundary. Move it fully inside, leaving a gap, and generate again. The plan was not changed.” The revision stayed “Edits are not generated yet.” Screenshots: `evidence/m2-g1/studio-desktop.png`, `studio-tablet.png`, `studio-mobile.png`.

## Fixtures

Nineteen geometry files in `fixtures/fill-holes/`. Twelve sew: `square-one-hole`, `two-holes`, `three-holes`, `concave-outer-with-hole`, `narrow-gap-0.4`, `clearance-0.5`, `underlay-avoids-hole`, `rotated-30`, `clockwise-hole`, `closed-duplicate-endpoint`, `boundary-gap-0.05`, `alpine-centre-hole`. Seven reject: self-intersecting outer, self-intersecting hole, hole outside, hole touching the outer ring, overlapping holes, nested holes, and touching holes. `designs/twin-square-hole.json` is the TwinLab document and is not part of the geometry loader.

## Corrections during the work

- A row exactly on a hole edge was sewn along that edge. Hole edges parallel to the row are now crossings, so that span is a jump. The sewing check also requires an interior crossing to sit inside the open segment.
- A limit test used identical points, which collapsed before the point-count check. It now uses distinct collinear points.
- The first TwinLab browser assertion treated the intentional 422 as an unexplained console error. That 422 is the rejected compile.
- The first studio browser assertion expected a single outline polygon. Club mark also draws the satin stripe, so the check compares the region count with the count before the hole.

## Remaining limits

General clipping, offsets, medial-axis extraction, nested or touching holes, decorative fills, and production stitch quality are not implemented. A topology test does not calibrate a machine. Physical stitch-out, Safari, Firefox, a screen reader, and `wasm32-unknown-unknown` were not run. The studio process on port 4173 does not contain this generator.
