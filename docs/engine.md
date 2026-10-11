# Engine

The production kernel is Rust. Python proposes regions and checks files. It does not plan stitches, offset geometry for export, or compile DST.

```text
image or vector
  → segmentation
  → DigitisingProposal
  → Rust generators
  → validate
  → risk score
  → human approval
  → DST
```

An agent may emit `DigitisingProposal` only. The model forbids extra fields, so a stitch array or a raw DST blob cannot hide inside the proposal. `Engine.compile_dst` rejects a document that has no `stitches` array and tells the caller to run the generators first.

## Crates

| Crate | Role |
|---|---|
| `embroidery-core` | Points, stitch plan, geometry, running/satin/fill generators, validation, native project document. |
| `embroidery-dst` | Ternary codec, DST compiler, DST reader. |
| `embroidery-simulate` | Length-density risk score. Not a fabric simulation. |
| `embroidery-ffi` | C ABI and an optional `wasm` feature. |
| `embroidery-motion` | Execution trace decoded from compiled DST, and the generic fixed-needle rig. |
| `embroidery-studio` | Loopback HTTP studio and TwinLab. It calls the Rust generators and serves `web/`. |

`unsafe_code` is forbidden in the core, DST, and simulate crates. The FFI crate is the only place that uses `unsafe`, and it is limited to the C boundary. The workspace is `publish = false` and has no license field.

Coordinates are millimetres, Y positive downward. See [format.md](format.md) for the file encoding.

## Generators

`generate_running` resamples a polyline to `max_stitch_length_mm` (default 2.5 mm) and commits the batch only if every sewing stitch fits in one DST record (±121 counts on each axis, 12.1 mm). On failure the plan is unchanged.

`generate_satin` samples the centreline by arc length. The left normal is the 90° CCW normal in Y-down coordinates, so a centreline moving in +X has its left rail toward +Y. Nominal rails sit at half the column width and are used for the edge-run underlay. Satin rails sit at half the width plus pull compensation. The column is a zigzag `L0 → R0 → L1 → R1`, and the connector is a satin stitch. A non-positive underlay spacing skips underlay. The default width is 8 mm. A width that cannot be sewn as one DST record, such as 20 mm, is an error and does not change the plan.

`generate_fill` scan-converts one closed ring. A duplicate closing point is dropped. Edge crossings use a half-open interval so a vertex is counted once. Spans are paired even-odd. An odd intersection count is an error. Rows are inset by half the spacing and alternate direction. A positive `underlay_spacing_mm` adds a perpendicular running underlay. Travel between spans is a jump. The default angle is 45°, row spacing 0.4 mm, maximum stitch 5 mm.

`offset_polyline` is a naive normal shift. Satin does not use it. It fails on self-intersections, sharp corners, holes, and narrow regions. A production offset belongs in a clipper-style polygon library, which is not in this tree.

## Validation and risk

`validate(plan, max_stitch_mm, max_jump_mm)` blocks a strict compile when the plan is empty, a coordinate is not finite, a sewing stitch exceeds the length limit, or a sewing stitch exceeds ±121 counts on an axis. Jumps may be longer than one record. They are split without adding needle penetrations. A start away from the origin is a warning because the compiler inserts a jump from `(0, 0)`.

`score` rasterises sewing length into 5 mm cells. Density above 12 mm of stitch per mm² is a flag. A sewing stitch longer than 12.1 mm is an error. A jump longer than 500 mm is a warning. The report keeps at most 24 flags and sets `truncated` when more were dropped. This does not model puckering, registration, or thread breaks.

## Native project

An editable design is an `embroidery-project` document, format version `"1"`. It holds the hoop, stable object ids, geometry in millimetres, digitising parameters, and an optional geometry review. It does not hold stitch coordinates. `generate_project` resolves each object, checks the capability manifest, and calls `generate_running`, `generate_satin`, or `generate_fill`.

The revision is the SHA-256 of the canonical JSON with the `review` field removed. A geometry review stores that revision, a reviewer label, and scope `geometry`. Editing a parameter changes the revision, so the previous review no longer matches. `machine_release_allowed` is always false. Engineering DST export is allowed only when validation passes. Reviewed export also requires a matching geometry review. Those are local labels. They are not authenticated production authority.

