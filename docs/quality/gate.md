# Quality gate

Delivery/build or commit: local workspace, not committed, not published. Crate version 0.1.0.
Environment and URL (no secrets): macOS, rustc 1.90.0 (Homebrew), Python 3.11.4, pydantic 2.14.0, pytest 9.1.1, pyembroidery 1.5.1 in `.venv`. No URL. The library was not served.
Scope and classification: non-UI local library. Rust stitch kernel, DST compiler, C ABI, and a Python proposal package. Not a public release, preview site, or native app.
Viewports and browser/device: none. There is no rendered interface.
Commands and evidence paths:

- `cargo fmt --all -- --check` — exit 0
- `cargo clippy --workspace --all-targets --offline -- -D warnings` — exit 0
- `cargo test --workspace --offline` — exit 0. embroidery-core 18 passed, embroidery-dst 16 passed, pyembroidery oracle 5 passed, embroidery-ffi 3 passed, embroidery-simulate 3 passed, doc-tests 0
- `.venv/bin/python -m pytest python/tests -q` — 15 passed in 0.10s
- `rustup target list --installed` — `aarch64-apple-darwin` only

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | No website and no release host. No domain was invented or configured. |
| Q02 Page titles | N/A | No pages or app views. |
| Q03 Meta descriptions | N/A | No public indexable pages. |
| Q04 Favicon and icons | N/A | No web or app icon surface. |
| Q05 Open Graph image | N/A | Nothing is shared as a web page. |
| Q06 Canonical URLs | N/A | No indexable origin. |
| Q07 Social previews | N/A | No social card. |
| Q08 Custom 404 | N/A | No document routes. Library errors stay on the function result. |
| Q09 Loading states | N/A | Compile, validate, and score are synchronous. There is no pending UI operation. |
| Q10 Error messages | PASS | Rejected plans name the failure and the next step. Empty plan: "Generate a running, satin, or fill object before compiling." Bad JSON: "Could not read the stitch plan JSON" and tells the caller to use the generators. Truncated DST: "export it again." A proposal passed to `compile_dst` is refused with "Run the Rust generators". Covered by `validate::tests`, `read::tests::truncated_record_names_the_next_step`, `c_api_compiles_and_rejects_bad_json`, `test_validate_empty_plan_names_the_next_step`, and `test_proposal_is_not_compiled`. |
| Q11 Heading structure | N/A | No rendered document view. |
| Q12 Alternative text | N/A | No product images. `fixtures/stitchouts/` has no photos. |
| Q13 Sitemap and indexing | N/A | Private local library. No sitemap was added. |
| Q14 Console errors | N/A | No browser session. The exercised paths are `cargo test` and pytest, both exit 0. |
| Q15 Debug output | PASS | No `println!`, `eprintln!`, `dbg!`, or `console.log` in the crates or the Python package. The only `print` is the pyembroidery oracle's stdout protocol inside `crates/dst/tests/pyembroidery_oracle.rs`, which the test reads. It is not shipped library code. |
| Q16 Source maps | N/A | No release artifact was built or published. The `wasm` feature was not compiled, so no WASM source map exists. |
| Q17 JavaScript and performance | N/A | No JavaScript bundle. WASM was not built, so there is no bundle to measure. |
| Q18 Mobile layout | N/A | No layout. |
| Q19 Consistent spacing | N/A | No UI. The Rust code uses the existing types and millimetre units. |
| Q20 Working controls and links | N/A | No buttons or links. The C ABI and proposal schema are what the tests exercise. |

Known gaps and next action:

- `wasm32-unknown-unknown` is not installed, so `compileDst` / `validatePlan` / `scorePlan` were not built. Install the target before treating the WASM build as checked.
- Polygon clipping, holes, medial axis, variable-width satin, PES/EXP/JEF/VP3, and a fabric simulator are not implemented. See `docs/engine.md`.
- The 500-design benchmark has not been run. No physical stitch-out exists. See `docs/benchmarks.md`.

Release-ready claim: no. This is a local foundation. The applicable library checks that were run passed. The web checks do not apply. The WASM build and every production-quality claim remain open.

## Studio review, 11 October 2026

Delivery/build or commit: local workspace on top of commit `98303601416a3b0d68eae37e939031732bd77c45`. The studio changes are not committed and were not pushed.
Environment and URL: macOS, rustc 1.90.0 (Homebrew), Python 3.11.4, Chrome headless. Studio at `http://127.0.0.1:4173`, bound to loopback. `rustup target list --installed` reports `aarch64-apple-darwin` only.
Scope and classification: local development UI plus the Rust generation bridge. One workflow: open the native club-mark project, edit an object, generate in Rust, validate, export DST, and read the file back. Not a public release.
Viewports and browser: Chrome headless at 390×844, 768×1024, and 1280×900. Safari, Firefox, and a screen reader were not used.
Commands and evidence paths:

- `cargo fmt --all` then `cargo clippy --workspace --all-targets --offline -- -D warnings` then `cargo test --workspace --offline` — exit 0. embroidery-core 23 passed, embroidery-dst 16 passed, pyembroidery oracle 5 passed, embroidery-ffi 4 passed, embroidery-simulate 3 passed, embroidery-studio workflow 4 passed.
- `cargo build -p embroidery-ffi --offline` then `.venv/bin/python -m pytest python/tests -q` — 19 passed. An earlier pytest run failed with a missing `embroidery_generate_project_json` symbol because it loaded a stale cdylib. The rerun after the build passed.
- Browser session recorded in `docs/quality/evidence/browser-report.json`, with screenshots `layout-390.png`, `layout-390-actions.png`, `layout-768.png`, `layout-1280.png`, `desktop-reviewed.png`, `desktop-error.png`, and `not-found.png`.
- Asset sizes: `web/studio.js` 21067 bytes, `web/studio.css` 5471 bytes, `web/index.html` 4596 bytes, `web/favicon.svg` 264 bytes.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | Local loopback studio. No public host and no domain was configured. |
| Q02 Page titles | PASS | Studio title `EmbroideryOS — Design studio`. Unknown document title `Page not found — EmbroideryOS`. One h1 on each page. |
| Q03 Meta descriptions | PASS | Both documents use a local, non-sensitive description and `noindex`. The description does not include a project name or reviewer. |
| Q04 Favicon and icons | PASS | `GET /favicon.svg` returned 200 `image/svg+xml`. The mark is the original zigzag in `web/favicon.svg`. No home-screen install icon is shipped. |
| Q05 Open Graph image | N/A | The studio is not a public shareable page. No production origin exists to host a card. |
| Q06 Canonical URLs | N/A | No indexable production origin. A canonical URL was not invented. |
| Q07 Social previews | N/A | No public social card. The page is `noindex` on loopback. |
| Q08 Custom 404 | PASS | `GET /missing-studio-page` returned 404 and the branded page. The recovery link navigated to `/` and the studio title. `POST /api/nope` returned 404 JSON: `That API route does not exist. Use the design studio at /.`. |
| Q09 Loading states | PASS | With fetch delayed 400 ms, Generate stitches showed `Generating stitches…`, `aria-busy=true`, and disabled. It then returned to `Generate stitches`, readiness `Geometry checked`, and 106 needle-down stitches. The client aborts after 30 seconds. |
| Q10 Error messages | PASS | An empty reviewer shows `Enter a reviewer name, then record the geometry review.` A 20 mm satin column shows the ±121 / 12.1 mm limit and says the plan was not changed. Undo clears that error and restores `Geometry checked`. A failed sample load tells the user to run `cargo run -p embroidery-studio`. |
| Q11 Heading structure | PASS | The studio has one h1, `Design studio`, then h2 regions for Objects, Preview, Inspector, Checks, and Length density. The 404 page has one h1, `Page not found`. |
| Q12 Alternative text | PASS | The preview SVG has a title and a description that changes when a plan exists. Controls have visible text. The favicon is a browser icon. |
| Q13 Sitemap and indexing | PASS | `meta robots` is `noindex`. `GET /sitemap.xml` and `GET /robots.txt` return the branded 404. No sitemap file is shipped. |
| Q14 Console errors | PASS | Chrome recorded two network logs and no uncaught exceptions. `422` is the rejected wide-satin generate, and the page shows that error. `404` is the unknown-page check. CSS, script, and favicon requests returned 200. |
| Q15 Debug output | PASS | `web/studio.js` has no `console.log` or `console.debug`. The studio workflow test checks those strings. The server prints its bind address and does not print project bodies. |
| Q16 Source maps | PASS | `web/` has no `.map` files and the script has no `sourceMappingURL`. Nothing was published. |
| Q17 JavaScript and performance | PASS | Measured `studio.js` at 21067 bytes and `studio.css` at 5471 bytes. No framework or dependency bundle. Local budget used for this page: 80 KB script and 40 KB CSS. |
| Q18 Mobile layout | PASS | At 390×844, `scrollWidth` equals `clientWidth` (390). The canvas is first. After scrolling, Generate, review, export, and verify are visible, at least 44 px tall, and inside the viewport. At 768×1024 the same stacked workflow fits without horizontal overflow (`scrollWidth` 753 equals `clientWidth`; `innerWidth` 768 includes the scrollbar). At 1280×900 the three columns fit and Generate is inside the viewport. |
| Q19 Consistent spacing | PASS | Spacing uses the 8 px step, 12 px radius, and the existing ink, blue, and surface colours. Desktop panels scroll inside the viewport. Dense measurements sit on opaque surfaces. Translucency is limited to the top bar when opaque controls are turned off. |
| Q20 Working controls and links | PASS | Chrome completed generate, row-spacing edit, undo of a rejected width, geometry review, review invalidation after another edit, DST verify, and the 404 recovery link. ArrowRight dirtied the plan. Engineering export stays disabled while edits are ungenerated. Machine release stayed unavailable. |

Known gaps and next action:

- `wasm32-unknown-unknown` is not installed. Native and WASM DST bytes were not compared. That agreement gate stays blocked.
- No physical stitch-out and no 500-design benchmark. See `docs/benchmarks.md`.
- The geometry review is a local label bound to a revision hash. It is not an authenticated release, and `machine_release_allowed` stays false.
- Safari, Firefox, and a screen reader were not exercised. This record is not a WCAG audit.
- The studio imports and exports one native project. It does not restore a full operational workspace of approvals, jobs, and inventory.

Release-ready claim: no. The local studio workflow and the Rust generation bridge passed the checks above. Machine release, WASM agreement, and physical qualification remain open.

## TwinLab review, 11 October 2026

Delivery/build or commit: local workspace on top of commit `98303601416a3b0d68eae37e939031732bd77c45`. The studio and TwinLab changes are not committed and were not pushed.
Environment and URL: macOS, Chrome headless 146.0.7680.155 with SwiftShader/ANGLE. TwinLab at `http://127.0.0.1:4173/twinlab`, bound to loopback. `wasm32-unknown-unknown` is not installed.
Scope and classification: local development UI. One demonstration: the club-mark project, Rust generation, exact DST compile, decoded execution trace, and linked 2D/3D playback on the generic fixed-needle XY-frame rig. The active mode is a kinematic demo. Calibrated prediction and a connected twin stay unavailable. Not a public release.
Viewports and browser: Chrome headless at 390×844, 768×1024, and 1280×900. Safari, Firefox, and a screen reader were not used.
Commands and evidence paths:

- `cargo clippy --workspace --all-targets --offline -- -D warnings` — exit 0.
- `cargo test --workspace --offline` — exit 0. embroidery-core 23 passed, embroidery-dst 16 passed, pyembroidery oracle 5 passed, embroidery-ffi 4 passed, embroidery-motion 10 passed, embroidery-simulate 3 passed, embroidery-studio workflow 5 passed. Doc-tests 0.
- `cargo test -p embroidery-studio --offline --test workflow` reads the current `web/twinlab.js` and passed, including `twinlab_replays_the_compiled_dst`.
- Pytest was not re-run. TwinLab adds no FFI symbol.
- Browser session in `docs/quality/evidence/twinlab-browser-report.json` and `docs/quality/evidence/twinlab-loading.json`. Screenshots: `twinlab-1280.png`, `twinlab-390.png`, `twinlab-390-top.png`, `twinlab-768.png`, `twinlab-injected.png`, `twinlab-from-studio.png`, `twinlab-not-found.png`.
- Asset sizes: `web/twinlab.js` 46660 bytes, `web/twinlab.css` 2736 bytes, `web/twinlab.html` 7237 bytes. `web/studio.js` is now 21662 bytes after the Rehearse control. The studio review above records the earlier 21067-byte measurement.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | Local loopback rehearsal. No public host and no domain was configured. |
| Q02 Page titles | PASS | Title `EmbroideryOS — Machine TwinLab`. Unknown document title `Page not found — EmbroideryOS`. One h1, `Machine TwinLab`. |
| Q03 Meta descriptions | PASS | The TwinLab description is local and non-sensitive, and the document is `noindex`. It does not include a project name, hash, or reviewer. |
| Q04 Favicon and icons | PASS | `GET /favicon.svg` returned 200 `image/svg+xml`, 264 bytes. The mark is `web/favicon.svg`. No home-screen install icon is shipped. |
| Q05 Open Graph image | N/A | TwinLab is not a public shareable page. No production origin exists to host a card. |
| Q06 Canonical URLs | N/A | No indexable production origin. A canonical URL was not invented. |
| Q07 Social previews | N/A | No public social card. The page is `noindex` on loopback. |
| Q08 Custom 404 | PASS | `GET /missing-twinlab-page` returned 404 and the branded page. The report records the recovery link landing on `/` with the design-studio title. `GET /sitemap.xml` and `GET /robots.txt` return that same branded 404. |
| Q09 Loading states | PASS | With fetch latency set to 800 ms, the event line read `Rehearsing the exported file…`, `#workspace` was `aria-busy=true`, and Play was disabled. It then became `Event 0, jump. Object 1.`, `aria-busy=false`, an empty error, and `poseCheck=pass`. The client aborts after 30 seconds. |
| Q10 Error messages | PASS | Geometric preview says timed playback is off and that the view is not a timing or quality claim. The calibrated and connected options stay disabled and show the server reasons: no measured observations, and no telemetry. A pose or qualification mismatch stops playback with a reload instruction. The thread-break control says the event was injected and was not reported by a machine. |
| Q11 Heading structure | PASS | One h1, `Machine TwinLab`. Section h2 headings are Assembly, 2D design, 3D machine, Inspector, Evidence, Selected event, and Timeline. The 404 page has one h1, `Page not found`. |
| Q12 Alternative text | PASS | The design SVG is `role=img` and labelled by its title and description. The machine canvas has an accessible name. Controls have visible text. The favicon is a browser icon. |
| Q13 Sitemap and indexing | PASS | TwinLab `meta robots` is `noindex`. `GET /sitemap.xml` and `GET /robots.txt` return the branded 404. No sitemap file is shipped. |
| Q14 Console errors | PASS | Chrome recorded three WebGL `ReadPixels` performance warnings while screenshots captured the preserved drawing buffer, plus the intentional 404 for `/missing-twinlab-page`. No uncaught exception and no app `console.log`. The driver warnings are environment noise from the screenshot path. |
| Q15 Debug output | PASS | `web/twinlab.js` has no `console.log`, `console.debug`, `generate_satin`, `generate_fill`, `encode_record`, or `scanline`. The workflow test checks those strings against the file on disk. The server prints its bind address and does not print project bodies. |
| Q16 Source maps | PASS | `web/` has no `.map` files and `twinlab.js` has no `sourceMappingURL`. Nothing was published. |
| Q17 JavaScript and performance | PASS | Measured `twinlab.js` at 46660 bytes and `twinlab.css` at 2736 bytes. No framework or dependency bundle. The scene is dependency-free WebGL. Local budget used for this page: 80 KB script and 40 KB CSS. The week-8 frame-time target was not measured. |
| Q18 Mobile layout | PASS | At 390 px, `scrollWidth` equals `clientWidth` (390). At scroll 0 the heading, qualification, and 2D view are visible. After scrolling, Play is 44×56 px and inside the viewport. At 768 px, `scrollWidth` 753 equals `clientWidth` (`innerWidth` 768 includes the scrollbar). At 1280×900 the assembly, split view, inspector, and timeline fit, with one h1. |
| Q19 Consistent spacing | PASS | TwinLab uses the studio ink, blue, surface, 8 px step, and 12 px radius. Desktop is assembly, stage, and inspector, with the timeline across the bottom. Below 961 px the order is stage, timeline, assembly, inspector, and each view is 240 px tall. Transport controls stack to one column. |
| Q20 Working controls and links | PASS | Chrome played to event 15 at 4× and left the estimate at 13.095 seconds (`13.094923647851099` both before and after). A same-turn scrub to event 3 read design 25.000 mm, 14.000 mm, hoop −25.000 mm, −14.000 mm, units 250, 140, object 1, plan stitch 1, file record 3. ArrowRight on the focused stage moved to event 4: design 30.000 mm, 14.000 mm, hoop −30.000 mm, −14.000 mm, units 300, 140, object 1, plan stitch 2, file record 4. Hoop selection, geometric mode, the injected thread break, the 404 recovery link, and Rehearse from the studio were exercised. `poseCheck` and `cameraCheck` were `pass`. |

