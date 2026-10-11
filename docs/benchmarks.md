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

## Project fixtures, measured

`crates/studio/tests/workflow.rs` runs the twelve documents in `fixtures/projects/` through project generation, validation, DST export, and `read_dst`. The valid running, satin, and fill fixtures produce a DST payload whose header starts with `LA:` and whose last point quantises back to the same design units. The four invalid fixtures fail with the expected unavailable-capability or record-limit message. The club-mark fixture inserts a colour change.

These fixtures lock the generation bridge. They are not a 500-design quality benchmark, and they were not sewn. Native and WASM DST bytes were not compared, because `wasm32-unknown-unknown` is not installed.

## TwinLab rehearsal, measured

`embroidery-motion` checks the named millimetre-to-metre transform, hoop hold during penetration, quantised replay against `read_dst`, and a hoop-limit warning whose origin is `kinematic_check`. `twinlab_replays_the_compiled_dst` posts the club-mark project to `/api/rehearse` and checks the qualification string, `machine_release_allowed: false`, a needle-down count, and a 64-character DST hash.

Chrome on `http://127.0.0.1:4173/twinlab` loaded that rehearsal. In the same turn, event 3 read design 25.000 mm, 14.000 mm and hoop −25.000 mm, −14.000 mm. The next event read 30.000 mm, 14.000 mm, units 300, 140. Setting playback to 4× left the estimated execution at 13.095 seconds. An injected thread break was labelled `injected_scenario` and was not observed. The 3D view started as an interactive scene. No physical sew-out was run.

This is not the proposed 192-stitch-out pilot (12 artworks × 2 flat configurations × 2 fabrics × 2 digitising methods × 2 repeats). That pilot has not been designed as a power calculation and has not been executed. It is not the 500-design benchmark above, and it is not a calibrated twin.

## Unit coverage that is not a benchmark

`cargo test --workspace` covers geometry resampling, satin width rejection, scanline fill on a rectangle and a C-shaped notch, validation messages, the risk score on an empty plan, a normal stitch, and a dense 1 mm pile, plus the DST cases above. Those tests lock the algorithms. They are not a production-quality score.

## Later measurements, when the assets exist

1. A licensed or self-created golden DST corpus with expected stitch counts, bounds, and machine acceptance.
2. The same artwork sewn on the fabrics listed above, with photos and stop codes stored under `fixtures/stitchouts/`.
3. A written defect taxonomy applied to those sew-outs.
4. A blind review by professional digitisers.
5. Only after those exist, the 500-design comparison against manual digitising.