The capability manifest marks running, satin, and fill as available. Motif, applique, zigzag underlay, lattice underlay, and machine release are planned. Satin underlay is available only as `centre_run` and `edge_run` together. Fill underlay is available only as `perpendicular_run`. Running has no underlay. An unsupported request fails with a message that names the capability as unavailable. Generation does not substitute another stitch type.

A file whose `format` is not `embroidery-project` is rejected. The reference EXF document is in that group until a tested adapter exists.

`fixtures/projects/` holds twelve documents: running, satin, fill, a two-colour club mark, and invalid motif, applique, zigzag, and over-wide satin cases.

## Design studio

`cargo run -p embroidery-studio` serves `http://127.0.0.1:4173`. The bind address is loopback. The browser posts the project to `/api/generate`, `/api/review`, and `/api/verify`. It draws the stitch plan from the response. `web/studio.js` has no scanline fill, stitch resampling, or DST encoder.

Verify reads the exported DST with the Rust reader. Needle-down records come back as running stitches, because DST does not store satin or fill. A successful read is a byte-level check. It is not machine acceptance.

## Export rehearsal

`/twinlab` and `POST /api/rehearse` run the generated plan through `compile_to_vec_with_links`, then decode those exact DST bytes. Each body record keeps a sidecar `object_id` and plan-stitch index. A trim signature becomes one trace event and still counts as three file records. Needle-down, jump, colour-change, trim, and encoded-record counts stay separate. The browser does not plan stitches or encode DST.

The generic profile is `generic-flat-xy-frame`: one fixed needle, not a measured needle bar. Travel moves the hoop by the negated design point. Penetration holds the hoop and moves the needle tip from the declared 14 mm raised position to the fabric and back. The development timing assumptions are 600 stitches per minute, 200 mm/s jumps, 2 s colour changes, and 0.4 s trims. They are not a manufacturer rating. Playback speed does not change `estimated_execution_seconds`.

Design space is millimetres, Y down. The scene is metres, Y up. `design_mm_to_scene_m` maps design Y to scene Z and divides by 1000. There is no extra scale.

Geometry for this demo is `declared`. Timing is `assumed_demo`. Collision, file acceptance, and fabric prediction stay unknown. Telemetry, cap attachment, and machine release stay unavailable. A thread-break control is an `injected_scenario` and is not an observed machine fault. The persistent qualification is “Virtual demo · Generic machine · No hardware connected · Physical quality not validated.”

This rehearsal is not a calibrated process model, a connected twin, a vendor machine pack, or a physical sew-out.

## FFI

`crates/ffi/include/embroidery.h` declares:

- `embroidery_engine_version`
- `embroidery_compile_dst_json`
- `embroidery_validate_json`
- `embroidery_score_json`
- `embroidery_generate_project_json`
- `embroidery_capabilities_json`
- `embroidery_free`

Return codes are 0 for success, 1 for a UTF-8 error message with no trailing NUL, and 2 for a null argument. Every non-null buffer is released with `embroidery_free(ptr, len, cap)`. The Python package `embroidery_ai` loads `target/debug/libembroidery_ffi.dylib` (or `EMBROIDERY_FFI`) and frees those buffers.

JSON stitch kinds are snake_case: `running`, `satin`, `fill`, `motif`, `applique`, `jump`, `trim`, `color_change`.

The `wasm` feature exports `compileDst`, `validatePlan`, and `scorePlan` through wasm-bindgen. It is not a default feature. The `wasm32-unknown-unknown` target is not installed in this environment, so the WASM build and a native/WASM DST byte comparison were not run. See the quality-gate record.

## What is deliberately absent

These are the next production steps. They are not implemented:

- Robust polygon clipping, hole-aware fill, and medial-axis extraction.
- Variable-width satin, short-stitch corners, and automatic density by fabric and thread.
- A fabric simulator and a defect taxonomy grounded in stitch-outs.
- PES, EXP, JEF, and VP3 writers.
- An agent that segments artwork. The Pydantic models are the contract that agent would have to satisfy.
- Prompt, scan, and web-asset intake, calibration fitting, vendor CAD, cap and multi-head rigs, and live telemetry.
- The proposed 192-stitch-out pilot. It has not been run.

Motif and applique exist on `StitchKind` so a plan can name them. There is no motif or applique generator.

Build from the repository root:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p embroidery-ffi
.venv/bin/python -m pytest python/tests
```

The Python package lives in `python/`. Its tests import it by putting `python/` on the path. The virtualenv at `.venv` provides `pydantic` and `pyembroidery`.
