# Embroidery engine

Deterministic stitch planning and a Tajima DST compiler. The kernel is Rust. Python holds region proposals and file checks. An agent emits a `DigitisingProposal`. It does not write stitch records or DST.

This repository is a foundation. It compiles a checked subset of DST that matches pyembroidery's writer on the cases in [docs/benchmarks.md](docs/benchmarks.md). It does not claim parity with any commercial digitising program. No physical stitch-out has been run.

## Layout

```text
crates/core       stitch model, geometry, running, satin, fill, validation, project document
crates/dst        DST reader and writer
crates/simulate   length-density risk score
crates/ffi        C ABI and optional WASM bindings
crates/motion     export-faithful execution trace and a generic fixed-needle rig
crates/studio     loopback design studio and TwinLab
web               studio and TwinLab pages. The pages draw what Rust returns.
python/embroidery_ai
python/tests
fixtures/dst      golden DST files
fixtures/projects native project fixtures
fixtures/stitchouts
docs/format.md
docs/engine.md
docs/benchmarks.md
```

## Build and test

Rust 1.83 or newer, and a Python 3.11 virtualenv with `pydantic` and `pyembroidery`:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p embroidery-ffi
.venv/bin/python -m pytest python/tests
```

`EMBROIDERY_FFI` overrides the library path the Python tests load. `EMBROIDERY_PYTHON` overrides the interpreter the Rust DST oracle uses. The default oracle interpreter is `.venv/bin/python`. Python tests load the cdylib, so build it with `cargo build -p embroidery-ffi` before pytest when the C ABI has changed.

The C header is `crates/ffi/include/embroidery.h`. Design notes are in [docs/engine.md](docs/engine.md). The DST byte layout is in [docs/format.md](docs/format.md).

## Design studio

The studio is a local page on `127.0.0.1`. It opens the sample project, edits object parameters, and asks the Rust engine to generate the stitch plan.

```text
cargo run -p embroidery-studio
```

Open `http://127.0.0.1:4173`. The process binds to loopback only. Keep it off public networks.

The page can change the project, draw the returned plan, and download the DST bytes the engine produced. Stitch generation and DST encoding stay in Rust. An engineering DST download is a local file export. Machine release stays unavailable.

Rehearse in TwinLab, or open `http://127.0.0.1:4173/twinlab`. Rust generates the design, compiles DST, and decodes those bytes. The 2D and 3D views play that trace on one generic rig: the needle stays on the machine axis and the hoop moves. The qualification line is “Virtual demo · Generic machine · No hardware connected · Physical quality not validated.” Timing figures are development assumptions. Changing playback speed does not change the estimated execution time. Calibrated prediction and a connected twin are unavailable. This demo is not a 192-stitch-out pilot, the 500-design benchmark, or an ISO certificate.

## TwinLab M1

`apps/twinlab` is a separate loopback service. Node launches `embroidery-twin`. That crate calls the existing running, satin and fill generators, `compile_to_vec`, and the DST decoder. The browser does not generate stitches or encode DST.

```text
cargo build --offline -p embroidery-twin
PORT=4175 node apps/twinlab/server.mjs
```

Open `http://127.0.0.1:4175/`. The default port is 4173, which is also the design studio’s port, so set `PORT` when the studio is already running. The page loads Alpine mark, compiles it in Rust, and replays the exact DST records in linked 2D and 3D views. The generic rig is labelled as a virtual six-needle flat frame. It is not a manufacturer model. Hardware controls are absent. Details and the acceptance command are in [docs/twinlab-m1.md](docs/twinlab-m1.md).

## Compatibility and Compare

[docs/compatibility-and-compare.md](docs/compatibility-and-compare.md) keeps the supplied hardware baseline as a recorded requirement. The coverage register is [contracts/compatibility-coverage.json](contracts/compatibility-coverage.json). Every row stays `not_qualified` until its own evidence exists. A file extension, a processor speed, or a computer brand is not a support badge.

The TwinLab page has a Compare section: original artwork, the editable design, the exact Rust DST, a reopened PDF proof, and the machine twin. Those five stay bound to one revision. Editing the design clears the export, the replay, and the proof. The PDF tool checks the stored DST hash and raw record bytes. It does not run Rust again, and it does not decode coordinates a second time. A `.dts` upload keeps its name and is not executed.

```text
.venv/bin/python -m unittest tests.compat_compare.test_compare
```

Scanner, tablet, printer, legacy operating-system, and other embroidery-format adapters are specified. They are not qualified by this workspace.

PES, EXP, JEF, and VP3 are not implemented. Polygon clipping, hole-aware fill, and a fabric simulator are not implemented. The reference EXF document is not accepted.
