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
| `embroidery-core` | Points, stitch plan, geometry, running/satin/fill generators, validation. |
| `embroidery-dst` | Ternary codec, DST compiler, DST reader. |
| `embroidery-simulate` | Length-density risk score. Not a fabric simulation. |
| `embroidery-ffi` | C ABI and an optional `wasm` feature. |

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

## FFI

`crates/ffi/include/embroidery.h` declares:

- `embroidery_engine_version`
- `embroidery_compile_dst_json`
- `embroidery_validate_json`
- `embroidery_score_json`
- `embroidery_free`

Return codes are 0 for success, 1 for a UTF-8 error message with no trailing NUL, and 2 for a null argument. Every non-null buffer is released with `embroidery_free(ptr, len, cap)`. The Python package `embroidery_ai` loads `target/debug/libembroidery_ffi.dylib` (or `EMBROIDERY_FFI`) and frees those buffers.

JSON stitch kinds are snake_case: `running`, `satin`, `fill`, `motif`, `applique`, `jump`, `trim`, `color_change`.

The `wasm` feature exports `compileDst`, `validatePlan`, and `scorePlan` through wasm-bindgen. It is not a default feature. The `wasm32-unknown-unknown` target was not built for this foundation. See the quality-gate record.

## What is deliberately absent

These are the next production steps. They are not implemented:

- Robust polygon clipping, hole-aware fill, and medial-axis extraction.
- Variable-width satin, short-stitch corners, and automatic density by fabric and thread.
- A fabric simulator and a defect taxonomy grounded in stitch-outs.
- PES, EXP, JEF, and VP3 writers.
- An agent that segments artwork. The Pydantic models are the contract that agent would have to satisfy.

Motif and applique exist on `StitchKind` so a plan can name them. There is no motif or applique generator.

Build from the repository root:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p embroidery-ffi
.venv/bin/python -m pytest python/tests
```

The Python package lives in `python/`. Its tests import it by putting `python/` on the path. The virtualenv at `.venv` provides `pydantic` and `pyembroidery`.
