# TwinLab M1 — native export/replay integration candidate

**Status: native path executed locally on 11 October 2026. Not a release and not committed.** The preparation package could not run Rust or a localhost browser. This checkout did. `bash scripts/check-twinlab-m1.sh` passed the workspace Rust tests, Clippy, pytest, the 51 transport tests and the 11 native tests. Port 4173 was already held by `embroidery-studio`, so the served browser run used `PORT=4175`. Chrome completed 31 native checks with the WebGL renderer, including a live recompile after an edit. A separate run changed Alpine mark fill spacing from 0.6 mm to 1.4 mm, downloaded the new DST, and confirmed the file hash. The original mountain outline self-intersected when that browser check set the first control point to x = 1. The sample outline is now a simple five-point field, so that edit stays inside the generator. This is not machine acceptance, a calibrated twin, or an ISO certificate.

## Repository and scope

Target: `FrankAsanteVanLaarhoven/DST-Embroidery-AI`.
Inspected base: `98303601416a3b0d68eae37e939031732bd77c45`.
This is an additive integration. Existing `core`, `dst`, `simulate` and `ffi` source files are not replaced. A new `embroidery-twin` crate calls the existing public Rust APIs. The workspace and local-package lock entry are the only changes to existing files. Registry package versions/checksums are preserved.

This milestone contains one editable design workspace, one generic six-needle flat-frame rig, a local Rust worker bridge, an exact-DST export route, byte-indexed replay, and two linked views. It does not contain the previous standalone EmbroideryOS platform, multi-tenant production management, AI image generation, segmentation, CAD import, general machine-file import, OEM profiles, machine control or physical calibration.

## Start the real implementation

Use a checkout of the target repository with the supplied patch applied. Do not treat the review package's `overlay/` directory as a complete standalone copy of the Rust repository.

Prerequisites: the repository's Rust toolchain (Rust 1.83 minimum; proposed CI uses 1.90.0), Node.js 22 or newer. The Rust dependencies must be available. No npm installation, model provider, GPU, machine connection or external web service is required by the application.

```sh
cargo build --locked -p embroidery-twin
node apps/twinlab/server.mjs
```

Open `http://127.0.0.1:4173/`, or the port in `PORT` when 4173 is already in use. Missing Rust is a blocking error, never a JavaScript stitch-generation fallback. The 11 October 2026 local run used port 4175 because the design studio was already bound to 4173.

The default native binary is `target/debug/embroidery-twin` (`.exe` on Windows). Set `EMBROIDERY_TWIN_BIN` to an explicitly selected release binary when needed. Set `PORT` to another free local port. Restart the service after replacing the engine binary. The service binds only to loopback; it is not a public deployment server.

## Demonstration script

1. Open the bundled **Alpine mark** native project. It contains fill, satin and running objects.
2. Inspect its points and parameters. Drag a point, or edit numeric coordinates. Change fill spacing from its current value to 1.4 mm. The revision advances and all old replay/export controls become unavailable immediately.
3. After native compilation completes, confirm a new SHA-256-bound artifact. The app verifies the bytes received over HTTP before enabling replay. A late result for an older revision is ignored.
4. Switch among split, 2D and 3D views. Play, pause, step, rewind and scrub. Both views receive the same sampled execution state. Camera changes do not change the execution clock.
5. Observe the frame moving opposite to design XY while the needle remains fixed in XY. During the displayed sewing phase, the frame stops before the needle descends. These are illustrative kinematics, not measured machine timing.
6. Download DST and the replay evidence JSON. The download route returns the saved artifact bytes; it does not recompile the current project on download.
7. Change the design again. The old artifact must not become downloadable as the current design. Undo restores geometry as a new revision and therefore still requires recompilation.

Record the engine binary hash, source revision, project, exact DST, replay JSON and native browser results before claiming this sequence has passed.

## Exact data path