Club-mark rehearsal observed in Chrome: qualification `Virtual demo · Generic machine · No hardware connected · Physical quality not validated`. Needle-down 106, jumps 13, colour changes 1, trims 0, encoded records 121. Hoop 120 × 100 mm. Revision `01c7f7f9b50cf34cc7b8a81f51772e9d198c8aa3932c3729c0ddf4f60bfe9742`. DST SHA-256 `5daa74c3fe61962ad7109f3c199f953aac501d44763e7adfe906e2d27888964d`. `machine_release_allowed`, `hardware_connected`, and `physical_quality_validated` are false. The earlier report field `seek.shownEvent` is 15 because that sample was taken before the readout refresh; `twinlab-loading.json` is the same-turn measurement. The 25 mm hoop shift is small against the machine bed, so the readout is the coordinate authority.

Known gaps and next action:

- `wasm32-unknown-unknown` is not installed. Native and WASM DST bytes were not compared. That agreement gate stays blocked.
- The second-view button is present. A follower window sharing the simulation clock was not exercised.
- No physical stitch-out, no calibrated observations, and no 192-stitch-out pilot. The 500-design benchmark has not been run. Timing constants are labelled development assumptions.
- Prompt, scan, and web-asset intake, Calibration Lab fitting, vendor CAD, cap and multi-head rigs, live telemetry, and production release are not in this milestone.
- Safari, Firefox, and a screen reader were not exercised. This record is not a WCAG audit.
- `HEAD /twinlab` returns the JSON API 404 because the loopback server handles `GET` for documents. Browsers request the page with `GET`, which returned 200.

Release-ready claim: no. The local export-replay demonstration passed the checks above. Machine release, WASM agreement, calibrated prediction, and a connected twin remain open.

## TwinLab M1 review, 11 October 2026

Delivery/build or commit: local workspace on top of commit `98303601416a3b0d68eae37e939031732bd77c45`. Not committed and not pushed.
Environment and URL: macOS, Chrome via Playwright 1.57.0, renderer `webgl`. Service at `http://127.0.0.1:4175/`. Port 4173 was already taken by `embroidery-studio`. Engine binary SHA-256 `dfeebd327d51c540d5cdaaff083050747c0bbe0d6b78be09a3dd883cd55305fe`.
Scope and classification: local development UI. Native path: Alpine mark JSON, `embroidery-twin`, existing generators, exact DST, raw decode, shared replay, 2D canvas and WebGL machine. Not a public release.
Viewports and browser: Chrome at 1600×1030, 1440×1000, and 390×844. Safari, Firefox, and a screen reader were not used.
Commands and evidence paths:

- `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo build --locked -p embroidery-ffi -p embroidery-twin` — exit 0. core 23, DST 16, pyembroidery oracle 5, FFI 4, motion 10, simulate 3, studio workflow 5, twin 17. Pytest 19 passed.
- `node --test tests/twinlab/unit.mjs` — 51 passed. These exercise HTTP and playback with an injected fixture runner. They do not execute Rust.
- `node --test tests/twinlab/native.mjs` — 11 passed, including the real binary, a fill-spacing change to 1.4 mm, and an HTTP download of the same buffer.
- `tests/twinlab/browser.py --native --url http://127.0.0.1:4175/` — 31 passed, 0 failed, renderer `webgl`, no page errors. Evidence: `evidence/native-gate/browser/browser-tests.json`, `twinlab-native-desktop.png`, `twinlab-native-dark.png`, `twinlab-native-mobile.png`.
- Spacing demonstration: `evidence/native-gate/spacing/spacing.json`. DST SHA-256 changed from `cc939bc4355bef786ebfbdb88b1de34df224d2c89cce527abec4a3475f0d5ada` to `c895401797b8ce29c217a75b5d808e530505594422ed70fbea4dad4b0269998e`. The download is 11324 bytes and matches that hash. Needle placements 3509, records 3604. Both views shared the end event.
- Asset sizes: `app.mjs` 20415 bytes, `scene.mjs` 11682 bytes, `style.css` 11482 bytes, `index.html` 7187 bytes, `server.mjs` 9736 bytes. No npm application dependencies.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | Loopback only. No domain was configured. |
| Q02 Page titles | PASS | Title `EmbroideryOS · TwinLab`. Unknown document title `Page not found — EmbroideryOS`. One h1, `See every stitch. Understand every move.` |
| Q03 Meta descriptions | PASS | The description is local and non-sensitive. `noindex` is set. It does not include a design name or hash. |
| Q04 Favicon and icons | PASS | `GET /favicon.svg` returned 200. The mark is `apps/twinlab/public/favicon.svg`. |
| Q05 Open Graph image | N/A | Not a public shareable page. |
| Q06 Canonical URLs | N/A | No indexable production origin. |
| Q07 Social previews | N/A | No public social card. |
| Q08 Custom 404 | PASS | `GET /missing-twinlab-page` returned 404, title `Page not found — EmbroideryOS`, and a link back to `/`. API misses stay JSON. |
| Q09 Loading states | PASS | With `/api/compile` delayed 800 ms, the status read `Generating in Rust, compiling exact DST bytes and decoding the export…` and Compile was disabled. It then read `Compiled 4,322 exact DST records…` and `Native Rust · byte identity verified`. |
| Q10 Error messages | PASS | Moving the sample so the fill self-intersects shows `Fill self-intersects. Repair its outline before generation.` and blocks export. A missing Rust binary is reported as unavailable. The status line says no hardware is connected and there is no physical calibration. |
| Q11 Heading structure | PASS | One h1. Section h2 headings are Design objects and Execution evidence. The 404 page has one h1. |
| Q12 Alternative text | PASS | Both canvases have accessible names. Icon buttons for rewind, step, play, theme, and fullscreen have accessible names. |
| Q13 Sitemap and indexing | PASS | `noindex` is set. No sitemap is shipped. Unknown paths return 404. |
| Q14 Console errors | PASS | The native browser run recorded no page errors. The spacing run’s only console error was the intentional 404 for `/missing-twinlab-page`. |
| Q15 Debug output | PASS | The page modules have no `console.log` or `console.debug`. The server prints its bind address only. |
| Q16 Source maps | PASS | The TwinLab assets have no `.map` files and no `sourceMappingURL`. |
| Q17 JavaScript and performance | PASS | Measured `app.mjs` 20415 bytes, `scene.mjs` 11682 bytes, `style.css` 11482 bytes. No framework bundle. Local budget used here: 80 KB of script and 40 KB of CSS. A frame-time percentile was not measured. |
| Q18 Mobile layout | PASS | At 390 px the native check and the spacing check reported no horizontal overflow. Play and the object controls remain visible. Desktop 1600 and 1440 show the split view, inspector, and timeline. |
| Q19 Consistent spacing | PASS | The page uses one system-font layout, shared panel radius, and the same blue primary action. Light and dark screenshots were captured. The 3D rig stays inside the viewport. |
| Q20 Working controls and links | PASS | Chrome compiled Alpine mark, scrubbed six timeline positions with both views on the same event, changed the camera without moving time, switched 2D/3D/split, downloaded the exact DST, applied one revision for a duplicate change, recompiled the edit in Rust, and changed fill spacing to 1.4 mm. The 404 link returns to `/`. |

Known gaps and next action:

- `wasm32-unknown-unknown` is not installed. Native and WASM bytes were not compared.
- The Alpine Club dashboard in the reference screenshot, with library, approvals, production, materials and insights, is not installed. TwinLab is the wired design and replay workspace. It does not add a JavaScript stitch generator.
- No physical stitch-out, calibrated timing, machine acceptance, or ISO certificate.
- Safari, Firefox, and a screen reader were not used.
- `embroidery-studio` and this service both default to port 4173. Run one, or set `PORT`.

Release-ready claim: no. The local native export and replay path passed the checks above. Production release and physical qualification remain open.

## Compare workspace review, 11 October 2026

Delivery/build or commit: local workspace on top of commit `98303601416a3b0d68eae37e939031732bd77c45`. Not committed and not pushed.
Environment and URL: macOS, system Chrome via Playwright 1.57.0. Service at `http://127.0.0.1:4175/`. Port 4173 remained in use by `embroidery-studio` and was not stopped.
Scope and classification: local development UI. This review covers the original-asset record and the Original, Editable design, Machine file, PDF proof, and Machine twin workspace on the existing Rust export path. It does not qualify a computer, scanner, tablet, printer, embroidery machine, extra file format, or AI provider. The coverage register stays a requirements record.
Viewports and browser: Chrome at 1440×1000, 768×1024, and 390×844, in light and dark. Safari, Firefox, and a screen reader were not used.
Commands and evidence paths:

- `node --test tests/twinlab/unit.mjs` — 57 passed, 0 failed. Includes the proof route, unconfirmed-scale refusal, `.dts` name retention, and the split content-security policy: proof PDFs send `frame-ancestors 'self'`; the HTML page sends `frame-ancestors 'none'`.
- `node --test tests/twinlab/native.mjs` — 11 passed, 0 failed, against the existing `embroidery-twin` binary. A fill-spacing change still changes the export. The HTTP download matches that buffer.
- `.venv/bin/python -m unittest tests.compat_compare.test_compare -q` — 42 tests, OK. These exercise the policy and the local proof utility. They do not execute Rust and they do not qualify a device.
- `tests/twinlab/browser.py` was not repeated in this appendix. The earlier native browser result remains in the M1 review.
- `BROWSER_EXECUTABLE="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" TWINLAB_URL=http://127.0.0.1:4175/ COMPARE_OUTPUT=evidence/compare .venv/bin/python tests/twinlab/compare_browser.py` — printed `compare browser checks passed`. No unexpected console errors. Evidence: `evidence/compare/compare-desktop.png`, `compare-dark.png`, `compare-tablet.png`, `compare-mobile.png`, `revised.dst`, `revised-proof.pdf`, `revised-proof-page-1.png`, `revised-proof-page-2.png`.
- Edited Alpine mark fill spacing from 0.6 mm to 1.4 mm. Rust wrote a new DST. Downloaded DST SHA-256 `c895401797b8ce29c217a75b5d808e530505594422ed70fbea4dad4b0269998e` matched the replay. The reopened PDF hash matched the downloaded PDF. The attached original was `reference-square.png`, SHA-256 `3499f537b294079f259c0b7c1a04d9485e671e55def24d542a93ed9b7bb3d091`, placed at an explicit 14 mm width. Approval and machine release stayed false.
- The PDF is two landscape pages. Page 2 states that the design exceeds the 1:1 area and was not downscaled. Its 100 mm bar measures 283.4645 points, which matches `100/25.4*72` within 0.02 point. No paper was printed and no scanner was measured.
- Asset sizes: `app.mjs` 36291, `compare.mjs` 7672, `scene.mjs` 11682, `playback.mjs` 2726, `style.css` 13130, `index.html` 13324, `server.mjs` 14983. Script total 58371 bytes. Local budget used here: 80 KB of script and 40 KB of CSS.
- `contracts/compatibility-coverage.json` has 54 rows: 24 supplied baseline rows are `not_qualified`; 30 extensions are `planned_not_qualified` (22) or `partial_developer_utility_only` (8). None is passed, certified, or device-qualified.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | Loopback only. No domain was configured. |
| Q02 Page titles | PASS | Title `EmbroideryOS · Compare`. The unknown-document title remains `Page not found — EmbroideryOS`. |
| Q03 Meta descriptions | PASS | The description names the five local views and says no machine is connected. `noindex` is set. It does not include a design name or hash. |
| Q04 Favicon and icons | PASS | `GET /favicon.svg` returned 200 `image/svg+xml`. The mark is the existing TwinLab icon. |
| Q05 Open Graph image | N/A | Not a public shareable page. |
| Q06 Canonical URLs | N/A | No indexable production origin. |
| Q07 Social previews | N/A | No public social card. |
| Q08 Custom 404 | PASS | `GET /missing-compare-page` returned 404, heading `Page not found`, and the link returned to a compiled TwinLab page. |
| Q09 Loading states | PASS | Changing fill spacing immediately marked the design dirty, disabled DST and PDF download, and said the proof was not current. The new Rust artifact then restored a current binding. |
| Q10 Error messages | PASS | A `.dts` upload keeps the name `kept-name.dts`, is not executed, and the status says it was not renamed. Unconfirmed scale is refused with HTTP 422. The proof says it is not a machine release. |
| Q11 Heading structure | PASS | One h1, `See every stitch. Understand every move.` Compare is a section under that heading. The 404 page has one h1. |
| Q12 Alternative text | PASS | The reopened PDF iframe is named `Reopened PDF proof`. Compare tabs have accessible names. The earlier canvas and transport controls keep their names. |
| Q13 Sitemap and indexing | PASS | `noindex` is set. No sitemap is shipped. |
| Q14 Console errors | PASS | The Compare browser run recorded no page errors. The only console error was the intentional 404 for `/missing-compare-page`. Proof responses allow this origin to frame them; other responses still send `frame-ancestors 'none'`. |
| Q15 Debug output | PASS | The page modules have no `console.log` or `console.debug`. The server prints its bind address only. |
| Q16 Source maps | PASS | `apps/twinlab` has no `.map` files and no `sourceMappingURL`. |
| Q17 JavaScript and performance | PASS | Measured script total 58371 bytes and CSS 13130 bytes, inside the local 80 KB / 40 KB budget. A frame-time percentile was not measured. The 25,000-event memory target was not measured. |
| Q18 Mobile layout | PASS | At 390 px the document did not overflow horizontally and the machine view stayed visible. Tablet 768 and desktop 1440 screenshots show the same revision, with controls stacked on the narrow widths. |
| Q19 Consistent spacing | PASS | Compare uses the existing type, radius, blue primary action, and panel surfaces. Light and dark screenshots were inspected. The 14 mm reference stays small beside the Alpine design because both share one millimetre frame. |
| Q20 Working controls and links | PASS | Chrome attached the reference, generated a proof, scrubbed record 1 in both views, recompiled the 1.4 mm spacing in Rust, downloaded the same DST and PDF bytes, reopened the stored PDF, moved from Original to Editable design with the keyboard, and recovered from the 404 link. |

Known gaps and next action:

- The 24 supplied baseline rows and 30 extensions are not device qualifications. XP, Vista, Windows 7, and earlier hosts are not claimed as full local runtimes. Gateway access is not on-device parity.
- Scanner, WinTAB, printer, and extra embroidery-format adapters are not implemented. The 100 mm bar is a PDF instruction. Physical print and scan error was not measured.
- The proof utility checks DST SHA-256 and raw record bytes. It does not decode coordinates itself and it does not re-run Rust. `coordinate_semantics_independently_verified` stays false.
- `wasm32-unknown-unknown` is not installed. Safari, Firefox, and a screen reader were not used.
- No physical stitch-out, calibrated timing, machine acceptance, AI provider call, or ISO certificate.
- `scripts/check-twinlab-m1.sh` was not re-run for this review.

Release-ready claim: no. The local Compare binding passed the checks above. Universal compatibility and physical qualification remain open.

## Professional baseline 0.3 review, 11 October 2026

Delivery/build or commit: local workspace on top of `4ea98f9`. Not committed in this review.
Scope and classification: non-UI requirements register and local audit utility. No application screen, route, or stitch generator was added.
Commands and evidence paths:

- `.venv/bin/python -m unittest tests.capability_register.test_audit -v` — 29 passed, 0 failed. Log: `evidence/baseline-0.3/ledger-tests.log`.
- `.venv/bin/python tools/audit_capabilities.py` — exit 0, structural audit PASS. 210 records, 54 inherited, 156 new or expanded, nine line groups, fourteen region families, six conflicts.
- `.venv/bin/python tools/audit_capabilities.py --require-acceptance` — exit 1, expected. Zero of 210 rows have accepted evidence.
- Workbook SHA-256 `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128`, copied unchanged from the supplied file. No spreadsheet application was available, so the dashboard was not pixel-rendered. Sheet names, merges, column widths, formula caches, and cell text were inspected from the package.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01–Q07, Q13 | N/A | No public page, domain, or social preview was added. |
| Q02–Q04, Q08–Q12, Q14, Q18–Q20 | N/A | No application UI was changed. The workbook is a requirements spreadsheet, not a product screen. |
| Q15 Debug output | PASS | The audit utility prints the audit result only. The new test module has no debug print. |
| Q16 Source maps | N/A | No client bundle was added. |
| Q17 JavaScript and performance | N/A | No JavaScript was added. The 25,000-event latency target remains unmeasured. |

Known gaps: the 210-row programme is not implemented. `LIBRARY_SUBSET` covers only the existing running, satin, fill, and underlay limits. Decorative sewing, cutting, lettering catalogues, machine transfer, and physical quality remain unqualified. The acceptance gate is intentionally red.

Release-ready claim: no.

## M2-G1 hole fill review, 11 October 2026

Delivery/build or commit: uncommitted tree on `4ea98f9ad29c18351a9b6b47257fc654f2acce94`. Not committed and not pushed. Evidence is [../m2-g1-holes.md](../m2-g1-holes.md).
Environment and URL: macOS, rustc 1.90.0 (Homebrew), Python 3.11.4, Chrome. TwinLab `http://127.0.0.1:4175/`. Studio hole check `http://127.0.0.1:4176/`. The pre-existing studio on port 4173 was not restarted and is not this binary.
Scope and classification: local development UI plus the existing Rust fill. One outer ring and explicit holes, then exact DST replay. Not a public release. Baseline 0.3 acceptance stays 0/210.
Viewports and browser: Chrome at 1440×1000, 768×1024, and 390×844 for TwinLab, and 1440×900, 768×1024, and 390×844 for the studio. Safari, Firefox, and a screen reader were not used.

