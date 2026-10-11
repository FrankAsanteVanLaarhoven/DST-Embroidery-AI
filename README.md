# Embroidery engine

Deterministic stitch planning and a Tajima DST compiler. The kernel is Rust. Python holds region proposals and file checks. An agent emits a `DigitisingProposal`. It does not write stitch records or DST.

This repository is a foundation. It compiles a checked subset of DST that matches pyembroidery's writer on the cases in [docs/benchmarks.md](docs/benchmarks.md). It does not claim parity with any commercial digitising program. No physical stitch-out has been run.

## Layout

```text
crates/core       stitch model, geometry, running, satin, fill, validation
crates/dst        DST reader and writer
crates/simulate   length-density risk score
crates/ffi        C ABI and optional WASM bindings
python/embroidery_ai
python/tests
fixtures/dst      golden DST files
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

`EMBROIDERY_FFI` overrides the library path the Python tests load. `EMBROIDERY_PYTHON` overrides the interpreter the Rust DST oracle uses. The default oracle interpreter is `.venv/bin/python`.

The C header is `crates/ffi/include/embroidery.h`. Design notes are in [docs/engine.md](docs/engine.md). The DST byte layout is in [docs/format.md](docs/format.md).

PES, EXP, JEF, and VP3 are not implemented. Polygon clipping, hole-aware fill, and a fabric simulator are not implemented.