```text
Browser native project JSON
    -> POST /api/compile with session token
    -> new Rust embroidery-twin process, JSON via stdin
    -> strict bounded design/profile validation
    -> embroidery-core running/satin/fill generators
    -> embroidery-core validation and nominal hoop checks
    -> embroidery-dst::compile_to_vec
    -> exact exported bytes
    -> embroidery-dst raw decoder + reader with trim collapse disabled
    -> record-indexed execution trace, including end/control/jump records
    -> SHA-256-bound in-memory artifact and exact byte download
    -> one playback clock -> 2D renderer + 3D renderer
```

The browser does not generate stitches, encode DST or decode its binary record semantics. Node checks transport/provenance and launches Rust; it is not another embroidery engine. This milestone uses a native process bridge, not a claimed verified WASM build.

The Rust trace checks the end marker, complete records, canonical record encodings, record count and supported command subset. Initial positioning and compiler-split jumps appear in replay because the decoder consumes the output bytes. Every event retains its exact raw bytes, file byte offset and integer 0.1 mm coordinates.

The reader's default three-jump trim inference is deliberately disabled. Three jumps remain three jumps. A manufacturer's real interpretation of such a sequence is unknown here. This is export-faithful raw-command rehearsal, not proof of device-level trim semantics.

Source object IDs and RGB/needle assignments live in a sidecar. They are NOT represented as if standard DST stores that design information. Stable compiled prefixes are checked before deriving object spans. Needle-down records cannot recover the original satin/fill classification from DST alone.

## Coordinates and timing

Design and decoded coordinates use X right, Y down, millimetres (trace integers represent 0.1 mm). The 3D scene uses metres, Y up, with design XY mapped to scene XZ. The moving frame offset is `[-x_mm/1000, 0, -y_mm/1000]`, so a design point is beneath the fixed needle at a completed movement.

For each illustrated sewing cycle, frame movement occupies the first 45%; the needle then descends with the frame held, contacts at 70%, rises by 95%, and remains raised before the next cycle. Jumps never generate a penetration. Display speed scales playback time, not trace duration.

The timing constants are **unmeasured assumptions**: 600 stitches/minute, 100 mm/s nominal jump movement, a minimum 20 ms jump and 1 s colour-change pause. These do not model acceleration limits, mechanical clearances, thread tension, material deformation, colour-change mechanisms or controller scheduling. They must not be presented as predicted production runtimes.

## UI and rendering

The interface has an original content-first, system-font layout with contextual controls, light/dark appearance, keyboard actions and mobile reflow. No Apple assets or proprietary font files are bundled. No complete WCAG audit was performed.

The original generic machine is constructed from 3D triangle meshes, not a photograph, video or manufacturer model. Orbit, zoom and camera presets operate on that scene. WebGL is the preferred backend. When unavailable, a software renderer projects the same meshes and transforms with approximate depth sorting. The fallback can show occlusion/edge artifacts; it is not a collision or physical-validation engine. The preparation browser had no WebGL. The 11 October 2026 Chrome run reported renderer `webgl`.

The 3D rig has six nominal needle positions but is not a claim about any brand or model. A virtual nominal 120 x 120 mm sewing envelope is enforced. No OEM CAD, attachment-specific constraints or machine instances have been qualified.

## Revision and artifact integrity

The artifact identity combines design, profile, trace, engine binary hash, palette and protocol. DST hash alone is insufficient: colour metadata or a design revision can change without changing the standard DST bytes. Exact source/byte associations are retained across downloads; the server keeps at most eight artifact entries.

These hashes detect accidental mismatch. They are not signatures, authenticated approvals, attestation of a compiler's trustworthiness, tamper-proof storage, or production release authority. Both server and local browser are trusted components in this demonstration. The existing repository's older approval prototypes are not silently promoted to production security.

## Boundaries and resource limits

One local user; loopback-only HTTP; exact Host/Origin checks; per-process session token for compile; bounded request and response sizes; static asset allowlist; two concurrent workers; 30-second worker timeout; eight retained artifacts. No machine-transfer or actuation endpoint exists.

Admitted design: 1–16 objects, 2–256 points per object, up to six distinct thread RGBs, running/satin/fill only. Point coordinates and generated sewing geometry must fit the generic envelope. Fill admits simple single-ring polygons and rejects self-intersections. Generation is bounded, but this is not a hardened hostile-input sandbox or public multi-user service.