Commands and evidence paths:

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, and `cargo test --workspace --offline` — exit 0. Counts are in `docs/m2-g1-holes.md`.
- `node --test tests/twinlab/unit.mjs` — 57 passed, 0 failed.
- TwinLab hole browser passed. DST SHA-256 `65fcab89dfa0167b9d4cfbad18c040a3fa25bf8cce6150d712beb1ffca7ad0e3`. PDF SHA-256 `8fc6c6b9f4d5479da62899b2b93376b61fe7208f7d7d36531ee955ae7ef4e081`. Screenshots `evidence/m2-g1/desktop.png`, `tablet.png`, `mobile.png`.
- Studio hole browser passed against the new binary on port 4176. Club-mark hole DST SHA-256 `cb74dd0aaf96dc97c4296a2e34602d47d8ef72af67d94bcbb1f7c2325c848737`, 851 bytes. Screenshots `evidence/m2-g1/studio-desktop.png`, `studio-tablet.png`, `studio-mobile.png`.
- `GET /favicon.svg` returned 200 `image/svg+xml` on both servers. `GET /missing-hole-page` and `GET /missing-studio-page` returned 404 with `Page not found` and a recovery link. The browser did not click those links in this review.
- Asset sizes: TwinLab `app.mjs` 39782, `compare.mjs` 7981, `scene.mjs` 11682, `playback.mjs` 2726, `style.css` 13227. Script total 62171 bytes. Studio `studio.js` 23846 bytes, `studio.css` 5536 bytes. Local budget remains 80 KB of script and 40 KB of CSS.
- Workbook and inherited JSON hashes are unchanged from the baseline 0.3 review. No acceptance cell was promoted.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | Loopback only. No domain was configured. |
| Q02 Page titles | PASS | TwinLab title `EmbroideryOS · Compare`. Studio title `EmbroideryOS — Design studio`. Unknown routes use `Page not found — EmbroideryOS`. One h1 on each exercised view. |
| Q03 Meta descriptions | PASS | Both pages are `noindex`. The descriptions do not include a design name, reviewer, or hash. |
| Q04 Favicon and icons | PASS | Both `GET /favicon.svg` requests returned 200 `image/svg+xml`. The marks are the existing project icons. |
| Q05 Open Graph image | N/A | Not a public shareable page. |
| Q06 Canonical URLs | N/A | No indexable production origin. |
| Q07 Social previews | N/A | No public social card. |
| Q08 Custom 404 | PASS | Both unknown-document requests returned 404 and a branded page with a link back. The browser click on that link was not repeated; the hole flow does not change the 404 route. `unknown_page_is_a_branded_404` passed in the studio workflow tests. |
| Q09 Loading states | PASS | A hole edit disabled DST and PDF download and said the previous artifact was not current. Rust then restored a current binding. Studio showed “Edits are not generated yet” until generate finished. |
| Q10 Error messages | PASS | A hole on the outer boundary was rejected and told the user to move it inside and generate again. Studio said the plan was not changed and kept export disabled. The TwinLab 422 is that rejection. |
| Q11 Heading structure | PASS | TwinLab h1 remains `See every stitch. Understand every move.` Studio h1 remains `Design studio`. |
| Q12 Alternative text | PASS | Compare’s reopened PDF iframe keeps its name. Studio preview text says the outlines include holes and that jumps are dashed. Hole controls have visible labels: Geometry ring, Add hole, Remove hole. |
| Q13 Sitemap and indexing | PASS | `noindex` remains set. No sitemap was added. |
| Q14 Console errors | PASS | TwinLab page errors were empty. The only console error was the intentional 422 for the rejected hole. Studio page errors were empty. A failed generate response was classified as that rejection. |
| Q15 Debug output | PASS | `app.mjs`, `compare.mjs`, and `studio.js` have no `console.log` or `console.debug`. The studio workflow test also rejects generator and debug strings. |
| Q16 Source maps | PASS | No new `.map` files and no `sourceMappingURL` were added. Nothing was published. |
| Q17 JavaScript and performance | PASS | TwinLab script 62171 bytes and CSS 13227 bytes. Studio script 23846 bytes and CSS 5536 bytes. Both sit inside the local 80 KB / 40 KB budget. A frame-time percentile was not measured. |
| Q18 Mobile layout | PASS | TwinLab at 390 px did not overflow horizontally, and the hole edit compiled. Studio at 390 px did not overflow, and Generate stayed usable. Tablet and desktop screenshots show the same hole workflow with stacked controls on the narrow widths. |
| Q19 Consistent spacing | PASS | The hole controls use the existing type, radius, blue action, and panel surfaces. Studio region outlines use the existing blue stroke. |
| Q20 Working controls and links | PASS | Chrome edited a hole, recompiled in Rust, downloaded the replayed DST, regenerated and reopened the PDF, and blocked the stale export. Undo and redo kept or cleared the hole. Add hole, Remove hole, and the studio holes field worked. Machine release stayed unavailable. |

Known gaps and next action:

- ED21, S02, G02, and Q10 stay `NOT_RUN`. This increment does not accept those rows.
- Touching, overlapping, and nested holes are rejected. General clipping and offsets are not implemented.
- `scripts/check-twinlab-m1.sh` was not re-run. The port 4173 studio is the older binary.
- Safari, Firefox, a screen reader, physical print or scan, and a physical stitch-out were not done. WASM remains blocked.

Release-ready claim: no. One edited, exported, and replayed fill with a hole passed the local checks above. Physical qualification and register acceptance remain open.

## Alpine Club studio shell, 11 October 2026

Delivery/build or commit: uncommitted tree on `4ea98f9ad29c18351a9b6b47257fc654f2acce94`. Not committed and not pushed. The studio binary was not rebuilt for the shell polish. SHA-256 `3d22d89ff13b7e3edd8a051d0d4835fdd20b4f73f78bd3cf100339987c4d6845`. Static files are read per request on `http://127.0.0.1:4176/`.
Environment and URL: macOS, Chrome, loopback only. TwinLab remains `http://127.0.0.1:4175/`. The studio on port 4173 was not restarted.
Scope and classification: local development UI. The Alpine Club window, the operational views, and links to the existing TwinLab and Compare pages. Stitches still come from the Rust engine. No public release. Baseline 0.3 acceptance stays 0/210. Workbook SHA-256 `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` and both coverage JSON files SHA-256 `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` are unchanged.
Viewports and browser: Chrome at 1440×1000, 768×1024, and 390×844. Safari, Firefox, and a screen reader were not used.

Commands and evidence paths:

- `tests/studio/shell_browser.py` against port 4176 — passed. Initial plan 4701 needle-down stitches and 301 jumps. Nominal time shown as 7.23 min, which is 4701 ÷ 650. The run opened every operational view, changed row spacing, regenerated, switched Artwork, Stitches, Travel, and Density, zoomed to 125% and back, explored a local proposal, and recorded a geometry review. Screenshots: `evidence/m2-g1/alpine-studio.png`, `alpine-studio-tablet.png`, `alpine-studio-mobile.png`, plus the approvals shots `alpine-desktop.png`, `alpine-tablet.png`, `alpine-mobile.png`.
- A follow-up Chrome load after the colour-name and assistant-height adjustment confirmed 4701 stitches, 301 travel moves, 7.23 min, the Forest colour name, and both assistant buttons inside the 1440×1000 viewport. Page errors and console errors were empty. The full shell script was not repeated after that adjustment.
- `tests/studio/hole_browser.py` against the same shell — passed. Club-mark hole DST SHA-256 `cb74dd0aaf96dc97c4296a2e34602d47d8ef72af67d94bcbb1f7c2325c848737`, 851 bytes. The touching hole was rejected and the plan was not changed.
- `node --check web/studio.js` — exit 0. `web/studio.js` 46118 bytes. `web/studio.css` 10692 bytes. Local budget remains 80 KB of script and 40 KB of CSS.
- `GET /favicon.svg` returned 200 `image/svg+xml`. `GET /missing-alpine-page` returned 404, title `Page not found — EmbroideryOS`, with the link “Back to the design studio”. The browser did not click that link in this review.
- `scripts/check-twinlab-m1.sh` was not re-run. WASM remains blocked: `wasm32-unknown-unknown` is not installed.

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | Loopback only. No domain was configured. |
| Q02 Page titles | PASS | Source title stays `EmbroideryOS — Design studio`. The live title becomes `Alpine Club — EmbroideryOS`. The visible h1 is the project name. Unknown routes use `Page not found — EmbroideryOS`. |
| Q03 Meta descriptions | PASS | The studio page is `noindex`. The description does not include a design name, reviewer, or hash. |
| Q04 Favicon and icons | PASS | `GET /favicon.svg` returned 200 `image/svg+xml`. |
| Q05 Open Graph image | N/A | Not a public shareable page. |
| Q06 Canonical URLs | N/A | No indexable production origin. |
| Q07 Social previews | N/A | No public social card. |
| Q08 Custom 404 | PASS | `GET /missing-alpine-page` returned 404 and a branded page with “Back to the design studio”. The browser did not click that link in this review. |
| Q09 Loading states | PASS | Generate sets the button label to “Generating stitches…” and `aria-busy` until the Rust plan returns. The shell test waited until Geometry checked and the button was enabled again. A separately delayed network fault was not injected. |
| Q10 Error messages | PASS | A hole on the outer boundary told the user to move it inside and generate again, and said the plan was not changed. An ungenerated edit sets the revision line to “Edits are not generated yet” and disables export. |
| Q11 Heading structure | PASS | One h1, the project name. Each operational view has one visible h2: Overview, Design library, Objects, Validation, Approvals, Production, Materials, Insights, Settings. |
| Q12 Alternative text | PASS | The stitch preview has a title and a description. Undo, redo, zoom, add object, and play have accessible names. Display colour chips sit beside the object name. |
| Q13 Sitemap and indexing | PASS | `noindex` remains set. No sitemap was added. |
| Q14 Console errors | PASS | The shell run and the follow-up load recorded no page errors and no console errors. The hole run’s only console error was the rejected-hole response. |
| Q15 Debug output | PASS | `web/studio.js` has no `console.log` or `console.debug`, and none of the forbidden generator strings. |
| Q16 Source maps | PASS | No new `.map` files and no `sourceMappingURL` were added. Nothing was published. |
| Q17 JavaScript and performance | PASS | Studio script 46118 bytes and CSS 10692 bytes, inside the local 80 KB / 40 KB budget. A frame-time percentile was not measured. |
| Q18 Mobile layout | PASS | At 390 px the studio document did not overflow horizontally, and Generate stayed enabled. Tablet 768 and desktop 1440 show the same Alpine project. The narrow layout stacks the rail, objects, canvas, and inspector. |
| Q19 Consistent spacing | PASS | The shell uses the existing surface, radius, type, and blue primary action. The inspected desktop window keeps the object list, artboard, summary, and assistant in one view. |
| Q20 Working controls and links | PASS | Chrome generated the Rust plan, edited row spacing, regenerated, switched the four preview modes, zoomed, explored a proposal, opened every operational view, and recorded a geometry review. The hole flow edited, undid, redid, and rejected a hole. TwinLab and Compare are linked. Machine release stayed unavailable. Arrow-key nudge was not driven from the browser in this pass. |

Known gaps and next action:

- The pictured 10,481 stitches and 16.12 minutes are not the engine result. This design is 4701 needle-down stitches, 301 jumps, and 7.23 nominal minutes.
- The 160 × 160 mm hoop does not fit Compare’s `generic-flat-120-v1`. Studio rehearsal uses the project hoop. Neither path releases a machine.
- Display colours are not thread brands and are not stored in DST. Physical validation was not performed. The digitising assistant is local rules only.
- Artwork draws the regions with the Rust plan faintly over them. It is not a fabric simulation. Stitch-level sculpting is not implemented.
- Keyboard nudge, Safari, Firefox, a screen reader, and a physical stitch-out were not done. `scripts/check-twinlab-m1.sh` was not re-run. WASM remains blocked.
- Register rows stay `NOT_RUN`. This window does not accept them.

Release-ready claim: no. The Alpine Club window and the operational views are wired to the existing Rust studio. Machine release, physical validation, and baseline 0.3 acceptance remain open.

## EmbroideryOS logo, 11 October 2026

The needle mark from the supplied icon is the favicon and the product mark. The studio rail, the studio TwinLab header, and the Compare header at `http://127.0.0.1:4175/` show that mark with the EmbroideryOS name. Compare also shows the line “Stitch a brighter tomorrow”. Both favicon requests returned 200 `image/svg+xml`. Chrome showed the mark at 1440 and 390 on the studio, and at 1440 on Compare and studio TwinLab. The studio page at 390 did not overflow horizontally. No server was rebuilt. The 404 page body was not changed; its tab icon uses the new favicon file. Release-ready claim: no.

## Collapsible sidebar, 11 October 2026

The design studio sidebar has Hide sidebar in the rail and a show control in the page header. The choice is stored as `embroideryos.rail`. Chrome hid and restored the rail at 1440 px, then in dark mode at 390 px. The collapsed 390 px page did not overflow horizontally. Enter on Hide sidebar collapsed it, and Overview and Settings still opened after it was shown again. Page errors and console errors were empty. Release-ready claim: no.

## Design prompt, 11 October 2026

The studio page has a design prompt. It accepts typed text, pasted text, a JPEG or PNG upload, and voice input. Voice uses the browser speech API and writes text into the prompt. Audio is not sent. Propose design posts to `/api/propose`. The studio process calls the xAI responses API, model `grok-4.7`, only when `XAI_API_KEY` is set, and only for region geometry. Rust `parse_project` checks that geometry. The route does not generate stitches. Apply geometry is a separate edit. Apply and generate is a separate control. A missing key, an empty prompt, or Discard leaves the open design unchanged.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback studio. No public origin was invented or published. |
| Q02 | PASS | Document title remains product-specific. The view heading stays the project name. |
| Q03 | PASS | The local description says a geometry proposal is optional and that the service does not release machine files. |
| Q04 | N/A | Favicon was not changed in this pass. |
| Q08 | N/A | No new document route. Unknown pages still use the branded 404. |
| Q09 | PASS | Propose design changes to “Proposing…” and sets `aria-busy` while the request is open. |
| Q10 | PASS | Empty prompt and a missing key both say what failed and that the design was not changed. The key value is not shown. |
| Q11 | PASS | The page keeps one project heading. The prompt is a labelled field, not a second `h1`. |
| Q12 | PASS | The logo remains decorative. The attached reference image has alternative text. Upload image, Voice, and Remove image have visible names. |
| Q14 | PASS | Chrome page errors were empty. Chrome logged the expected 422 when the key was missing. No other console error was recorded. |
| Q15 | PASS | `web/studio.js` has no `console.log` or `console.debug`. The studio workflow test also rejects generator strings. |
| Q16 | N/A | No release artifact or source map was published. |
| Q17 | PASS | `web/studio.js` 56718 bytes, `web/studio.css` 12289 bytes. Local budget remains 80 KB of script and 40 KB of CSS. |
| Q18 | PASS | The prompt flow ran at 1440. Chrome at 1440, 768, 390, and 360 did not overflow horizontally. Dark mode was inspected at 390. The propose click was not repeated at 768 or 360. |
| Q19 | PASS | The prompt uses the existing surface, line, radius, and control height. |
| Q20 | PASS | Empty propose, image attach, voice, discard, and the missing-key propose were exercised. Apply stays unavailable until a proposal exists. |

`pytest python/tests/test_geometry_proposal.py` — 8 passed. Extra stitch fields, motif, a point outside the hoop, and a missing key are rejected without a network call. `cargo test -p embroidery-studio --offline` — 8 passed. `cargo clippy -p embroidery-studio --offline --all-targets -- -D warnings` passed. `node --check web/studio.js` passed. A normalized square fill generates in Rust and `machine_release_allowed` stays false.

Chrome at `http://127.0.0.1:4176/` kept the title Alpine Club and the stitch count 4701 after an empty prompt and after a prompt with an attached JPEG. The studio process has no `XAI_API_KEY`, so a live model call was not run. Headless Chrome has no microphone; the voice control reported that audio is not sent. Safari, Firefox, and a physical microphone were not tested. Screenshots: `evidence/m2-g1/prompt-desktop.png`, `prompt-mobile.png`, `prompt-mobile-dark.png`.

Register rows stay `NOT_RUN`. This prompt does not accept them. Release-ready claim: no.

## SVG geometry intake, 11 October 2026

The design studio has a vector-artwork form. Preview import posts the SVG text to `POST /api/import-svg`. Python converts a bounded subset and Rust `parse_project` checks the draft. The response has no stitches and no DST. Apply geometry is a separate edit. Cancel, a rejected file, and a stale Apply leave the open design unchanged. Generate and studio TwinLab rehearsal use the existing Rust routes. `machine_release_allowed` stays false. A08, G02, and G03 are not accepted. The native hoop is not treated as the centred Compare document.

