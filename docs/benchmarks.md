# Benchmarks

This file records what has been measured. It is not a claim that the engine matches or beats any commercial digitising program.

## Interoperability, measured

The DST writer was compared with pyembroidery `DstWriter.write` and `encode_record` by `crates/dst/tests/pyembroidery_oracle.rs`:

- Every stitch and jump displacement from −121 to +121 on both axes matches `encode_record`, plus the colour-change and end control records.
- The 10 mm square, a stitch to `(0, -5)` mm, and an in-place colour change are byte-identical to `DstWriter.write`.
- A 30 mm jump split into 100 + 100 + 100 counts, followed by a 1 mm stitch, matches a pyembroidery pattern that was given those commands already split.
- pyembroidery reads the square back as stitch `(100, 0)`, stitch `(100, 100)`, stitch `(0, 100)`, end.

`fixtures/dst/square_running.dst` is that square. The Python FFI test compiles the same plan and checks it against both the fixture and `DstWriter.write`.

These checks show file compatibility for the covered commands. They do not show that a machine accepts the file, and they do not cover trim's `ST` field, which this compiler counts as real records while pyembroidery counts pattern commands. The trim body itself matches. That difference is described in [format.md](format.md).

## Not measured

The following bar is the one that would support a quality claim. It has not been run:

> On a fixed 500-design logo benchmark, the engine achieved equal or better blind QC acceptance, at least 15% lower stitch count, and fewer production defects than baseline manual digitising.

There is no 500-design corpus in this repository. `fixtures/dst/` contains the one generated square. `fixtures/stitchouts/` contains no photographs, QC labels, or machine telemetry. No sew-out has been done on cotton, polyester, fleece, cap panels, towel, or knit. No blind expert review has been done. No defect counts exist for puckering, registration drift, thread breaks, gaps, dense areas, small text, or trims.

Do not describe this tree as matching Wilcom, or any other digitising system, on stitch quality.

## Unit coverage that is not a benchmark

`cargo test --workspace` covers geometry resampling, satin width rejection, scanline fill on a rectangle and a C-shaped notch, validation messages, the risk score on an empty plan, a normal stitch, and a dense 1 mm pile, plus the DST cases above. Those tests lock the algorithms. They are not a production-quality score.

## Later measurements, when the assets exist

1. A licensed or self-created golden DST corpus with expected stitch counts, bounds, and machine acceptance.
2. The same artwork sewn on the fabrics listed above, with photos and stop codes stored under `fixtures/stitchouts/`.
3. A written defect taxonomy applied to those sew-outs.
4. A blind review by professional digitisers.
5. Only after those exist, the 500-design comparison against manual digitising.
