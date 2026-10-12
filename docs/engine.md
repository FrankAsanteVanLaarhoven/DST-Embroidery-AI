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

`generate_satin` samples the centreline by arc length. The left normal is the 90° CCW normal in Y-down coordinates, so a centreline moving in +X has its left rail toward +Y. Nominal rails sit at half the column width and are used for the edge-run underlay. Satin rails sit at half the width plus pull compensation. The column is a zigzag `L0 → R0 → L1 → R1`, and the connector is a satin stitch. Centre-run and edge-run underlay are selected independently. An empty selection sews the top satin only. Each enabled pass uses its own maximum stitch length, and an absent length inherits `max_stitch_mm`. Selecting both keeps the historical centre-then-edges traversal. The default width is 8 mm. A width that cannot be sewn as one DST record, such as 20 mm, is an error and does not change the plan.

`generate_fill` scan-converts one closed ring. A duplicate closing point is dropped. Edge crossings use a half-open interval so a vertex is counted once. Spans are paired even-odd. An odd intersection count is an error. Rows are inset by half the spacing and alternate direction. A positive `underlay_spacing_mm` adds a perpendicular running underlay. Travel between spans is a jump. The default angle is 45°, row spacing 0.4 mm, maximum stitch 5 mm.

`offset_polyline` is a naive normal shift. Satin does not use it. It fails on self-intersections, sharp corners, holes, and narrow regions. A production offset belongs in a clipper-style polygon library, which is not in this tree.

## Validation and risk

`validate(plan, max_stitch_mm, max_jump_mm)` blocks a strict compile when the plan is empty, a coordinate is not finite, a sewing stitch exceeds the length limit, or a sewing stitch exceeds ±121 counts on an axis. Jumps may be longer than one record. They are split without adding needle penetrations. A start away from the origin is a warning because the compiler inserts a jump from `(0, 0)`.

`score` rasterises sewing length into 5 mm cells. Density above 12 mm of stitch per mm² is a flag. A sewing stitch longer than 12.1 mm is an error. A jump longer than 500 mm is a warning. The report keeps at most 24 flags and sets `truncated` when more were dropped. This does not model puckering, registration, or thread breaks.

## Native project

An editable design is an `embroidery-project` document. Format version `"1"` holds the hoop, stable object ids, geometry in millimetres, digitising parameters, and an optional geometry review. Format version `"2"` is used when the design contains a text object. That object stores the entered string, the font id, the font version, the font-content hash, the origin, the cap height, and the spacing. An optional `layout` object selects policy `embroideryos-line-layout/1`: one to four explicit horizontal lines, left, centre, or right alignment by line advance, a first baseline, and baseline spacing in millimetres. Omitting `layout` keeps the historical single-line path. The layout policy, the font version, and the document version are separate identities. A version 1 document that contains text is rejected, and the text is not dropped. A version 1 document with no text stays version 1. The document does not hold stitch coordinates. `generate_project` resolves each object, checks the capability manifest, and calls `generate_running`, `generate_satin`, `generate_fill`, or the lettering stage. Lettering lays out EmbroideryOS Block 01 and sews each glyph component with the existing satin generator. It does not scale an earlier DST.

The revision is the SHA-256 of the canonical JSON with the `review` field removed. A geometry review stores that revision, a reviewer label, and scope `geometry`. Editing a parameter changes the revision, so the previous review no longer matches. `machine_release_allowed` is always false. Engineering DST export is allowed only when validation passes. Reviewed export also requires a matching geometry review. Those are local labels. They are not authenticated production authority.

The capability manifest marks running, satin, fill, and this lettering font as available. Horizontal lines are the layout policy above, not a second font. Motif, applique, zigzag underlay, lattice underlay, and machine release are planned. Satin underlay can be `centre_run`, `edge_run`, or both. Lettering uses those same passes. An absent pass length inherits `max_stitch_mm`. Old projects that omit the new length fields keep their previous bytes. Fill underlay is available only as `perpendicular_run`. Running has no underlay. An unsupported request fails with a message that names the capability as unavailable. Generation does not substitute another stitch type.

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

## Geometry intake that does not generate stitches

The loopback studio can ask for region geometry when `XAI_API_KEY` is set for the studio process. That request uses the xAI responses API. The model is not allowed to emit stitch records or DST. Apply is a separate edit.

`python/embroidery_ai/svg_geometry.py` and `POST /api/import-svg` convert a bounded SVG subset into a draft `embroidery-project` version 1 document. Preview does not change the open design. Apply is an edit, so a previous export no longer matches the revision. The route does not generate stitches or compile DST. A flattened outline is not Bézier editing. Coordinates use the native nonnegative hoop. They are not a centred TwinLab `generic-flat-120-v1` document. See [svg-geometry-intake.md](svg-geometry-intake.md).

Neither path writes stitches, authorises a machine, or accepts a baseline row.

## What is deliberately absent

These are the next production steps. They are not implemented:

- General polygon clipping, offsets, and medial-axis extraction. A fill may carry one simple outer ring and up to 32 simple holes that sit strictly inside it, without touching, overlapping, or nesting. Perpendicular underlay uses that same region. See [m2-g1-holes.md](m2-g1-holes.md).
- Automatic density by fabric and thread. Centre-run and edge-run underlay can be selected alone or together. An absent pass length inherits the object's maximum stitch. Zigzag underlay, a new inset, and automatic fabric prescriptions are not in this increment. A per-point `column_widths_mm` list varies a satin column, miters a sharp corner up to twice the half-width, splits a sewing movement wider than 12 mm on an axis into at most 16 pieces, and omits a satin segment shorter than 0.2 mm. A reversed, folded, or crossing column is rejected and is not repaired. An empty list keeps the constant-width column, including its rejection of an over-wide rung.
- A fabric simulator and a defect taxonomy grounded in stitch-outs.
- PES, EXP, JEF, and VP3 writers.
- An agent that segments artwork. The Pydantic models are the contract that agent would have to satisfy.
- Scan and web-asset intake, calibration fitting, vendor CAD, cap and multi-head rigs, and live telemetry.
- The proposed 192-stitch-out pilot. It has not been run.
- Automatic wrapping, curved baselines, TrueType or OpenType import, and monograms. One to four explicit horizontal lines use layout policy `embroideryos-line-layout/1`, with left, centre, or right alignment and a declared baseline spacing. That policy is separate from the font version and the document version. EmbroideryOS Block 01 version 1 remains A–Z, 0–9, space, hyphen, and a straight apostrophe. Version 2 is a separate pinned recipe, chosen explicitly, with lowercase, U+2019, and a bounded accent set. A bounded name drop clones the selected text object for each table row. It does not accept the lettering register.

Motif and applique exist on `StitchKind` so a plan can name them. There is no motif or applique generator.

Build from the repository root:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p embroidery-ffi
.venv/bin/python -m pytest python/tests
```

The Python package lives in `python/`. Its tests import it by putting `python/` on the path. The virtualenv at `.venv` provides `pydantic` and `pyembroidery`.