An empty image chip was visible beside Propose design because `.prompt-preview { display: flex }` overrode the `hidden` attribute. `.prompt-preview[hidden] { display: none }` restores the intended hidden state. The chip stays hidden until an image is attached.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback studio. No public origin was invented or published. |
| Q02 | PASS | The studio document title stays product-specific. After import the view heading is the project name `badge-hole`. |
| Q03 | PASS | The local description says SVG import and geometry proposals stay local drafts and that the service does not release machine files. |
| Q04 | N/A | Favicon was not changed in this pass. |
| Q08 | N/A | No new document route. Unknown pages still use the branded 404. Empty SVG and a bad width return JSON 400. Oversized SVG text returns JSON 413. |
| Q09 | PASS | Preview import changes to “Previewing…” and sets `aria-busy` while the request is open. |
| Q10 | PASS | Text SVG, cancel, and a stale Apply each say what failed and that the design was not changed. After generate, the status says stitches were generated and machine release remains unavailable. |
| Q11 | PASS | The page keeps one project heading. Vector artwork is a labelled form, not a second `h1`. |
| Q12 | PASS | The logo remains decorative. Choose SVG, Preview import, Apply geometry, and Cancel have visible names. Palette swatches are hidden from assistive technology. |
| Q14 | PASS | Chrome page errors were empty. Chrome logged the expected 422 for the text SVG. No other console error was recorded. |
| Q15 | PASS | `web/studio.js` has no `console.log` or `console.debug`. The studio workflow test also rejects generator strings. |
| Q16 | N/A | No release artifact or source map was published. |
| Q17 | PASS | `web/studio.js` 63415 bytes, `web/studio.css` 12511 bytes. Local budget remains 80 KB of script and 40 KB of CSS. |
| Q18 | PASS | Chrome at 1440 completed reject, cancel, stale, apply, and generate. Overflow was checked at 1440, 768, and 390. Dark mode at 390 completed preview and cancel. Generate was not repeated at 768 or 390. |
| Q19 | PASS | The form uses the existing prompt bar, file button, choice row, radius, and control height. The width field is 8.5rem. |
| Q20 | PASS | Reject, cancel, stale Apply, undo, Apply, Generate, and Production → Rehearse in TwinLab were exercised. Apply stays unavailable until a preview exists. Preview import took keyboard focus at 390. |

`cargo test -p embroidery-studio --offline` — 9 passed. `cargo clippy -p embroidery-studio --offline --all-targets -- -D warnings` passed. `node --check web/studio.js` passed. `.venv/bin/python -m unittest discover -s tests/svg_geometry -v` — 48 passed. `tools/check_svg_geometry_native.py` passed both fixtures. `tools/audit_capabilities.py` — structural audit PASS. Workbook and inherited JSON hashes were unchanged.

Chrome at `http://127.0.0.1:4176/` kept Alpine Club at 4701 needle-down stitches and 7 objects through reject, cancel, and stale Apply plus undo. Apply of `fixtures/svg-geometry/badge-hole.svg` then Generate produced 1380 needle-down stitches, revision `665f912eb3ce`, and DST SHA-256 `62e97ba0a9548c8e8ace1504d5f3d41c69929829d175b1e0f214f20f6a859060`. Studio TwinLab on the same origin showed hoop 100 × 100 mm and that same hash. Qualification remained “Virtual demo · Generic machine · No hardware connected · Physical quality not validated”. The native project was not loaded into Compare on port 4175. Screenshots: `evidence/svg-import/desktop-generated.png`, `tablet.png`, `mobile-dark.png`, `mobile-cancel.png`, `twinlab.png`.

Safari, Firefox, and a physical stitch-out were not done. The full workspace regression suite was not re-run; the studio package tests and the SVG unit and native-smoke gates were. Register rows stay `NOT_RUN`. This import does not accept them. Release-ready claim: no.

## Variable-width satin, 11 October 2026

A satin object can store one width per centreline point in `column_widths_mm`. An empty list keeps the constant-width generator, including its rejection of a rung wider than one DST record. A non-empty list uses `generate_satin_variable` in Rust. A sharp corner is mitered up to twice the half-width. A rung whose axis span would pass 12 mm is split. A sewing segment shorter than 0.2 mm is omitted, and a column that sews no cross-rung fails before the plan is written. Underlay is still only centre run and edge run together. Separately selected underlays are not implemented. Undo keeps the selected object when that object is still in the restored project. `machine_release_allowed` stays false. This does not accept a register row.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback studio. No public origin was invented or published. |
| Q02 | PASS | The document title stayed Alpine Club — EmbroideryOS. The view heading stayed Alpine Club. |
| Q03 | PASS | The existing local description was left in place. This pass did not add a public page. |
| Q04 | N/A | Favicon was not changed. |
| Q08 | N/A | No new document route. The mismatched width uses the existing generate JSON error. |
| Q09 | PASS | Generate uses the existing “Generating stitches…” control. The mismatched width returned an error and the design stayed editable. Undo restored the generated plan. The in-flight label was not screenshot. |
| Q10 | PASS | One width for two centreline points produced an error containing “one column width” and a revision line containing “not generated”. Undo cleared that error and restored the generated revision. |
| Q11 | PASS | The page kept one project heading. Column widths is a labelled inspector field, not a second `h1`. |
| Q12 | PASS | “Column widths · mm” names the textarea. The hint is visible text. The logo remains decorative. |
| Q14 | PASS | Chrome page errors were empty. No console error other than an allowed 422 from the mismatched generate failed the run. |
| Q15 | PASS | `web/studio.js` has no `console.log` or `console.debug`, and no generator or encoder names. |
| Q16 | N/A | No release artifact or source map was published. |
| Q17 | PASS | `web/studio.js` 64792 bytes, `web/studio.css` 12511 bytes. Local budget remains 80 KB of script and 40 KB of CSS. |
| Q18 | PASS | Chrome at 1440 added a satin column, generated, rejected a short width list, and undid it. Overflow was checked at 768 and 390. Dark mode was inspected at 390. Generate was not repeated at 768 or 390. |
| Q19 | PASS | The widths control uses the existing inspector label, textarea, hint, and stack spacing. |
| Q20 | PASS | Generate, the mismatch error, and Undo were exercised. The widths field remained on the selected satin after undo. A separate tab-order pass was not repeated. |

`cargo test --workspace --offline` passed on the generator before the undo-selection change: core 34, dst 16, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio workflow 9, twin 18. `cargo clippy -p embroidery-core --offline --all-targets -- -D warnings` exited 0 and reported `embroidery-core` fresh. `node --check web/studio.js` passed. `tests/studio/satin_browser.py` passed against `http://127.0.0.1:4176/` after the studio binary was rebuilt. The structural audit was not re-run; the workbook and inherited JSON were not edited.

Chrome at 1440 started from Alpine Club at 4701 needle-down stitches, added a satin centreline from (30, 40) to (70, 40) with widths 2 mm and 8 mm, and generated 4768 needle-down stitches. Travel moves were 302. The display time was 7.34 min at the studio 650 spm estimate. Revision `646eccd46b22`. Readiness stayed “Geometry checked”. The preview note said machine release remains unavailable, and physical validation stayed “Not performed”. A one-width list was rejected, then Undo restored the generated count and the width list. Screenshots: `evidence/satin-variable/desktop-generated.png`, `tablet.png`, `mobile-dark.png`.

The miter is capped and is not a Boolean offset, so rails can still cross on a sharp or folded centreline. A near-reversal keeps the outgoing normal. Constant-width satin still rejects an over-wide rung. Widths must match the centreline point count and stay in (0, 48] mm. Safari, Firefox, and a physical stitch-out were not done. The native project was not sent to Compare on port 4175. Register rows stay `NOT_RUN`. Release-ready claim: no.

## Satin correctness and export integration, 11 October 2026

This pass keeps the variable-width generator and closes the folded-rail gap recorded above. A collapsed corner bisector is rejected. The outgoing normal is not substituted, and a crossed column is not reshaped. Nominal rails and pull-compensated rails are both admitted before a batch is committed. The split policy stays visible: 12 mm on an axis, at most 16 pieces, and satin sewing shorter than 0.2 mm is omitted. Constant-width satin still rejects a rung past one DST record. Underlay is still centre run and edge run together. Independent underlay selection and lettering were not started. `machine_release_allowed` stays false. This does not accept a register row.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. The workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128`. Both inherited coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6`.

The studio used for the browser run was PID 79089, started 18:42:01, `target/debug/embroidery-studio` SHA-256 `079ebbeb840d32697b51966bb39208935a808f412eb6847e179bb5e6bec205b8`, listening on `127.0.0.1:4176`. Port 4173 was left running and was not used. Port 4175, node PID 47889, was left running and was not used; its worker hash was not measured. TwinLab Compare for this run was a new process, node PID 79091, on `127.0.0.1:4181`. Its session reported `native-rust` and `generic-flat-120-v1`. The compile provenance `engine_sha256` was `ccb40ff2c385ccfa6d394e88f81f3f7ffd33827b0a4890817f68fa09dda28209`, the same SHA-256 as `target/debug/embroidery-twin` on disk.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | Studio heading stayed Alpine Club. TwinLab heading stayed “See every stitch. Understand every move.” |
| Q03 | PASS | Existing local descriptions were left in place. No new public page. |
| Q04 | N/A | Icons were not changed. |
| Q08 | N/A | No new document route. The fold and width errors use the existing JSON error surfaces. |
| Q09 | PASS | Studio generate and TwinLab compile showed progress, then a result or a recoverable error. The rejected fold left the previous 4768-stitch plan in place. |
| Q10 | PASS | Chrome showed “Object 8, section 2: the centreline reverses or folds too sharply to place a column. Move the centreline, reduce the width, or split this into separate columns. The plan was not changed.” A one-width list still failed with “one column width”. |
| Q11 | PASS | Each page kept its existing single primary heading. Column widths remains a labelled field. |
| Q12 | PASS | “Column widths · mm” names the textarea. The hint is visible text. Icon controls in these views already have accessible names. |
| Q14 | PASS | Both Chrome runs finished with no page errors. Console errors other than an allowed 422, or 404 on the TwinLab run, failed the scripts. None remained. |
| Q15 | PASS | `web/studio.js` and `apps/twinlab/public/app.mjs` have no `console.log` or `console.debug`. |
| Q16 | N/A | No release artifact or source map was published. |
| Q17 | PASS | `web/studio.js` 64888 bytes and `web/studio.css` 12511 bytes, inside the local 80 KB and 40 KB budgets. `apps/twinlab/public/app.mjs` is 41375 bytes. No separate TwinLab budget is recorded. |
| Q18 | PASS | Studio generation and rejection ran at 1440. Overflow was checked at 768 and 390, with dark mode at 390. TwinLab compile, replay, proof, undo, and download ran at 1440. Overflow was checked at 768 and 390. The smaller viewports were not asked to generate again. |
| Q19 | PASS | The width list uses the existing inspector label, textarea, hint, and stack. The hint wraps inside the inspector. |
| Q20 | PASS | Generate, rejection, Undo, and Redo kept the satin selected in Studio. TwinLab save/reopen, proof, both-view replay, stale download, Undo, Redo, and exact DST download were exercised. A separate tab-order pass was not repeated. |

`cargo fmt --all` exited 0. `cargo test --workspace --offline` exited 0 after the corner rejection: core 36, dst 17, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio workflow 9, twin 19. `cargo clippy --workspace --all-targets --offline -- -D warnings` exited 0. `.venv/bin/python -m pytest python/tests -q` passed 27. SVG geometry unittest passed 48. Capability-register unittest passed 29. `tools/audit_capabilities.py` exited 0. `tools/audit_capabilities.py --require-acceptance` exited 1, with 0 of 210 rows accepted. `tools/check_svg_geometry_native.py` passed 2 fixtures and kept machine release false. `node --check` passed for `web/studio.js` and `apps/twinlab/public/app.mjs`.

The first TwinLab browser run timed out because it read the example’s existing satin instead of “New satin”. After that fix, Undo did not restore 2 mm and 6 mm: a duplicate change event stored the same 2 mm and 8 mm list again. An unchanged width list is now ignored, and the rerun passed.

Studio Chrome on 4176 started from Alpine Club at 4701 needle-down stitches, added object 8 from (30, 40) to (70, 40) with widths 2 mm and 8 mm, and generated revision `646eccd46b22fcae9732dc023412d6cb18eada977c6a75463cdf2e46246771b3`. DST SHA-256 `7e6c7a7a65a0dbd98878edbe185110b5ba6861bee6ffee4d4488e7c4908683bf`. Plan sewing counts were 2486, 408, 641, 416, 142, 464, 144, and 67. Object 8, the satin column, is the 67. The seven earlier objects sum to 4701, so the project total is 4768. That total is not a stitch-quality score. Travel buckets in the same response, counting jump, trim, and colour change, were 85, 43, 70, 36, 23, 48, 2, and 2. Readiness included the 12 mm split policy with separate split and omitted counts. Machine release stayed false. A one-width list was rejected and Undo restored 2 mm and 8 mm. A reversed centreline was rejected without changing the 4768 count. The desktop screenshot is that rejection state.

TwinLab on 4181 kept widths through save and reopen. The saved file `evidence/satin-variable/twinlab-variable.json` is the 2 mm and 6 mm design, SHA-256 `bf2b2bb91957eaaa7f4aa391dca33f7ca1c3a52c6823fce99ef4aa819ba1f184`. Its design SHA-256 at the later 2 mm and 8 mm compile was `849e47d195898e74ab43852049517eae8ab2f56b15a790156b6e4170e9f57b51`. Constant-width DST SHA-256 `be595dcdeb8dd1cf6b30f1cfd8c854b237b8d699da289529e3e501713d1eb5a4`. Listed 2 mm and 6 mm DST SHA-256 `f702d7e3876cb340d04d9b89b03fcf50618e7b60361c2b359a3250d6eeadcfa5`. The downloaded file `evidence/satin-variable/twinlab-variable.dst` is the later 2 mm and 8 mm export, SHA-256 `61c472cdbe01a922eded6dddd31ae531b85d0fb4172ade2f80cb24977bcaed77`, matching provenance. Those three files are different revisions. Scrubbing the decoded trace kept the 2D and 3D views on the same record, and the machine window showed raw records. The PDF reference was the constructed 14 mm square, not the satin artwork. Editing the width cleared the current binding and disabled download. Undo restored 2 mm and 6 mm with New satin still selected. Redo returned 2 mm and 8 mm. Release stayed false. The proof still reports that the design does not fit the 1:1 panel.

Twenty-one declared fixtures cover straight, tapered, bent, square, obtuse, compensated, underlay, split, and filtered columns, plus repeated points, mismatched and invalid widths, reversal, hairpin, self-crossing, folded rails, nonlocal overlap, and a column that fails only after pull compensation. Rejected fixtures leave the plan empty. A filtered column’s decoded DST adds no compiler jump beyond the unit pen model. The constant-width golden hash in the twin unit test remains `1267d41f8fdb70b6878222e5f33cda61af04ddd3022ce36e3d5252eb08e46398`.

The (0, 48] mm width range is an input range, not a manufacturing range. The 12 mm split is the implemented policy, not a validated sewing length. This is not a Boolean offset, not commercial parity, and not a physical stitch-out. Safari and Firefox were not run. `tests/twinlab/native.mjs` was not re-run because it overwrites `evidence/native`. WASM remains blocked. Register rows stay `NOT_RUN`. Release-ready claim: no.

## Independent satin underlay, 11 October 2026

Centre run and edge run can each be on or off, for constant-width and variable-width satin. The four selections are none, `centre_run`, `edge_run`, and both. An absent pass length inherits that object's maximum stitch length. A present length is stored only when it differs from the inherited value, so older canonical documents keep their revision identity. Both passes still travel centre line, then the nominal left rail, then the reversed nominal right rail, and then the top satin. Empty underlay still sews the top satin only. Nominal and pull-compensated rail admission still runs before any underlay choice. A rejected column leaves the previous plan unchanged. Phase text separates centre-run underlay, edge-run underlay, top satin, and positioning and inter-phase connectors. The top-satin placement sequence stays the same when only the underlay selection changes; the connectors and the whole DST may change. Zigzag underlay, a new inset, lettering, and automatic fabric prescriptions were not added. `machine_release_allowed` stays false. This does not accept a register row.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. Rehashed on this source state, the workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54782 bytes). Both coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32594 bytes each).

Earlier export records stay separate. The Studio DST `7e6c7a7a65a0dbd98878edbe185110b5ba6861bee6ffee4d4488e7c4908683bf` belongs to revision `646eccd46b22fcae9732dc023412d6cb18eada977c6a75463cdf2e46246771b3` and studio binary `079ebbeb840d32697b51966bb39208935a808f412eb6847e179bb5e6bec205b8`. The saved TwinLab design `evidence/satin-variable/twinlab-variable.json`, SHA-256 `bf2b2bb91957eaaa7f4aa391dca33f7ca1c3a52c6823fce99ef4aa819ba1f184`, is the 2 mm and 6 mm revision. The later 2 mm and 8 mm download `evidence/satin-variable/twinlab-variable.dst`, SHA-256 `61c472cdbe01a922eded6dddd31ae531b85d0fb4172ade2f80cb24977bcaed77`, belongs to design `849e47d195898e74ab43852049517eae8ab2f56b15a790156b6e4170e9f57b51`, worker `ccb40ff2c385ccfa6d394e88f81f3f7ffd33827b0a4890817f68fa09dda28209`, and preservation PDF `52959306ffd0e75dd433c84326e6eaedd50d889285448dd1effbd162388233dc`. Those files were present and unchanged. Port 4175 was not measured and is not part of this run. Port 4173, PID 40352, stayed up and was not used.

