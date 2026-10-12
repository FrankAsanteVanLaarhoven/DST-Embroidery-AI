# SVG geometry intake

11 October 2026. Base commit `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. The converter, studio route, and browser checks are local working-tree changes on `svg-geometry-intake`. They are not in that commit. This note does not accept A08, G02, or G03, and it does not claim commercial parity.

## What the path does

`python/embroidery_ai/svg_geometry.py` reads a bounded SVG subset and writes a draft native `embroidery-project` version 1 document. The CLI `tools/import_svg_geometry.py` writes three files into a new directory: the original `source.svg`, `project.json`, and `import-manifest.json`. It does not call a model, generate stitches, compile DST, edit the capability workbook, or authorise a machine.

The studio route `POST /api/import-svg` uses that converter, then `parse_project`. The response carries the project, the manifest, and the losses. It does not carry stitches or DST. The browser shows the original source SHA-256, the physical hoop, the converted objects, the source colours, and the losses. Apply is a separate edit. Cancel and a rejected file leave the open design unchanged. If the design changes after the preview, Apply refuses and changes nothing. Apply invalidates the previous export through the existing revision check. Generate still goes through the Rust engine.

Coordinates are `native-hoop-nonnegative-v1`. The manifest records `not_interchangeable_with: twinlab-generic-flat-120-v1` and `timing_model: null`. The studio rehearsal that follows Generate uses the project hoop and the studio TwinLab timing note (600 stitches per minute, 200 mm/s jumps, 2 s colour changes, 0.4 s trims). That is not the Compare `generic-flat-120-v1` frame and not the studio’s 650 stitches-per-minute display estimate.

## What was run on this machine

Python 3.11.4. `.venv/bin/python -m unittest discover -s tests/svg_geometry -v` — 48 test methods passed. `cargo build -p embroidery-ffi --offline` and `tools/check_svg_geometry_native.py` — both fixtures generated and an independent reader accepted the DST. `machine_release_allowed` stayed false.

| Fixture | Source SHA-256 | Revision prefix | Sewing | DST SHA-256 |
|---|---|---|---|---|
| `badge-hole.svg` | `166a2fd6b55fa61fd4324a9e7b4a30dd5c2dd4fa2f4a236364d8916419211c4c` | `665f912eb3ce` | 1380 | `62e97ba0a9548c8e8ace1504d5f3d41c69929829d175b1e0f214f20f6a859060` |
| `cubic-leaf.svg` | `8054cf4cd14d48622432c0fe60fdc5b88990d9956d4bcc5ebad2ac9f519d5023` | `cd291861c5b6` | 564 | `82881461a7443fb174acb30a2488699563b6e2519433813a921bfe5ccb55c9b7` |

The native smoke reopened each `project.json` before generation. Evidence: `evidence/svg-native-gate.json`. Those two fixtures do not replace the repository regression suites.

`cargo test -p embroidery-studio --offline` — 9 passed, including a rectangle preview and a `<text>` rejection. `cargo clippy -p embroidery-studio --offline --all-targets -- -D warnings` passed. `node --check web/studio.js` passed. `web/studio.js` is 63415 bytes and `web/studio.css` is 12511 bytes. The local budget remains 80 KB of script and 40 KB of CSS.

Chrome at `http://127.0.0.1:4176/` rejected a text SVG, cancelled a badge preview, refused a stale Apply after a row-spacing edit, then undid that edit back to Alpine Club (4701 needle-down stitches, 7 objects). A later Apply of `badge-hole.svg` produced one editable fill with a hole. Generate returned 1380 needle-down stitches and revision `665f912eb3ce`. Production → Rehearse in TwinLab opened `/twinlab` on the same studio. The rehearsal facts showed hoop 100 × 100 mm, 1380 needle-down stitches, and DST SHA-256 `62e97ba0a9548c8e8ace1504d5f3d41c69929829d175b1e0f214f20f6a859060`. Qualification stayed the virtual-demo sentence. The file was not sent to the Compare service on port 4175. Screenshots: `evidence/svg-import/`.

`tools/audit_capabilities.py` exited 0: structural audit PASS, 210 records, 6 conflicts. The workbook SHA-256 remains `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128`. Both inherited coverage JSON files remain SHA-256 `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6`. No acceptance cell changed.

## What this does not establish

A08 still needs native editing of the source curves, browser evidence was recorded here for this route only, and the register row stays `NOT_RUN`. G02 is not accepted by this restricted topology parser. G03 is not accepted because the source colours live in the manifest and the native project still stores `color_index` only. G01 and G04 still govern evidence and acceptance.

The supplied zip and patch were not in this checkout. Nothing was applied with `git apply`. The other environment’s 50-test log and blocked native gate are not results for this tree. No physical stitch-out was run. Machine release stays false. Variable-width satin, lettering, and further machine-format adapters remain later work.
