# DST Embroidery AI

EmbroideryOS is a local design studio on a deterministic Rust embroidery kernel. You edit a native project, the Rust generators sew it, and the studio can export a Tajima DST file and replay those exact bytes. Python proposes regions and checks files. It does not plan stitches or compile DST.

The repository is [DST-Embroidery-AI](https://github.com/FrankAsanteVanLaarhoven/DST-Embroidery-AI). The studio owner line is Frank Asante Van Laarhoven. This is a foundation. It does not claim parity with a commercial digitising program. No physical stitch-out has been run. `machine_release_allowed` stays false.

## What you can do

- Open a native `embroidery-project` version 1 design, including the Alpine Club badge and the holed club mark. Version 2 adds one editable text object. Version 1 documents stay version 1.
- Edit running, satin, and fill regions in millimetres. Fill can sew one simple outer ring and explicit holes that sit strictly inside it.
- Type one to four horizontal lines in one EmbroideryOS Block 01 text object, choose left, centre, or right alignment, a baseline spacing, a cap height, and underlay, and generate it in Rust. The object keeps the entered characters and the pinned font. Version 1 is the default repertoire: A–Z, 0–9, space, hyphen, and a straight apostrophe. Version 2 is a separate pinned recipe, chosen explicitly, with lowercase, U+2019, and a bounded accent set.
- Generate the stitch plan in Rust, inspect it, record a local geometry review, and download the engineering DST.
- Rehearse that DST in TwinLab. The 2D and 3D views play the decoded file on a generic fixed-needle frame.
- Compare original artwork, the editable design, the machine file, a PDF proof, and the twin on one revision.
- Ask for a geometry proposal from typed text, pasted text, an uploaded JPEG or PNG, or voice. Voice becomes text in the browser. The model returns regions. You apply them yourself, then Rust generates the stitches.

## What stays unavailable

Machine release, a connected machine, and physical validation are unavailable. Motif, applique, zigzag, and lattice fail clearly. Satin underlay is `centre_run`, `edge_run`, or both. Each pass has its own maximum stitch length, and an absent length inherits the object's maximum stitch. Lettering uses those same passes. Fill underlay is only `perpendicular_run`. Running stitch has no underlay. Version 1 does not sew lowercase, accents, or U+2019. Version 2 is an explicit selection of one separate pinned recipe with lowercase, U+2019, and a bounded accent set. Curved baselines, automatic wrapping, font import, monograms, and a font catalogue are not in this increment. One to four explicit horizontal lines, with left, centre, or right alignment and a declared baseline spacing, are layout policy `embroideryos-line-layout/1` on the same text object. That policy is not a new font. An unsupported character is rejected and the entered text is left unchanged. A bounded name drop personalises one text object from a table. It does not accept the lettering register.

PES, EXP, JEF, and VP3 are not implemented. The reference EXF document is rejected until a tested adapter exists. General polygon clipping, offsets, and a fabric simulator are not implemented. Hole-aware fill is the bounded case in [docs/m2-g1-holes.md](docs/m2-g1-holes.md). It does not accept a broad requirement row.

The professional baseline in [docs/professional-baseline-0.3.md](docs/professional-baseline-0.3.md) is a requirements register. All 210 acceptance statuses are `NOT_RUN`. A recorded row is not a working generator.

Display colours in the studio are a viewing palette. DST does not store those RGB values. The nominal time on screen is the sewing stitch count divided by 650 stitches per minute. It excludes setup, trims, acceleration, and stops.

## Design studio

The studio binds to `127.0.0.1` only. Keep it off public networks.

```text
cargo run -p embroidery-studio
```

Open `http://127.0.0.1:4173`. Another port is `cargo run -p embroidery-studio -- serve 4176`.

The page posts the project to the Rust process. `web/studio.js` draws the returned plan. It does not generate stitches, encode DST, or decode record semantics. An engineering DST download is a local file. It is not permission to transfer or run a machine.

The design prompt calls the xAI responses API only when you choose Propose design and the studio process has `XAI_API_KEY`. The model id is `grok-4.7`. The key stays in the server process. A missing key, an empty prompt, or Discard leaves the open design unchanged. Apply geometry is a separate edit. Apply and generate then uses the existing Rust route. A proposal cannot approve itself.

The local digitising assistant is a spacing and underlay rule. It does not call a model.

## TwinLab and Compare

Studio rehearsal is `http://127.0.0.1:4173/twinlab`. It uses the project hoop. The qualification line is “Virtual demo · Generic machine · No hardware connected · Physical quality not validated.” Playback speed does not change the trace duration. Development timing on this route is 600 stitches per minute, 200 mm/s jumps, 2.0 s colour changes, and 0.4 s trims. Those figures are assumptions.

`apps/twinlab` is a separate loopback service. Node launches `embroidery-twin`, which calls the same generators, `compile_to_vec`, and the DST decoder.

```text
cargo build -p embroidery-twin
PORT=4175 node apps/twinlab/server.mjs
```

Open `http://127.0.0.1:4175/`. The default port is 4173, so set `PORT` when the studio is already running. This path uses 600 stitches per minute, jumps at 100 mm/s with a 20 ms minimum, and 1 s colour changes. The two timing models are separate.

The Compare profile is `generic-flat-120-v1`, with points in [−60, 60] mm. A 160 mm design does not fit that profile. The studio rehearsal can still play it when the geometry is inside the project hoop. Details are in [docs/twinlab-m1.md](docs/twinlab-m1.md) and [docs/compatibility-and-compare.md](docs/compatibility-and-compare.md).

## Build and test

Rust 1.83 or newer, and a Python 3.11 virtualenv with `pydantic` and `pyembroidery`:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p embroidery-ffi
.venv/bin/python -m pytest python/tests
```

`EMBROIDERY_FFI` overrides the library path the Python tests load. `EMBROIDERY_PYTHON` overrides the interpreter the Rust DST oracle uses. The default oracle interpreter is `.venv/bin/python`. Build the cdylib with `cargo build -p embroidery-ffi` before pytest when the C ABI has changed.

The C header is `crates/ffi/include/embroidery.h`. Design notes are in [docs/engine.md](docs/engine.md). The DST byte layout is in [docs/format.md](docs/format.md). The `wasm32-unknown-unknown` target is not part of the checked local toolchain, so WASM parity is not claimed.

Capability ledger:

```text
.venv/bin/python tools/audit_capabilities.py
.venv/bin/python tools/audit_capabilities.py --require-acceptance
```

The first command checks ledger consistency. The second exits 1 until every scope has accepted evidence. The checker cannot authorise a machine release.

## Layout

```text
crates/core       stitch model, geometry, running, satin, fill, holes, validation, project document
crates/dst        DST reader and writer
crates/simulate   length-density risk score
crates/ffi        C ABI and optional WASM bindings
crates/motion     export-faithful execution trace and a generic fixed-needle rig
crates/studio     loopback design studio, TwinLab route, and geometry proposals
crates/twin       native TwinLab compile and replay
apps/twinlab      separate Compare workspace
web               studio and TwinLab pages
python/embroidery_ai
fixtures/projects native project fixtures
fixtures/dst      golden DST files
contracts         compatibility coverage and the baseline 0.3 workbook
docs              format, engine, TwinLab, holes, baseline, and quality notes
```

## Kernel boundary

An agent may emit a `DigitisingProposal` or a geometry proposal. Neither document contains stitch coordinates or DST bytes. `generate_project` is the stitch authority. `Engine.compile_dst` rejects a document that has no stitches and tells the caller to run the generators first.

Coordinates are millimetres, Y positive downward. The DST writer flips Y. Quantisation is half-even through `mm * 10`. A sewing record is at most ±121 counts (12.1 mm). The trim signature is `(+2,+2)`, `(−4,−4)`, `(+2,+2)`. pyembroidery is the DST oracle for the checked cases in [docs/benchmarks.md](docs/benchmarks.md). It is not the engine.

The workspace is `publish = false` and has no license field.