This run's studio was PID 85866 on `127.0.0.1:4176`, binary SHA-256 `b50c7c4712b81cfdcbf80cb06fa2ee6e669b8214e3d246ab514cc1965702ae45`, mtime 2026-10-11 19:43:18. TwinLab stayed the existing node PID 79091 on `127.0.0.1:4181`. Its compile hashed `target/debug/embroidery-twin` SHA-256 `37666818f87fe30aa25847da51aaffdca7e7afb71d7e4a518c014f81485cd3f4`, mtime 2026-10-11 19:43:35. The session mode was `native-rust` and the profile was `generic-flat-120-v1`.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | Studio title and heading stayed Alpine Club. TwinLab heading stayed “See every stitch. Understand every move.” |
| Q03 | PASS | Existing local descriptions were left in place. No new public page. |
| Q04 | N/A | Icons were not changed. |
| Q08 | N/A | No new document route. Fold rejection uses the existing generate error. A non-JSON open says the file is not JSON and leaves the project in place. |
| Q09 | PASS | Studio generate and TwinLab compile returned a plan or a recoverable rejection. The in-flight label was not separately captured. |
| Q10 | PASS | With edge run still enabled, the folded centreline showed an error containing “reverses” and “not changed”. The displayed stitch count stayed on the previous plan. Unit tests reject a pass length above 12.1 mm without writing a partial batch. |
| Q11 | PASS | Each page kept its existing single primary heading. Centre run and Edge run are labelled inspector controls. |
| Q12 | PASS | The checks have visible labels. Each length field is shown only while its pass is enabled, with the text “Centre-run maximum stitch length” or “Edge-run maximum stitch length”. The phase note is visible text. |
| Q14 | PASS | Both Chrome runs finished. Console lines containing 422, or 404 on the TwinLab run, were the allowed class. No other console error remained. |
| Q15 | PASS | `web/studio.js` and `apps/twinlab/public/app.mjs` have no `console.log` or `console.debug`. `node --check` passed for both on the final source. |
| Q16 | N/A | No release artifact or source map was published. |
| Q17 | PASS | `web/studio.js` 70461 bytes and `web/studio.css` 12594 bytes, inside the local 80 KB and 40 KB budgets. `apps/twinlab/public/app.mjs` is 46969 bytes. No separate TwinLab budget is recorded. |
| Q18 | PASS | The Studio sequence ran at 1440, then overflow was checked at 768 and 390 with dark mode at 390. The TwinLab sequence ran at 1440, then overflow was checked at 768 and 390. The smaller viewports were not asked to generate again. |
| Q19 | PASS | The new controls use the existing inspector choice, field, hint, and stack. Phase text wraps inside the column. |
| Q20 | PASS | Open, underlay selection, generate, save, reopen, phase inspection, export, replay, PDF generate and reopen, selection change, invalidation, Undo, and Redo were exercised on both routes. A separate tab-order pass was not repeated. |

`cargo fmt --all` exited 0. `cargo test --workspace --offline` exited 0. Counts from that log, kept separate: core 39, dst 17, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio workflow 9, twin lib 20, twin binary 0, doc-tests 0. `cargo clippy --workspace --all-targets --offline -- -D warnings` then exited 0; `/tmp/embroidery-clippy.log` has no warning or error line. `.venv/bin/python -m pytest python/tests -q` passed 27. SVG geometry unittest passed 48. Capability-register unittest passed 29. `tools/audit_capabilities.py` exited 0: records 210, inherited 54, new or expanded 156, line groups 9, region families 14, conflicts 6. `tools/audit_capabilities.py --require-acceptance` exited 1, with 0 of 210 rows accepted. `tools/check_svg_geometry_native.py` passed 2 fixtures and kept machine release false. These suite results are not one combined total.

Clippy first failed on `sew_selected_underlay` having more than seven arguments. The three rail slices were grouped into a private `UnderlayPaths` value. The later browser DST hashes matched the run from before that regroup. The first TwinLab proof timed out on `#attach-reference` because the machine and twin panes hide that control. The test opens the original pane first. That rerun passed. The product still fails an unsupported underlay name instead of dropping it.

The Rust selection test generates constant straight, variable taper, and an admitted bend in all four modes, twice each. That is 12 valid configurations. Within each configuration the stitch list matches. Across the four modes the recorded top-satin sequence matches, and a bare column differs from the same column with both passes. The 21-fixture domain set still passes: 10 admitted columns, and 11 rejected columns in every mode, each leaving an empty plan empty. A separate test starts from a running-stitch plan, rejects a folded satin add in every mode, and keeps both the stitches and the metadata. The locked constant DST hash `1267d41f8fdb70b6878222e5f33cda61af04ddd3022ce36e3d5252eb08e46398`, variable hash `cd82ac5eee2d07e84e771a13cf9567b865ece1e15b4d8174e8d3d7b6e47c0cbc`, combined-underlay hash `9de0dbdb2cb803c585be157e1b85e9df4f28216e97e045278493fe59761d4f71`, and running revision `daaf1d0fed513d354da2cc305c3eafeffeeb7a6028f379b5e18074d128542580` remain the assertions in those tests. A centre-run stitch shorter than 0.2 mm is kept. Centre length 5 mm and edge length 2 mm are applied on the same column.

Studio Chrome on 4176 started from Alpine Club at 4701 needle-down stitches and added Column eight. The four constant-width DST hashes, in order none, centre, both, and edge, were `267e29372ac1743e895946ba308864bfe365633d0b5e50d10bad1cc9acc7872f`, `843f7b954b04139c9f511b5348dcc76da6707280e3eab2c33683cbaae3b1babc`, `d64fd3b7d5fd5741c0996b2de28ce3294c152c5b06b6eeac09d4cc0a0aaebc8d`, and `dec16b577ee18c413736c3581d4db8464a3eb843741502a238f3a90f972c2979`. Their top-satin sequences matched. Repeating the edge-only generate reproduced the fourth hash. An unchanged inherited length, displayed as 3.5, did not add an undo step. Widths 2 mm and 8 mm kept one top sequence across none, both, and edge-only. The saved project `evidence/satin-underlay/studio-project.json`, SHA-256 `fbc9659a1b7f2cca7f8f69c5467fe0243955de7cd7ac9f08e5b419a8a9695ec7`, has underlay `["edge_run"]`, `edge_run_max_stitch_mm` 2, widths 2 and 8, and no centre length field. Reopen regenerated revision `103a25cf0840224d2f7137dfb4d3e7387f5cc0b40f5847f2f3cb2d11cb9e4947`. The downloaded file `evidence/satin-underlay/studio-taper-edge.dst`, SHA-256 `82f3850e9470bb02ed4b65272ea629b999f020631b20b677e16e62bfef39c007` (16073 bytes), matches that response. A pyembroidery decode of that file counts 4834 stitch commands, 345 jumps, 7 colour changes, and 1 end. Machine release stayed false. Checking centre run dirtied the revision, disabled export, and asked for a new generate while Column eight stayed selected. Undo restored the edge-only revision. Redo restored the dirty centre-run selection. A folded centreline with edge run still on was rejected and the displayed count stayed put. Screenshots: `evidence/satin-underlay/studio-desktop.png`, `studio-tablet.png`, `studio-mobile.png`.

TwinLab on 4181 left the example Shield border on the historical both-pass reading until a control changed: `underlay_mm` 2.5 and no underlay list. New satin then ran the four selections. The constant top sequences matched, and a second compile of the edge-only design reproduced its DST. The saved file `evidence/satin-underlay/twin-constant-edge.json` is that edge-only design. Its download `twin-constant-edge.dst`, SHA-256 `79fc95f2168c816d0d8540eeab199432c8e4400f3efca7d4540d6c85d988ca6f` (14306 bytes), decodes to 4434 stitch commands, 160 jumps, 1 trim, 3 colour changes, and 1 end. Widths 2 mm and 8 mm with centre length 4 mm and edge length 2.5 mm produced design SHA-256 `3746826d5430faa7fa4b8b6a0a6cdc855e3e80e381b2f6a73b5aff05f186c3bd` and DST SHA-256 `97561291dcefe9776cf85d901816ea0139a72dacd7585049a7b781fb9b8d69fe` (14348 bytes; 4444 stitch commands, 164 jumps, 2 trims, 3 colour changes, 1 end). The proof PDF `evidence/satin-underlay/twin-taper-proof.pdf`, SHA-256 `6e12f8934aa3e616c9701b5598b805d1bfe8f46fc192422a078ed33b5bc25843` (111673 bytes), was generated, reopened, and downloaded against artifact `af1b78a46ac836bea9402ea431e92361e026aa11bdc0dac2bdae0b7d8e1000f9`. The reference image is the constructed 14 mm square. Unchecking edge run cleared the current binding and disabled both downloads while New satin stayed selected. Undo restored the taper geometry and the same DST hash. The proof stayed not current because undo advances the design revision. Redo returned edge run off with New satin selected. Release stayed false. The manifest is `evidence/satin-underlay/twin-manifest.json`. Screenshots: `twin-desktop.png`, `twin-tablet.png`, `twin-mobile.png`. The desktop shot is the post-redo state, whose current design had not been compiled again.

The (0, 48] mm width range remains an input range. Underlay running stitches are not put through the 0.2 mm satin omit. This is not a new offset algorithm, not commercial parity, and not a physical stitch-out. Safari and Firefox were not run. `tests/twinlab/native.mjs` was not re-run. WASM remains blocked. Register rows stay `NOT_RUN`. Release-ready claim: no.

## Native lettering L1, 11 October 2026

One editable text object sews through the existing Rust satin, underlay, and DST path. The font is EmbroideryOS Block 01, id `embroideryos-block-01`, version `1`, content SHA-256 `aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd`. The repertoire is A–Z, 0–9, space, hyphen, and a straight apostrophe: 39 entries, 38 of them drawable. A native design that contains text is `embroidery-project` version 2. Version 1 documents stay version 1, and a version 1 document that contains text is rejected without dropping the text. TwinLab keeps schema version 1 and stores the same text intent on the object. The entered string, font id, font version, and font-content hash stay on the object after generation. Glyph geometry is produced by Rust from original stroke recipes and the existing satin generator. Cap heights of 8, 12, and 20 mm in the kernel test are software sizes, not measured sewing limits. `machine_release_allowed` stays false. Rows F01–F11 stay `NOT_RUN`.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. The workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54782 bytes). Both coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32594 bytes each). The closed satin-underlay and satin-variable evidence directories were not rewritten. This run did not start or use port 4175. Port 4173, PID 40352, stayed up and was not used.

Clippy rejected three lettering lints before the final test: the spacing check now uses an inclusive range, the drawable count uses `is_empty`, and one trace assertion uses a string literal. The first Studio browser attempt timed out on Verify export because that control sits on the Production page. The test opens Production, reads the export back, and returns to Design studio. That rerun passed against the same binaries. No Rust rebuild happened after the proof hashes below were recorded.

`cargo fmt --all` exited 0. `cargo clippy --workspace --all-targets --offline -- -D warnings` exited 0. `cargo test --workspace --offline` exited 0. Counts from that log, kept separate: core 47, dst 17, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio workflow 9, twin lib 21, twin binary 0, doc-tests 0. `.venv/bin/python -m pytest python/tests -q` passed 27. SVG geometry unittest passed 48. Capability-register unittest passed 29. `tools/audit_capabilities.py` exited 0: records 210, inherited 54, new or expanded 156, line groups 9, region families 14, conflicts 6. `tools/audit_capabilities.py --require-acceptance` exited 1, with 0 of 210 rows accepted. `tools/check_svg_geometry_native.py` passed 2 fixtures and kept machine release false. `node --check` passed for `web/studio.js` and `apps/twinlab/public/app.mjs`. `web/studio.js` is 79279 bytes and `web/studio.css` is 12638 bytes. `apps/twinlab/public/app.mjs` is 51771 bytes. These suite results are not one combined total.

The kernel repeats every drawable glyph at 8, 12, and 20 mm and checks the checkpoint strings `A H I L O V 0 8`, `ALPINE 08`, and `O'NEIL-08`. Space advances without a sewing stitch. A 20 mm gap between letters is a jump, and the tests record no trim from the lettering layer. All four underlay selections keep the top-satin sequence on representative glyphs and on `ALPINE 08`. Seventeen rejection cases leave a pre-populated running plan and its metadata unchanged. The messages name the unsupported character and its code point and say the text was left unchanged.

Studio Chrome on 4176, PID 91445, used binary SHA-256 `323aff13e424bb0581e1dc700c7e9cbb1e2a2e53662d9c851df433efe425bf34`, mtime 2026-10-11 20:49:43. It started from Alpine Club at 4701 needle-down stitches, added object 8 as text `ALPINE 08` at cap height 12 mm with both underlay passes, and generated revision `a30438072d75cfaf5f696deb2636adec8208dc529e1dc3fd27c66420f783505f`. DST SHA-256 `f4c4c078f6b8415e1de8cb68692d158eb5deea01038666807a5ab99644a013b5`. The trace contains `U+0041`, `U+0020 space`, and `U+0038`. A 1 mm move and a letter-spacing edit each made one undo step and restored that generated revision. Changing the text to `ALPINE 09` was one undo step before the next generate. That revision is `a4507ca9e5d037cfacd2b70ae2d554d0001f79219f78886d08ef2e7b8fd5e0bb`, DST SHA-256 `f5ee1c34edc29e3570d2e6b818acf148523c0a99e34c2425f8f5d68b6aa5cf40`. Undo restored `ALPINE 08`, kept Lettering selected, and disabled export. Generating again reproduced the first DST and revision. Edge, none, centre, and both kept one top-satin sequence, and the second both-pass generate reproduced the first hash. `O'NEIL-08` kept the same font pin. Its revision is `b53795583a6bf1ef597fcdda661b2ffbbe17d71e3bab3926201b018f5c45ed5d`, DST SHA-256 `445b00aad88c3b180046c29410743f17c55b8a5f7d081c8ab0cca35e217c6a81`. The saved project `evidence/lettering/studio-project.json`, SHA-256 `7bae214910b7909353c9f096a534b56de48cfb597f4efffe6647ae1269523e9e` (11745 bytes), is format version 2 and still has empty points. Reopen regenerated that DST. The download `evidence/lettering/studio-oneil.dst` matches it (19790 bytes). A pyembroidery decode counts 5956 stitch commands, 462 jumps, 7 colour changes, and 1 end. Verify export reported a read-back. `Alpine` was rejected with `U+006C` and the field stayed `Alpine`. `CAFÉ` was rejected with `U+00C9` and the field stayed `CAFÉ`. The displayed stitch count stayed on the previous plan, and export stayed disabled. Screenshots: `evidence/lettering/studio-desktop.png`, `studio-tablet.png`, `studio-mobile.png`. The desktop shot is the accent rejection.

TwinLab stayed node PID 79091 on `127.0.0.1:4181`. Its compile hashed `target/debug/embroidery-twin` SHA-256 `cdb9e4310f1b73fca34eea2f6a294f125613b153c84076714ec8af0c78fce064`, mtime 2026-10-11 20:50:14. The session mode was `native-rust`. Schema version stayed 1. Added text `ALPINE 08` at cap height 10 mm ran none, centre, both, and edge. The top sequences matched, and repeating both reproduced DST SHA-256 `9975176af363a70bcbfeac23051b81bc1660e159d75866fa969f2445c9c75672` at design revision 8. The proof was bound to that revision. `ALPINE 09` is revision 9, DST SHA-256 `887a46640127b6bb7dbcfbb9ba8f6a483fb4416586c678142a349fd7869658eb` (17303 bytes). Undo restored the `ALPINE 08` bytes at revision 10 and left the previous PDF not current. The rebound proof is id `5cc11ccea8f36d2e5ed149f463f534c385783823bbfb078bfd6bd099a3c92590`, PDF SHA-256 `c0dca9e16f178d9a8d5dacbd096a853e2b13e99383b07db8468a397f01a4189c` (124703 bytes), artifact `ed1e20399b44b8d866251f6231cbb11f4bd9430b361302473b5c0c96606c4ae6`. The PDF was reopened and downloaded. The DST download `evidence/lettering/twin-alpine08.dst` (17288 bytes) matches the restored hash. A pyembroidery decode counts 5304 stitch commands, 284 jumps, 1 trim, 3 colour changes, and 1 end. The `ALPINE 09` file counts 5317 stitch commands, 276 jumps, 1 trim, 3 colour changes, and 1 end. The decoded replay is `evidence/lettering/twin-alpine08-replay.json`, SHA-256 `7b2b10df532e1140935dd0c6b828ec6ee00c87e7f8253eaf99afb886a4886ecb1` (2436197 bytes). Moving the origin to -50 mm cleared the current binding. Undo restored -52 mm, the same DST hash, and a proof that was still not current. `O'NEIL-08` on a 390 px viewport kept the font pin. The saved design `evidence/lettering/twin-oneil.json`, SHA-256 `f2cbd23912d6aeb1aaa72187772b6ea486ecef6e24552f602ea9009dfcf63ee5` (2504 bytes), reopened as text. `O’NEIL` with U+2019 was rejected, the field stayed on that character, and export stayed disabled. The reference image is the constructed 14 mm square. Screenshots: `twin-desktop.png`, `twin-tablet.png`, `twin-mobile.png`. The desktop shot is the rejected curly-apostrophe state, revision 14, not compiled. Release stayed false.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | Studio heading stayed Alpine Club. TwinLab heading stayed “See every stitch. Understand every move.” |
| Q03 | PASS | Existing local descriptions were left in place. No new public page. |
| Q04 | N/A | Icons were not changed. |
| Q08 | N/A | No new document route. Unsupported text uses the existing generate error. |
| Q09 | PASS | Studio generate and TwinLab compile returned a plan or a recoverable rejection. The in-flight label was not separately captured. |
| Q10 | PASS | `Alpine` showed `U+006C` and “left unchanged”. `CAFÉ` showed `U+00C9`. TwinLab showed `U+2019`. Export stayed disabled and the entered text stayed in the field. |
| Q11 | PASS | One primary heading remained on each page. Lettering controls use labelled fields. |
| Q12 | PASS | Add text and + Text have visible names. The font pin is text. The preview draws returned stitches. |
| Q14 | PASS | The exercised Chrome flows reported no unexplained console errors. A 422 response is the rejection path. |
| Q15 | PASS | `node --check` passed. The edited client scripts have no `console.log` or `console.debug`. |
| Q16 | N/A | Local debug binaries only. No public source map was published. |
| Q17 | PASS | `web/studio.js` is 79279 bytes and `web/studio.css` is 12638 bytes, inside the recorded 80 KiB and 40 KiB budgets. TwinLab `app.mjs` is 51771 bytes. |
| Q18 | PASS | The flow was completed at about 1440 px. Overflow stayed within one pixel at 1440, 768, and 390. `O'NEIL-08` was entered at 390 px in TwinLab. Mobile shots show the text field. |
| Q19 | PASS | The new controls use the existing field, button, and add-row spacing. |
| Q20 | PASS | Add, edit, undo, redo, underlay, save, reopen, DST download, verify, compile, proof, and the rejection path were exercised. |

