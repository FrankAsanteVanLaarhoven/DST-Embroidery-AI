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