The project format is an explicit M1 schema. It is not automatically interchangeable with the previous reference application's EXF document. Supported import is this native JSON only. Unsupported images, SVG, CAD and embroidery formats should not be described as implemented.

## Testing: keep the two modes separate

Local transport/playback checks, with a pinned repository fixture and no Rust generation:

```sh
node --test tests/twinlab/unit.mjs
```

Optional, explicitly labelled fixture UI for inspection without Rust:

```sh
node tests/twinlab/fixture-server.mjs
# Open http://127.0.0.1:4174/
```

That server replays one exact repository square and refuses changed geometry. It is not an alternate compiler. The fixture contains three sewing movements and an end record; expected decoded positions are recorded test data, not a second implementation of the decoder.

Prepare optional test tools in a normal network-enabled development environment:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r tests/twinlab/requirements.txt
.venv/bin/python -m playwright install chromium
```

The preparation environment did not install these pins or run the native baseline. This checkout installed Playwright 1.57.0 into the existing virtualenv and ran the gate with the system Chrome binary. The requirements file is still not a complete transitive hash lock.

Run the genuine acceptance gate after building and reviewing:

```sh
cargo fmt --all
bash scripts/check-twinlab-m1.sh
```

The script does not automatically install dependencies or accept fixture mode. It runs baseline Rust/Python tests, Clippy, builds the real engine, executes transport and native acceptance tests, then exercises the actual served application through Playwright. `cargo fmt --all` was run in this checkout before the gate. The workflow file is in the tree and has not been executed on GitHub. Adding it does not make the check mandatory.

A restricted browser can run the narrower check:

```sh
.venv/bin/python tests/twinlab/browser.py --offline-fixture
```

This mode installs explicit test HTTP and SHA adapters and renders production UI modules through `set_content`. It records itself as `offline-fixture`; it is **not** a substitute for the real native browser gate. Never publish its passing count as evidence that edited geometry was compiled in Rust.

## Remaining acceptance gates

Before merge: native build, all existing tests, the new 17 Rust tests (one includes 100 parameter cases), 11 native transport/integration tests, and a successful `browser.py --native` run. Correct any discovered failures and rerun the exact source being merged. Do not reuse this package's fixture report as the final gate.

Before a broader demo release: evaluate WebGL on a real browser/GPU, verify Safari/Firefox and keyboard/screen-reader flows, expand the corpus beyond a square, exercise admitted upper input limits and record performance on named hardware. None of those results is claimed here.

Before physical use: independent machine acceptance, properly scoped calibration, manufacturer procedures, authenticated release authority and actual stitch-out quality evidence. This M1 makes no ISO certification, universal compatibility, calibrated twin, machine safety or superior embroidery-quality claim.

## Compare workspace

The same loopback page now has Original, Editable design, Machine file, PDF proof, and Machine twin. Rust remains the only stitch and DST authority. Node stores the artifact and asks `tools/compat_compare/proof.py` to draw a PDF from that stored bundle plus an attached original. The utility checks SHA-256 and raw record bytes. A matching hash does not independently prove that the reported coordinates were decoded correctly; that check stays with the Rust decoder that produced the artifact.

Attaching or moving the original invalidates the proof and leaves the compiled DST in place. Editing the design invalidates the DST, the replay, and the proof together. Download of the current PDF stays disabled until the binding matches the open revision. Approval and machine release stay unavailable. There is no quality percentage.

The supplied compatibility contract and the 54-row register are in `docs/compatibility-and-compare.md` and `contracts/compatibility-coverage.json`. Recorded requirements are not passed requirements. The local proof command is:

```sh
.venv/bin/python -m unittest tests.compat_compare.test_compare
BROWSER_EXECUTABLE="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  TWINLAB_URL=http://127.0.0.1:4175/ .venv/bin/python tests/twinlab/compare_browser.py
```

The engineering square and the 14 mm reference image are fixtures. They are not customer artwork. A design that does not fit the 1:1 PDF panel is reported and is not shrunk. The 100 mm bar is a print instruction. It is not a measured paper calibration.