Lowercase, accents, more than one line, curved baselines, font import, monograms, and batch names remain outside this increment. The 8, 12, and 20 mm heights are software checks. Physical legibility was not measured. Safari and Firefox were not run. `tests/twinlab/native.mjs` was not re-run. WASM remains blocked. Register rows stay `NOT_RUN`. Release-ready claim: no.

## Name drop L2, 11 October 2026

A bounded name drop personalises one text object from a table. It is an F06 implementation candidate. It does not accept F06 or any other lettering-register row. F01–F11 stay `NOT_RUN`. Acceptance stays 0 of 210. The font remains EmbroideryOS Block 01 version 1, content SHA-256 `aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd`. No glyph was added. Cap height on the badge is 8 mm as a software size, not a measured sewing limit. `machine_release_allowed` stays false. `evidence/name-drop/source-identity.json` remains the pre-edit snapshot. The workbook and both coverage JSON files were not edited.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. The workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54782 bytes). Both coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32594 bytes each). Satin, underlay, and lettering L1 evidence directories were not rewritten. Port 4175 stayed down and was not measured. Port 4173, PID 40352, stayed up and was not used. TwinLab stayed node PID 79091 on `127.0.0.1:4181`; its JavaScript is read with `Cache-Control: no-store`, so the node process was not restarted. Studio was rebuilt and restarted only on `127.0.0.1:4176`, PID 97722, binary SHA-256 `54e0914ac45e4973c4706ae7abd9324057822f0b222c146eb36f93c4c85a5c6a`, mtime 2026-10-11 22:01:59, 4103296 bytes. Twin binary SHA-256 remained `cdb9e4310f1b73fca34eea2f6a294f125613b153c84076714ec8af0c78fce064`.

`cargo fmt --all` exited 0. `cargo clippy --workspace --all-targets --offline -- -D warnings` exited 0 after the proof command arguments were grouped into `ProofCommand`. `cargo test --workspace --offline -- --test-threads=8` exited 0. Counts from that log, kept separate: core 53, dst 18, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio lib 1, studio workflow 9, twin lib 21, twin binary 0, doc-tests 0. `.venv/bin/python -m pytest python/tests -q` passed 27. Compat unittest passed 43, including the native replay protocol assertion, and the square fixture protocol remained `twinlab-m1/1`. SVG geometry unittest passed 48. Capability-register unittest passed 29. `tools/audit_capabilities.py` exited 0: records 210, inherited 54, new or expanded 156, line groups 9, region families 14, conflicts 6. `tools/audit_capabilities.py --require-acceptance` exited 1, with 0 of 210 rows accepted. `tools/check_svg_geometry_native.py` passed 2 fixtures and kept machine release false. `node --check` passed for `web/studio.js`, `web/batch.js`, and `apps/twinlab/public/app.mjs`. `web/studio.js` is 81605 bytes, `web/batch.js` is 39108 bytes, and `web/studio.css` is 13525 bytes. `apps/twinlab/public/app.mjs` is 55825 bytes. These suite results are not one combined total. `tests/twinlab/native.mjs` and `scripts/check-twinlab-m1.sh` were not run.

The kernel keeps 20 names, `ALPINE 01` through `ALPINE 20`, as 20 completed rows, and keeps `ITEM-001` and `ITEM-004` when both say `ALPINE 08`. Their native revisions match. Their item ids stay distinct. One debug run of that test, which then also checks the four-row duplicate, took 0.03 s real and peaked at 4997120 bytes resident, with a 2917184 byte peak footprint. The figure is in `evidence/name-drop/timing.json`. It is one run, not a percentile. The same core suite finished in 0.11 s. Negative cases in `negative_tables_preserve_the_template` keep the template: lowercase `Alpine` (`U+006C`), `CAFÉ` (`U+00C9`), a curly apostrophe (`U+2019`), blank text, spaces, a comma (`U+002C`), placement overflow at max X 20 mm, a wrong font pin, `generic-flat-120-v1`, an empty profile, a missing object, a repeated item id, an item id containing `/`, a wrong CSV header, 101 rows, both `items` and `csv`, and a version 1 document that contains text. The row limit test generates 100 rows and does not certify a cancellation. A name is not shrunk to fit.

Chrome on 4176 opened `fixtures/name-drop/badge.json` after Alpine Club had reached 4701 stitches. The badge is a 120 × 80 mm design with one running outline and one text object. The table came from Load example and then `fixtures/name-drop/four-names.csv`. Placement was 14, 22, 110, 48 mm. Generate was started with Enter on the focused button. All four rows completed. `ITEM-001` and `ITEM-004` share DST SHA-256 `6569b2e57cc4bc3704cfe2939dbbe845be09f176b908985cfe801961030fd135` and project revision `3a2eaa44582645c538612785dec6c689b9e2a6430629a99586510f11b46e52a4`. Their proof ids differ: `4a162535e078bdf5aa1ea991d02044d191d86751d81fef36e2857a4b6ce23578` and `5b2dbd22f9fcf613f13eda339717ba130936a3c24ed72011773c8fda7e288642`. The shared PDF SHA-256 is `b73d9541ad6cd157b45d1c1c5ea16068185b3100b89911df7809e33665ee717f`. `ITEM-002` DST is `584e7ae9f3f1cd3218907cbdcb8e4e15bebf10959693c930fbb96b3efb99dbd4`, proof `582192d546558e64879bbd8a8eee4a59d12c082778e3ba1773fef28cda824a0c`. `ITEM-003` is `O'NEIL-08`, DST `9aaf49387f0c275d5cfa21adec7ea02ee5714d67bc0b9250218e3c8e28b83cff`, proof `e373b6d0cd1df46a601fa2537573d03f0f9651efb5048f369de642fb6ebf2ce4`, with `U+0027` and `U+002D`. The bundle stayed disabled after two proofs and enabled only after four. The bundle `evidence/name-drop/name-drop-batch.json`, SHA-256 `aae7e0e3387dc94ed7dc182df8e4baa351b4c869c347103d822a2210667599cd` (89046 bytes), has `certifies_batch` true, `machine_release_allowed` false, and `physical_qualification` false. The downloaded DST is 1160 bytes and begins `LA:`. A pyembroidery decode counts 179 stitch commands, 35 jumps, 1 colour change, and 1 end. The saved project is format version 2, keeps empty points, and keeps the font pin. Selecting `ITEM-002` showed that row’s proof text and did not call `ITEM-001` current. The status line follows the selected row.

Undo of an edit from `ALPINE 08` to `ALPINE 09` and back restored studio export SHA-256 `6569b2e57cc4bc3704cfe2939dbbe845be09f176b908985cfe801961030fd135`, the same bytes as the name-drop DST. The batch proof stayed not current and the completion bundle stayed disabled. Changing `ITEM-003` to `CAFÉ` left that text visible, failed the row with `U+00C9`, completed the other three rows, and did not certify the batch. Show template returned the badge text `ALPINE 08`. Reopening the saved `ITEM-001` project kept version 2, the entered text, and the font pin, and marked the previous batch stale. A template edit while `ITEM-002` was in flight marked the batch stale, cleared current artifacts, kept the entered table text, and did not apply the late response. Cancellation after `ITEM-001` had completed left that row inspectable, marked the other three cancelled, and did not publish a completion manifest. The server’s all-cancelled report was not copied over the completed row.

TwinLab on 4181 loaded `ITEM-001.replay.json` without replacing the open Alpine mark design. A binding with item id `ITEM-999` stayed not current. A binding with the replay’s snapshot, item, revision, and DST became current, with machine release unavailable. Clear removed the name-drop replay and left Alpine mark in place. Screenshots: `evidence/name-drop/studio-desktop.png`, `studio-tablet.png`, `studio-mobile.png`, `twin-desktop.png`, `twin-mobile.png`. The 390 px studio shot is `ITEM-002` selected. The 390 px TwinLab shot is after Clear. Document overflow stayed within one pixel at 1440, 768, and 390 on the studio, and at 1440 and 390 on TwinLab. Page errors were empty. Console errors were empty. Safari and Firefox were not run.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | The open badge set the title to `Name drop badge — EmbroideryOS`. The page kept one `h1`. TwinLab’s heading stayed “See every stitch. Understand every move.” The unknown-page title is `Page not found — EmbroideryOS`. |
| Q03 | PASS | The studio and TwinLab pages keep their existing local descriptions and `noindex`. The name drop does not add a public page. |
| Q04 | N/A | Icon files were not changed. `GET /favicon.svg` on 4176 still returned 200 `image/svg+xml`. |
| Q08 | PASS | `GET /not-a-studio-page` on 4176 returned 404 with the branded “Page not found — EmbroideryOS” document. Workflow test `unknown_page_is_a_branded_404` passed. A bad batch prepare returned 422 JSON, not a document. |
| Q09 | PASS | Generate set `aria-busy`. Counts moved through running, completed, failed, and cancelled. Cancellation finished between rows. A late response did not become the current result. |
| Q10 | PASS | `CAFÉ` stayed in the row and failed with `U+00C9`. The template text stayed `ALPINE 08`. Stale and cancelled batches said they were not a current completion. |
| Q11 | PASS | One `h1` remained. The name drop section is an `h2`. Row controls have accessible names. |
| Q12 | PASS | Show, item id, and text controls expose `aria-label`s. The decorative mark keeps an empty alt. Glyph codes are text. |
| Q14 | PASS | The exercised Chrome flows reported no page errors and no console errors. |
| Q15 | PASS | `node --check` passed. Workflow test `the_browser_does_not_plan_stitches` scanned `web/studio.js` and `web/batch.js` and passed. |
| Q16 | N/A | Local debug binaries only. No public source map was published. |
| Q17 | PASS | `web/studio.js` is 81605 bytes, inside the 80 KiB budget. `web/batch.js` is 39108 bytes, inside the 48 KiB budget. `web/studio.css` is 13525 bytes, inside the 40 KiB budget. |
| Q18 | PASS | The batch was completed at 1440 px. `ITEM-002` was selected at 390 px. TwinLab Clear was used at 390 px. Overflow stayed within one pixel at the checked widths. |
| Q19 | PASS | The section uses the existing panel, field, button, and table spacing. Placement uses the same 8 px gap, in four columns, and in two columns below 700 px. |
| Q20 | PASS | Example, CSV import, Enter to generate, row selection, proof, DST, project, replay, PDF, bundle, undo, template, reopen, cancel, stale rejection, and TwinLab replay, mismatch, match, and clear were exercised. |

Lowercase, accents, a curly apostrophe, more than one line, curved baselines, font import, monograms, and new machine formats remain outside this increment. The 8 mm badge height is a software size. Physical legibility was not measured. Safari and Firefox were not run. `tests/twinlab/native.mjs` was not re-run. WASM remains blocked. Register rows stay `NOT_RUN`. This name drop does not accept F06. Release-ready claim: no.

## Lettering L3, 11 October 2026

Block 01 version 2 is a separate pinned font. Version 1 and its content hash stay `aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd`. Choosing version 2 is an explicit edit. A missing or mismatched version 1 pin still fails and is not upgraded. This does not accept F06 or any other lettering-register row. F01–F11 stay `NOT_RUN`. Acceptance stays 0 of 210. `machine_release_allowed` stays false. The workbook and both coverage JSON files were not edited. `evidence/lettering/` and `evidence/name-drop/` were not rewritten. `evidence/name-drop/timing.json` still records the earlier debug invocation at 0.03 s and 4997120 bytes resident. That figure is not a browser or PDF measurement.

Increment A adds original lowercase a–z and an explicit U+2019 mapping, and records baseline 92, cap-top 8, cap-body 84, x-top 50, x-height 42, ascender-top 8, descender-bottom 132, and accent-top -28 in a 100-unit scale box. The straight apostrophe stays a separate character. Increment B adds exactly Áá Éé Íí Óó Úú Ää Ëë Ïï Öö Üü Ññ Çç. Entered text is stored unchanged. Layout uses NFC from `unicode-normalization` 0.1.24 and grapheme clusters from `unicode-segmentation` 1.12.0. Both crates are pinned. The Unicode data pin is 16.0.0. Compatibility normalization is not used. An unlisted sequence, including a second combining mark, is rejected and is not stripped. The version 2 content SHA-256 is `78e8506438960cc575d896ba338d65d711144bb5acc956205dbb28c081074a89`.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. The workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54782 bytes). Both coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32594 bytes each). Fresh listeners, measured after the studio rebuild: `127.0.0.1:4173` embroidery-studio PID 40352, left running and not used; `127.0.0.1:4175` not listening and not started; `127.0.0.1:4176` embroidery-studio PID 4961, binary SHA-256 `4c2ff89721477e2335034a12b1d73c47ae356227de625b7765e97eabdec8f2c4`, 4504384 bytes, mtime 2026-10-11 23:22:03; `127.0.0.1:4181` node PID 79091, not restarted. The twin binary on disk is SHA-256 `6b12abfc7bc96a87124e8a040c4e547632a832696e7309dbb61d91ce7eb9b900`, 2837936 bytes, mtime 2026-10-11 23:22:02. TwinLab spawns that binary for each compile.

`cargo fmt --all -- --check` exited 0. `cargo clippy --workspace --all-targets --offline -- -D warnings` exited 0. `cargo test --workspace --offline -- --test-threads=8` exited 0. Counts from that log, kept separate: core 64, dst lib 18, dst `lettering_v2` 1, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio lib 1, studio workflow 9, twin lib 21, twin binary 0, doc-tests 0. `.venv/bin/python -m pytest python/tests -q` passed 27. Compat unittest passed 43. SVG geometry unittest passed 48. Capability-register unittest passed 29. `tools/audit_capabilities.py` exited 0: records 210, inherited 54, new or expanded 156, line groups 9, region families 14, conflicts 6. `tools/audit_capabilities.py --require-acceptance` exited 1, with 0 of 210 rows accepted. `tools/check_svg_geometry_native.py` passed 2 fixtures and kept machine release false. `node --check` passed for `web/studio.js`, `web/batch.js`, and `apps/twinlab/public/app.mjs`. `web/studio.js` is 81888 bytes, `web/batch.js` is 41067 bytes, and `web/studio.css` is 13525 bytes. `apps/twinlab/public/app.mjs` is 55825 bytes. These suite results are not one combined total. `tests/twinlab/native.mjs` and `scripts/check-twinlab-m1.sh` were not run. `tests/studio/name_drop_browser.py` was not re-run.

The first lettering run in this milestone failed two assertions that were probes: the version 2 hash was still the sentinel `pending`, and a ligature check rejected the word “replaced” inside the sentence “was not replaced”. The hash is now the value above. The ligature check looks for `U+FB01` and `not replaced`, and expects the message not to contain `fi`. A version 1 batch of `Café` fails on the first unsupported scalar, `U+0061`, and does not skip ahead to `U+00E9` or normalize. That assertion was corrected to the historical path. Clippy required `accepted_font` to use `?`, and the version 2 twin imports were moved into the twin test module. The first browser run reached the downloaded proof and then failed a raw search for “Engineering review only” inside the compressed PDF. The binding hash is the check. The rerun passed.

Every version 2 repertoire character is admitted at the reference configuration for software heights 8, 12, and 20 mm. Cap 12 mm with max Y 23 mm rejects `g` and admits `A`. Cap 20 mm with min Y 1.0 mm rejects `Á` and admits `A`. On the badge, cap 8 mm, origin Y 28 mm, column 1.6 mm, and max Y 36.5 mm admit `A` and reject `g`. The admitted row stays, the failed row keeps `g` and has no project, and the batch is not certified. Dots, accents, and the cedilla stay disconnected under none, centre, edge, and both underlays for `i`, `é`, and `Ç`. Accented `í` and `ï` use a stem without the `i` dot. A failing final character restores the pre-populated plan. Twenty-five composed and decomposed pairs, including `Café` and `Cafe` plus U+0301, share stitch geometry and DST bytes. Their stored strings stay different. `O'Neil` and `O’Neil` do not share DST bytes.

