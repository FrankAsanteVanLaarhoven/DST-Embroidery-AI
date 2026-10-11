# Validation and limits — Compatibility + Compare 0.2

This is the preparation package's own report. It is not this checkout's test log. Re-run the utilities from the repository root before citing a pass count here.

Date: 11 October 2026. Evidence scope: local Python policy and proof utilities.

The source package records all 24 supplied minimum/recommended obligations and 30 proposed extension obligations. They remain requirements, not device qualifications. The JSON register provides owner roles and acceptance evidence requirements.

**Final observed result: 42 tests passed, 0 failed**, as stored in `evidence/tests.log`. It covers numerical units, explicit unknown/legacy lifecycle handling, conservative 2D/3D routing, disabled AI/control/release authority, DST/PDF naming separation, fixture byte/hash checks, malformed trace rejection, source image intake, PDF source-page derivatives, PDF physical line dimensions, overwrite refusal and no silent 1:1 downscale. Register tests check presence, uniqueness, owners and absence of certified/passed qualification claims.

The source figure is a newly constructed TEST REFERENCE from the known 10 mm square's design geometry. It is not historical customer artwork and was not recovered from the DST. The DST/replay is the explicit M1 transport fixture, not new native Rust output. The full M1 native compile/decode pipeline has NOT been executed in this extension.

The PDF is two pages: source versus export-replay; registered overlay versus 1:1 vector proof. It has been rendered with PyMuPDF and both pages visually inspected. The 100 mm test bar's length was checked in PDF page coordinates (tolerance 0.02 point). No physical paper print or scanner calibration was performed.

## Failures and refinement

The initial 38-test run had one failure: an exact floating-point equality asserted that 1000 * (25.4/254) is bit-identical to 100. The result was 99.99999999999999. The numerical test now uses an explicit 1e-10 mm-scale tolerance; the conversion formula did not change. The initial log is retained.

Visual inspection found the second page's footer inherited a near-white drawing fill from the panel. The annotation fill is now explicitly set. A regression assertion checks the print-bar label's text colour. The regenerated pages were re-rendered and inspected.

## Deliberate trust boundary

The utility checks a DST SHA-256 and the trace's raw record bytes, not an independent semantic decode. A forged, hash-consistent trace can still misreport coordinates. A dedicated test confirms this limitation rather than concealing it. Native Rust decoding and an authenticated artifact store/release authority are still required for stronger claims. Similarly, hashing a selected source does not prove it is the original artwork that generated the file.

## Not tested or delivered

No native Rust run, new browser integration, GPU performance benchmark, OS or device compatibility lab, live AI/provider call, machine acceptance, fabric test, cloud deployment, external account change, GitHub write, standardisation submission or ISO certification. The proof tool's dependencies are a modern Python developer environment, not XP/Vista/7 executables. Renderer/CPU/memory floors in the specification are proposed targets only.