Chrome on 4176 opened the unchanged badge, which is still version 1. The four-name example completed. Switching from `ITEM-001` to `ITEM-002` kept the running outline identical and changed the text, stitch count, preview lines, proof panel, and replay. Those DST hashes match the earlier name-drop run: `6569b2e57cc4bc3704cfe2939dbbe845be09f176b908985cfe801961030fd135` and `584e7ae9f3f1cd3218907cbdcb8e4e15bebf10959693c930fbb96b3efb99dbd4`. The 20-name table then failed under version 1. `Café` stayed in the row and failed with `U+0061`. `O’Neil` failed with `U+2019`. `O'Neil` failed with `U+0065`. No row was dropped. Selecting Block 01 version 2 marked that batch not current and left every entered name in place. A second page opened the same badge file and still showed version 1. Undo returned the open object to version 1. Redo returned version 2. Regeneration at placement 14, 22, 110, 48 mm completed all 20 rows. `N-03` and `N-04` share DST SHA-256 `fb48287b4ce88b8076e3088d03dd79747e89982f38821ab70c2016d95b6bfb2e` and differ in project revision. `N-03` and `N-20` share that DST and revision `3190aa3f2925df8edf1daea03db7e5777a9f56a1f2910a28922d028274fd04ad`, and their proof ids differ: `9246926cab1eab402ebc29f7d0aec5060ba4789e008689ca06edd4e30adefff6` and `9e3f8bdc82d951f66258d9ad510715288f8c16fdf57e6c80fc8e21017d39b156`. `N-17` DST is `e2df4b51b188fd04c16eee6655b9da4272010060c1e8cb90f71886ca41ab458d`. `N-18` DST is `881a9bc5aec800900d36c0e876e173d5e8db41f8e5f404a8ba9508bc922bb6d9`. The `N-03` PDF SHA-256 is `69dc090188ef1788d9043039eab626e4aaddd1cb827ee061a5fb5e713de558ff` (10192 bytes). The DST is 920 bytes, begins `LA:`, and pyembroidery counts 115 stitch commands, 19 jumps, 1 colour change, and 1 end. The bundle `evidence/lettering-v2/name-drop-batch.json`, SHA-256 `54fbc0099b698ac6aa451fff22e5919f636cc2d006d28b163ff7563e31f47159` (408143 bytes), has `certifies_batch` true, `machine_release_allowed` false, and `physical_qualification` false. Reopening the saved `N-03` project kept `Café` and version 2, and marked the batch not current. The badge file on disk was unchanged. TwinLab on 4181 loaded `N-03.replay.json` without replacing Alpine mark. A binding for `N-04` stayed not current. The `N-03` binding became current, with machine release unavailable. Clear removed the replay and left Alpine mark in place.

The studio screenshots are the reopened `Café` project, so the name-drop message says the batch is not current and the placement fields show the suggested 119 by 42 mm. The certified generation used 14, 22, 110, 48 mm. Document overflow stayed within one pixel at 1440, 768, and 390. At 390 px the font control could take focus and the `Café` field still showed `Café`. Page errors were empty. Console errors were empty. Safari and Firefox were not run. `GET /not-a-studio-page` returned 404 with “Page not found — EmbroideryOS”. `GET /favicon.svg` returned 200 `image/svg+xml`.

The partner stitch-out study was not run. The proposed set is six fixed strings, two heights chosen with an operator, and two repeats: 24 samples. No machine, hoop, fabric, needle, or thread setting was chosen. No photograph, measurement, stop, or transcription was captured. This is not a minimum lettering size and not machine certification.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | The open badge set the title to `Name drop badge — EmbroideryOS`. The page kept one `h1`. TwinLab’s heading stayed “See every stitch. Understand every move.” The unknown-page title is `Page not found — EmbroideryOS`. |
| Q03 | PASS | The studio and TwinLab pages keep their existing local descriptions and `noindex`. Version 2 does not add a public page. |
| Q04 | N/A | Icon files were not changed. `GET /favicon.svg` on 4176 still returned 200 `image/svg+xml`. |
| Q08 | PASS | `GET /not-a-studio-page` on 4176 returned 404 with the branded page. Workflow test `unknown_page_is_a_branded_404` passed. |
| Q09 | PASS | Generate set `aria-busy`. The version 1 table finished as 20 failed rows. The version 2 table finished as 20 completed rows. A font change marked the previous rows not current. |
| Q10 | PASS | `Café` stayed visible and failed on version 1 with `U+0061` and “left unchanged”. Version 2 sewed that same string. The font-mismatch and descender failures say the plan or template was not changed. |
| Q11 | PASS | One `h1` remained. The name drop section is an `h2`. The font control is labelled “Font version”. |
| Q12 | PASS | Show, item id, and text controls expose `aria-label`s. The new font control has a visible label. The preview draws returned stitches. |
| Q14 | PASS | The exercised Chrome flows reported no page errors and no console errors. |
| Q15 | PASS | `node --check` passed. `web/studio.js` and `web/batch.js` contain no `console.log` or `console.debug`. Workflow test `the_browser_does_not_plan_stitches` passed. |
| Q16 | N/A | Local debug binaries only. No public source map was published. |
| Q17 | PASS | `web/studio.js` is 81888 bytes, inside the 80 KiB budget. `web/batch.js` is 41067 bytes, inside the 48 KiB budget. `web/studio.css` is 13525 bytes, inside the 40 KiB budget. |
| Q18 | PASS | The version 2 batch was completed at 1440 px. At 390 px the font control took focus and the `Café` field kept `Café`. Overflow stayed within one pixel at 1440, 768, and 390. TwinLab Clear was used at 390 px. |
| Q19 | PASS | The font control uses the existing select style. Placement and the table keep the existing spacing. |
| Q20 | PASS | Example generation, row binding, CSV import, version 1 rejection, explicit version 2, undo, redo, 20 proofs, DST, project, replay, PDF, bundle, reopen, and TwinLab replay, mismatch, match, and clear were exercised. |

More than one line, curved baselines, font import, monograms, automatic fitting, and new machine formats remain outside this increment. The 8, 12, and 20 mm heights are software checks. The 24-sample stitch-out was not run. Safari and Firefox were not run. `tests/twinlab/native.mjs` was not re-run. WASM remains blocked. Register rows stay `NOT_RUN`. This repertoire does not accept F06. Release-ready claim: no.

## Lettering L4, 12 October 2026

Horizontal lines are an explicit layout policy, `embroideryos-line-layout/1`, on the existing text object. They are not a new font and not a document-format change. Native projects stay version `"2"`. TwinLab stays `schema_version` 1. Objects that omit `layout` keep the historical single-line path. Block 01 version 1 remains `aca657de98489950e35cb72d7c802d9a4d81958f2915b93307af7b9a4ca028bd`. Block 01 version 2 remains `78e8506438960cc575d896ba338d65d711144bb5acc956205dbb28c081074a89`. Unicode normalization stays `unicode-normalization` 0.1.24, grapheme segmentation stays `unicode-segmentation` 1.12.0, and the Unicode data pin stays 16.0.0. Line breaks are a separate derived rule: LF and CRLF. The received string is stored unchanged. This does not accept F06 or any other register row. F01–F11 stay `NOT_RUN`. Acceptance stays 0 of 210. `machine_release_allowed` stays false. The workbook and both coverage JSON files were not edited. `evidence/lettering/`, `evidence/name-drop/`, and `evidence/lettering-v2/` were not rewritten.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. The workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54782 bytes). Both coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32594 bytes each). The badge file remained `519cdab0c3e4c4368c9f2eb60d32df422259533d19561fc6bfc2f7cfd2f8e5ba`. The saved L3 bundle remained `54fbc0099b698ac6aa451fff22e5919f636cc2d006d28b163ff7563e31f47159` (408143 bytes). Listeners left running and unused for this proof: `127.0.0.1:4173` embroidery-studio PID 40352; `127.0.0.1:4175` not listening and not started; `127.0.0.1:4176` embroidery-studio PID 4961; `127.0.0.1:4181` node PID 79091, not restarted. The proof used new processes: `127.0.0.1:4186` embroidery-studio PID 14601, binary SHA-256 `447acc2319158584174ad34125288dbcb6cd76812790fc23c875cd3a55ad5472`, 4604480 bytes, mtime 2026-10-12 01:17:09; `127.0.0.1:4187` node PID 14602. The twin binary on disk is SHA-256 `6d5c61bfacd6a612b8d7dec7713cd1e07624510af48c6e8780627516ce6292ca`, 2947936 bytes, mtime 2026-10-12 01:17:53.

The worker check is the child image, not the Node pid. On 4187, one compile without the trace header returned no worker headers. Two compiles with `X-Worker-Trace: 1` used child pids 14849 and 14855. Both mapped `/Users/favl/embroidery-engine/target/debug/embroidery-twin`. The header hash, the `lsof` text-image hash, and the on-disk file hash were the same value above. Both children had exited after the response. The Node pid stayed 14602. The JSON body did not carry the worker identity, and the traced artifact id matched the untraced compile. The record is `evidence/lettering-l4/worker-identity.json`.

`cargo fmt --all -- --check` exited 0 after formatting long lines in the layout sources. `cargo clippy --workspace --all-targets --offline -- -D warnings` exited 0. `cargo test --workspace --offline -- --test-threads=8` exited 0. Counts from that log, kept separate: core 73, dst lib 18, dst `lettering_v2` 2, fill-hole 2, pyembroidery oracle 5, ffi 4, motion 10, simulate 3, studio lib 1, studio workflow 9, twin lib 21, twin binary 0, doc-tests 0. The workflow test was run again after the later `web/studio.js` edit and passed, including the 80 KiB and forbidden-token checks. `.venv/bin/python -m pytest python/tests -q` passed 27. Compat unittest passed 44, including `test_saved_lettering_proof_shows_the_selected_row`. SVG geometry unittest passed 48. Capability-register unittest passed 29. `tools/audit_capabilities.py` exited 0: records 210, inherited 54, new or expanded 156, line groups 9, region families 14, conflicts 6. `tools/audit_capabilities.py --require-acceptance` exited 1, with 0 of 210 rows accepted. `tools/check_svg_geometry_native.py` passed 2 fixtures and kept machine release false. `node --test tests/twinlab/unit.mjs` passed 57 with the worker-trace environment unset. `node --check` passed for `web/studio.js`, `web/batch.js`, `web/lettering.js`, `apps/twinlab/public/app.mjs`, `apps/twinlab/server.mjs`, and `tests/twinlab/worker_trace.mjs`. `web/studio.js` is 81897 bytes, `web/batch.js` is 44820 bytes, `web/lettering.js` is 5756 bytes, and `web/studio.css` is 13665 bytes. `apps/twinlab/public/app.mjs` is 59673 bytes. These suite results are not one combined total. `tests/twinlab/native.mjs` and `scripts/check-twinlab-m1.sh` were not run. `tests/studio/name_drop_browser.py` and `tests/studio/lettering_v2_browser.py` were not re-run.

The added PDF check keeps the binding-hash assertion and also reads the page. On the saved L3 bundle it required the engineering labels, rendered the DST replay panel, and required ink. `N-03` and `N-04` replay crops match because those PDF bytes match. `N-17` and `N-18` crops differ. Extracted PDF text is not required to equal the source string. The project and manifest still contain `Café`, `Cafe` plus U+0301, `O’Neil`, and `O'Neil`. The L4 browser proof for `ML-001` is `evidence/lettering-l4/ML-001.proof.pdf`, SHA-256 `d8bb5ff62dec018ae94ed2a255098d200598ea207f8c8afb0507ad30463c85d3`, 13181 bytes, beginning `%PDF`. Its labels include the engineering review text, the replay panel has ink, and the rendered crop is `evidence/lettering-l4/ML-001-replay.png`. The saved project and replay keep `Café` followed by a line feed and `Alpine 08`. The DST is `evidence/lettering-l4/ML-001.dst`, SHA-256 `74df59eec1576f960f7dabf712efc3b9233b449ebe2f9119138053caa7308e26`, 1310 bytes, beginning `LA:`.

Layout coverage in Rust asserts one through four lines at left, centre, and right, with advance, offset, and baseline. Repeated multiline DST bytes match. A newline without `layout` stays on the historical rejection path. Blank lines consume a baseline and sew nothing. Spaces keep their spacing. Descender-and-accent clearance at a short spacing returns an actionable software error and says it is not a thread-collision or legibility measurement. A wider spacing admits the same pair. Four underlay selections keep the top satin distinct from the jump connectors. The quoted 20-row CSV keeps each item, including decomposed equivalents, both apostrophes, a blank line, and a spaces-only line. Duplicate text can share DST bytes and keeps separate item records. A failing last row does not certify the batch.

Chrome on 4186 opened the unchanged badge. Enter in the text area inserted a line and did not call generate. The text area was then set to `Café` / `Alpine 08`, centre alignment, block width 100 mm, and baseline spacing 18 mm. Control+Enter generated. Save and reopen kept the original string, the layout policy, and centre alignment. Enter in the table inserted a line and did not call the batch item route. The quoted CSV imported 20 items. Placement 2, 2, 118, 78 mm was set after the layout. The batch completed and certified. `ML-001` and `ML-003` shared DST bytes and differed in revision. `ML-001` and `ML-004` shared DST bytes and revision and differed in item id. Their proof ids differed. `ML-006` and `ML-007` did not share DST bytes. `ML-009` retained U+0020. Glyph codes did not contain U+000A. Changing spacing to 24 mm cleared the current proof and the completion manifest while the text stayed. Undo restored 18 mm and redo returned 24 mm. The proof stayed not current after both. A second page held `ML-002`, cancelled, and released the late response. `ML-001` stayed completed, `ML-002` stayed cancelled with its text, neither proof became current, and the bundle stayed disabled. TwinLab on 4187 compiled the lined design. Enter inserted a line and did not compile. Changing origin Y left the first baseline in place. Changing spacing changed the DST hash. Undo restored that hash. Loading the `ML-001` replay named `ML-001` and left the open design name `Lined badge`. Overflow stayed within one pixel at 1440, 768, and 390 on Studio, and at 1440 and 390 on TwinLab. Page errors were empty. Console errors were empty.

Two corrections were required. At 1440×1000 the name-drop panel covered the inspector, so the line control could not be clicked. The studio grid now keeps a 640 px minimum and the studio page scrolls, which places the name-drop below the editor. Changing the font while the text area still held an uncommitted newline called `replaceChildren` from that field’s blur and threw. `edit` now applies that nested text change without starting a second render inside the first, then renders once more. The browser proof was run again after both corrections and passed.

The partner stitch-out study was not run. No machine, hoop, fabric, stabiliser, needle, thread, photograph, or readability result was recorded. Software milestones do not change that status. This is not a minimum lettering size and not machine certification.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | The open badge set the title to `Name drop badge — EmbroideryOS`. The page kept one `h1`. TwinLab’s heading stayed “See every stitch. Understand every move.” The unknown-page title is `Page not found — EmbroideryOS`. |
| Q03 | PASS | The studio and TwinLab pages keep their existing local descriptions and `noindex`. The layout policy does not add a public page. |
| Q04 | N/A | Icon files were not changed. `GET /favicon.svg` on 4186 returned 200 `image/svg+xml`. |
| Q08 | PASS | `GET /not-a-studio-page` on 4186 returned 404 with the branded page. Workflow test `unknown_page_is_a_branded_404` passed. |
| Q09 | PASS | Generate completed the two-line badge. The 20-row batch finished completed. Spacing change and cancellation showed a not-current proof and a not-current completion manifest. The late response did not restore either. |
| Q10 | PASS | Rejected controls and a failing last row keep the source and say the plan was not changed. The clearance failure names the lines and the next step, and says it is not a physical measurement. |
| Q11 | PASS | One `h1` remained. The name drop section is an `h2`. The text area is labelled “Lettering text”. |
| Q12 | PASS | Show, item id, and text controls expose `aria-label`s. The alignment control has a visible label. The preview and proof draw returned stitches. |
| Q14 | PASS | The exercised Chrome flows reported no page errors and no console errors. |
| Q15 | PASS | `node --check` passed. Workflow test `the_browser_does_not_plan_stitches` scanned `web/studio.js`, `web/batch.js`, and `web/lettering.js` and passed. |
| Q16 | N/A | Local debug binaries only. No public source map was published. |
| Q17 | PASS | `web/studio.js` is 81897 bytes, inside the 80 KiB budget. `web/batch.js` is 44820 bytes, inside the 48 KiB budget. `web/studio.css` is 13665 bytes, inside the 40 KiB budget. |
| Q18 | PASS | The multiline batch was completed at 1440 px. A fresh 390 px page generated the two-line centre badge. TwinLab was used at 390 px and 1440 px. Overflow stayed within one pixel at the checked widths. |
| Q19 | PASS | The new controls use the existing field, choice, and panel styles. The name-drop table and placement grid keep their spacing. |
| Q20 | PASS | Enter inserts a line. Control or Command Enter generates. CSV import, centre alignment, spacing, save, reopen, the selected row’s project, DST, replay, and PDF, undo, redo, cancel, and the late response were exercised. |

Curved baselines, wrapping, shrinking, font import, monograms, and new machine formats remain outside this increment. The cap height is a software size. The inter-line check is a software extent check. The 24-sample stitch-out was not run. Safari and Firefox were not run. `tests/twinlab/native.mjs` was not re-run. WASM remains blocked. Register rows stay `NOT_RUN`. This layout does not accept F06. Release-ready claim: no.

## Compare navigation, 12 October 2026

Studio Compare no longer navigates to port 4175. One meta destination, `http://127.0.0.1:4181/`, is the only permitted Compare service. Both the sidebar Compare control and Production → Open Compare workspace use it. A click sends the selected proved name-drop replay and its `embroideryos-name-drop-binding/0.1` binding. It does not open Compare’s sample design, and it does not replace the open Compare design. Port 4175 stayed down and was not started. Nothing was committed or pushed. Creative Studio C1 was not implemented in this change.

The proof used the already running studio on `http://127.0.0.1:4186/` and Compare on `http://127.0.0.1:4181/`. Neither process was restarted. HTML and script are read from disk. `web/studio.js` stayed 81897 bytes. `web/batch.js` is 46642 bytes, inside the 48 KiB budget. `web/studio.css` is 13751 bytes, inside the 40 KiB budget. `apps/twinlab/public/app.mjs` is 61888 bytes. `cargo test -p embroidery-studio --test workflow the_browser_does_not_plan_stitches --offline` passed. `tests/studio/shell_browser.py` was updated for the new destination and was not run, because that script writes `evidence/m2-g1/`.

Chrome proved one row, `CMP-001`, text `ALPINE 08`, on the unchanged name-drop badge. The handoff kept snapshot `ea05ca45ef521e331e53ec109df9d8d9c8896781a68b2cccd79997010b79c768`, project revision `3a2eaa44582645c538612785dec6c689b9e2a6430629a99586510f11b46e52a4`, DST SHA-256 `6569b2e57cc4bc3704cfe2939dbbe845be09f176b908985cfe801961030fd135`, PDF SHA-256 `b73d9541ad6cd157b45d1c1c5ea16068185b3100b89911df7809e33665ee717f`, and replay SHA-256 `a69772f338b7bbf09813ea979deb786f9f7b404743e36dece43204d2438ef751`. Compare showed that item and kept the open design name `Alpine mark` at schema version 1. `machine_release_allowed` and `physical_qualification` stayed false. A binding whose snapshot id was changed to `stale-snapshot-id` was refused and did not replace that item. Destinations `http://127.0.0.1:4175/` and `https://127.0.0.1:4181/` were refused and nothing was sent. Aborting requests to 4181 showed “Compare is unavailable”, kept the studio project, and showed Retry. After the route was restored, Retry opened Compare with the same binding. The Production link was then activated with Enter and opened the same item. The studio project name stayed `Name drop badge`. The unavailable probe produced one Chrome `net::ERR_FAILED` console line for the aborted request. There were no page errors. Record: `evidence/compare-handoff/result.json`, SHA-256 `298c5b06d968b8aaac917d179ba9496fbd46b64e12dccba74b736b9f755938fe`.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | The studio title stayed `Name drop badge — EmbroideryOS` with one `h1`. Compare stayed `EmbroideryOS · Compare` and its heading stayed “See every stitch. Understand every move.” |
| Q03 | PASS | The existing local descriptions and `noindex` stayed. The new meta names only the loopback Compare destination. |
| Q04 | N/A | Icon files were not changed. |
| Q08 | N/A | The 404 route was not changed. |
| Q09 | PASS | The unavailable probe ended in a visible Retry control. Retry then opened the proved item. |
| Q10 | PASS | A missing proof, a forbidden destination, an unavailable service, and a stale binding each say what failed and that the studio design was kept. |
| Q11 | PASS | The production view kept its `h2`. The page kept one `h1`. |
| Q12 | PASS | Compare, Open Compare workspace, and Retry Compare have visible text. No new informative image was added. |
| Q14 | PASS | The exercised Chrome flow had no page errors. The one console error was the intentional aborted request to 4181. |
| Q15 | PASS | `node --check` passed for `web/batch.js`, `web/studio.js`, and `apps/twinlab/public/app.mjs`. The workflow test found no `console.log` or `console.debug` in `web/batch.js`. |
| Q16 | N/A | Local debug binaries only. No public source map was published. |
| Q17 | PASS | `web/studio.js` remains 81897 bytes. `web/batch.js` is 46642 bytes, inside 48 KiB. `web/studio.css` is 13751 bytes, inside 40 KiB. |
| Q18 | PASS | The proved handoff was checked at 1440. At 768 and 390 the Production link remained reachable, the status wrapped inside the card, and overflow stayed within one pixel. |
| Q19 | PASS | The status line, alert, and button use the existing type, colour, radius, and spacing. |
| Q20 | PASS | The sidebar click, Retry, and Production Enter were exercised. Forbidden and unavailable destinations did not open a sample. |

Safari and Firefox were not run. No machine was connected. This does not accept a register row. Release-ready claim: no.

## Creative image jobs, 12 October 2026

C1a adds one Creative Director and one image-generation adapter. A creative job may finish with raster artwork alone. It does not require a DST or a name-drop proof. The proved-item Compare handoff still requires those proved artifacts. Artwork review was not added to that handoff.

Live xAI generation did not run. `XAI_API_KEY` was absent. `EMBROIDERY_CREATIVE_LIVE` was absent. The proof-store policy has `transmit_permitted` false and `remaining_calls` 0. No budget was invented and no key was copied from another process. No prompt was sent. The configured image model recorded for a future authorised call is `grok-imagine-image-2.0`. The geometry model `grok-4.7` is rejected for this operation. Image editing was not called. The four authorised briefs are BLOCKED in `evidence/creative-c1a/live-cases.json`. The checkerboard stored for the robot sentence is a local test raster. It is not an illustration of that robot, and it is not a successful stand-in for the blocked briefs.

The local lifecycle was exercised on a new studio, `http://127.0.0.1:4192/`, PID 26797, binary SHA-256 `9a59fd354ee9b3ae5b98679934ae44c30aba429b9f14db5d376edc4e8e7a325d`. The store is `evidence/creative-c1a/store` with provider `test_image` and model `test-raster`. `cargo test -p embroidery-studio --lib creative_ -- --test-threads=1` passed, 2 tests. The isolation test covers 23 named cases: artwork without a DST, a candidate that does not apply itself, duplicate submission, select and apply leaving `fixtures/projects/08_club_mark.json` unchanged, a revision change blocking select, a second success that does not replace the current master, changed references, a portrait with no reference, name-drop material refused, cancel before submit, cancel after submit keeping a late image unselected, an uncertain timeout that is not retried, malformed output that is not stored and not replaced with a fixture, a checkerboard that is not treated as transparent, alpha read from the file, missing credentials, the unsupported geometry model, an exhausted budget, transmit refused, a key and budget that still need the live switch, an unconfigured provider, restart of the same bytes with an in-flight job marked uncertain, and three distinct local style rasters. `creative_http_reopens_the_same_artwork_bytes` passed. `python -m embroidery_ai.images self-check` returned `{"ok": true}` and made no network call. `cargo test -p embroidery-studio --test workflow the_browser_does_not_plan_stitches --offline` passed. `node --check` passed for `web/creative.js` and `apps/twinlab/public/app.mjs`.

Chrome on 4192 opened Creative Director, submitted the robot sentence at 64 by 64 with the cartoon profile, saved the candidate, downloaded it, and reopened it. The file is a PNG of 12420 bytes, SHA-256 `97ec83a5205e3283b546991e6573b32aaf4234fc439fd22e52cbddf09e358422`. The caption says it is a local test raster, not a live provider image, with no alpha channel, and that it is raster artwork rather than vector geometry, layered art, or a stitch plan. The open project stayed `Alpine Club` and its JSON did not change. A portrait with no file was refused with “No likeness was invented”. An 8 by 8 RGB block that is not a likeness was refused with “Reference editing was not called” and “No likeness was invented”. Those two refusals are the only console errors, both HTTP 422. There were no page errors. After only the 4192 process was stopped and started again on the same store, both stored artifact URLs downloaded that same hash. Record: `evidence/creative-c1a/browser.json` and `evidence/creative-c1a/restart.json`. Screenshots: `evidence/creative-c1a/creative-mobile.png`, `creative-tablet.png`, and `creative-desktop.png`.

Compare was re-checked without restarting 4181 and without starting 4175. The first attempt against the already running 4186 process failed the console check because that process’s binary does not serve `/creative.js` and returned HTML. The handoff assertions in that attempt had already passed. The passing run used the current binary on 4192 and Compare on `http://127.0.0.1:4181/`. The test file’s default studio URL remains 4186. Sidebar and Production both opened CMP-001. Compare kept the editable design name `Alpine mark` and identified the imported review of `Name drop badge`. The status says the DST and PDF were not generated by the open design. The displayed DST hash stayed `6569b2e57cc4bc3704cfe2939dbbe845be09f176b908985cfe801961030fd135`. A stale snapshot id was refused. Unavailable and Retry still worked. `evidence/compare-handoff/result.json` remains SHA-256 `298c5b06d968b8aaac917d179ba9496fbd46b64e12dccba74b736b9f755938fe` because the recorded binding fields did not change.

HEAD remained `23bd144ac42ef4a6d3cc10088a5ecdd5ef4556d1`. Nothing was committed or pushed. The workbook SHA-256 remained `d1cd2147c9a282d36a9e0ff6fc812bf9f5a775bb2bac6cf92360c7f82ca32128` (54782 bytes). Both coverage JSON files remained `829295f1095ff5080b454e6b2a8a0893da531d300ad07e790e7383caac687db6` (32594 bytes each). The badge file remained `519cdab0c3e4c4368c9f2eb60d32df422259533d19561fc6bfc2f7cfd2f8e5ba`. The saved L3 bundle remained `54fbc0099b698ac6aa451fff22e5919f636cc2d006d28b163ff7563e31f47159` (408143 bytes). `web/studio.js` stayed 81897 bytes. `web/batch.js` stayed 46642 bytes. `web/creative.js` is 12866 bytes, inside the 32 KiB check. `web/studio.css` is 14807 bytes, inside 40 KiB. `apps/twinlab/public/app.mjs` is now 63228 bytes because the imported-review sentences were added. `machine_release_allowed` stayed false. Acceptance stays 0 of 210. F01–F11 stay `NOT_RUN`. This does not accept F06. Port 4175 stayed down. Listeners left running and not restarted: 4173, 4176, 4181, 4186, and 4187.

| ID | Result | Evidence |
|---|---|---|
| Q01, Q05, Q06, Q07, Q13 | N/A | Private loopback only. No public origin was invented or published. |
| Q02 | PASS | With Creative Director open, the title was `Creative Director — EmbroideryOS`. The page kept one `h1`. The creative heading is an `h2`. Compare’s heading was unchanged. |
| Q03 | PASS | The studio and Compare pages keep their existing local descriptions and `noindex`. The creative panel does not add a public description. |
| Q04 | N/A | Icon files were not changed. `GET /favicon.svg` on 4192 returned 200 `image/svg+xml`. |
| Q08 | N/A | The document 404 route was not changed. `GET /not-a-studio-page` on 4192 still returned 404 HTML. |
| Q09 | PASS | The local raster reached a ready state, save, download, and reopen. The two portrait refusals returned to an explicit error. A separate slow-loading screenshot was not captured because the local raster returned inside the wait. |
| Q10 | PASS | Portrait, reference, credential, model, budget, transmit, timeout, cancel, and malformed cases name the failure and leave the current artwork and embroidery design unchanged. The browser showed the two portrait sentences. |
| Q11 | PASS | One `h1` remained. Creative Director is an `h2`. |
| Q12 | PASS | The candidate image alt starts `Raster artwork candidate:`. Generate, Save, Download, Reopen, and Close have visible names. |
| Q14 | PASS | The creative Chrome run had no page errors. Its two console errors were the expected portrait 422 responses. The passing Compare run had no unexpected console errors. |
| Q15 | PASS | `web/creative.js` has no `console.log` or `console.debug`. `node --check` passed. The workflow test passed. |
| Q16 | N/A | Local debug binaries only. No public source map was published. |
| Q17 | PASS | `web/studio.js` remains 81897 bytes. `web/batch.js` remains 46642 bytes. `web/creative.js` is 12866 bytes. `web/studio.css` is 14807 bytes. |
| Q18 | PASS | The creative task was completed at 390, 768, and 1440. The save control stayed at least 40 px wide and overflow stayed within one pixel. |
| Q19 | PASS | The dialog uses the existing card, field, button, type, colour, and radius. The image sits in the card with the existing border colour. |
| Q20 | PASS | Generate, save, download, reopen, close, and both portrait refusals were exercised. Compare’s sidebar click, Retry, and Production Enter still opened the proved item. |

Safari and Firefox were not run. No machine was connected. Brief adherence, character consistency, and protected-region evaluation were not run. Release-ready claim: no.

## Live robot preflight, 12 October 2026

The original-robot live call was not submitted. The checkerboard remains a `test_image` asset. The four blocked records in `evidence/creative-c1a/live-cases.json` are unchanged, SHA-256 `60e6b0cd09821ca71ed45c7b203df4de43c9f5655fea5ae30a73bfb5d6e1343a`. The proof-store policy is unchanged, SHA-256 `c846ceb9a9a0bb34b54784c709594a6cde569a0c0f047043ece09bf550c2255a`.

A separate store, `evidence/creative-live-robot/store`, names provider `xai_images`, operation `generate`, and model `grok-imagine-image-2.0`. After the one-call authorisation, that store records `remaining_calls` 1, `transmit_permitted` true, and `spend_ceiling_usd` 1.0. `test_script` stays empty. No studio process was started against that store, and `EMBROIDERY_CREATIVE_LIVE` was not set. The switch is enabled only when its trimmed value is exactly `1`. `XAI_API_KEY` is absent from this process, the login shell, the repository, and the Keychain items `embroideryos-creative-xai` and `XAI_API_KEY`. The key was not copied from another process.

The job service now refuses a live xAI job that has no positive `spend_ceiling_usd`, with code `spend_ceiling_required`, before a provider call and without reducing `remaining_calls`. A live policy that still names a test script fails with `test_fallback_disabled`. `cargo test -p embroidery-studio --lib creative_ -- --test-threads=1` passed after that check, 2 tests. `python -m embroidery_ai.images self-check` returned `{"ok": true}` and made no network call. The on-disk studio binary is SHA-256 `3ea21b7d0b781a76ff8eace94eeffbca1350c455cbb664c23a11ef6bc2f2d135`. The studio already listening on 4192 was not restarted. Port 4186 was not restarted. Port 4175 stayed down.

The prepared request, still unsent, is the original robot sentence, cartoon direction, no reference, one image, `response_format` `b64_json`, aspect ratio `1:1`, resolution `1k`, quality `low`, and no deferred polling. Image editing was not prepared. A synchronous response’s `request_id` and `usage`, when the provider supplies them, are now retained on the job. The historical test-raster label still comes from the stored job’s provider id. Record: `evidence/creative-live-robot/preflight.json`. The user then authorised one original-robot generation, with no reference upload and no automatic retry, and a 1.00 USD ceiling. That permission is recorded on the isolated store. The call was not submitted because the server process has no xAI key. The live case stays BLOCKED on missing credentials. Nothing was committed or pushed. `machine_release_allowed` stays false.

## OpenRouter image route, 12 October 2026

The same one-call authorisation now uses OpenRouter instead of a direct xAI request. The isolated policy provider is `openrouter_images`, the model is `x-ai/grok-imagine-image-2.0`, and `direct_xai_authorisation` is `superseded_unused`. `remaining_calls` stays 1 and `spend_ceiling_usd` stays 1.0. There is no second allowance. The proof-store policy and `evidence/creative-c1a/live-cases.json` are unchanged, SHA-256 `c846ceb9a9a0bb34b54784c709594a6cde569a0c0f047043ece09bf550c2255a` and `60e6b0cd09821ca71ed45c7b203df4de43c9f5655fea5ae30a73bfb5d6e1343a`.

The public OpenRouter endpoint record for that model was saved as `evidence/creative-live-robot/endpoint.json`, SHA-256 `d25aa26ffabbca9ca05815ec4f68a9293ec621eacad93f4c982bf90ec1c51d9b`. It lists one upstream endpoint, provider tag `xai`, resolution `1K` or `2K`, quality `low` or `medium`, and `n` limited to 1. `output_format` is absent, so the adapter does not send it. The prepared body is one image, resolution `1K`, quality `low`, aspect ratio `1:1`, and `provider.only` `["xai"]` with `allow_fallbacks` false. The listed `low_1k` output price is 0.04 USD, which is inside the 1.00 USD ceiling. `python -m embroidery_ai.images self-check` returned `{"ok": true}` and made no generation call. `cargo test -p embroidery-studio --lib creative_ -- --test-threads=1` passed, 2 tests. Those tests refuse an OpenRouter job that only has `XAI_API_KEY`, refuse spending the superseded direct route, refuse an estimate above the ceiling, and refuse a process provider that does not match the policy. None of them decrement `remaining_calls`.

`OPENROUTER_API_KEY` is absent. `EMBROIDERY_CREATIVE_LIVE` is absent. No image was requested. This checkout has no Vercel project, so no secret was added and nothing was deployed. The studio on 4192 was not restarted. Port 4175 stayed down. `web/creative.js` is 12908 bytes. The transmit control is shown for `openrouter_images` as well as `xai_images`; that checkbox was not exercised in a browser in this pass. `machine_release_allowed` stays false. Nothing was committed or pushed. The live robot case remains BLOCKED until the OpenRouter key is present in the process that will make the one authorised call.
