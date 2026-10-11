# Quality gate

Delivery/build or commit: local workspace, not committed, not published. Crate version 0.1.0.
Environment and URL (no secrets): macOS, rustc 1.90.0 (Homebrew), Python 3.11.4, pydantic 2.14.0, pytest 9.1.1, pyembroidery 1.5.1 in `.venv`. No URL. The library was not served.
Scope and classification: non-UI local library. Rust stitch kernel, DST compiler, C ABI, and a Python proposal package. Not a public release, preview site, or native app.
Viewports and browser/device: none. There is no rendered interface.
Commands and evidence paths:

- `cargo fmt --all -- --check` — exit 0
- `cargo clippy --workspace --all-targets --offline -- -D warnings` — exit 0
- `cargo test --workspace --offline` — exit 0. embroidery-core 18 passed, embroidery-dst 16 passed, pyembroidery oracle 5 passed, embroidery-ffi 3 passed, embroidery-simulate 3 passed, doc-tests 0
- `.venv/bin/python -m pytest python/tests -q` — 15 passed in 0.10s
- `rustup target list --installed` — `aarch64-apple-darwin` only

| Check | Result | Evidence or concrete reason |
|---|---|---|
| Q01 Production domain | N/A | No website and no release host. No domain was invented or configured. |
| Q02 Page titles | N/A | No pages or app views. |
| Q03 Meta descriptions | N/A | No public indexable pages. |
| Q04 Favicon and icons | N/A | No web or app icon surface. |
| Q05 Open Graph image | N/A | Nothing is shared as a web page. |
| Q06 Canonical URLs | N/A | No indexable origin. |
| Q07 Social previews | N/A | No social card. |
| Q08 Custom 404 | N/A | No document routes. Library errors stay on the function result. |
| Q09 Loading states | N/A | Compile, validate, and score are synchronous. There is no pending UI operation. |
| Q10 Error messages | PASS | Rejected plans name the failure and the next step. Empty plan: "Generate a running, satin, or fill object before compiling." Bad JSON: "Could not read the stitch plan JSON" and tells the caller to use the generators. Truncated DST: "export it again." A proposal passed to `compile_dst` is refused with "Run the Rust generators". Covered by `validate::tests`, `read::tests::truncated_record_names_the_next_step`, `c_api_compiles_and_rejects_bad_json`, `test_validate_empty_plan_names_the_next_step`, and `test_proposal_is_not_compiled`. |
| Q11 Heading structure | N/A | No rendered document view. |
| Q12 Alternative text | N/A | No product images. `fixtures/stitchouts/` has no photos. |
| Q13 Sitemap and indexing | N/A | Private local library. No sitemap was added. |
| Q14 Console errors | N/A | No browser session. The exercised paths are `cargo test` and pytest, both exit 0. |
| Q15 Debug output | PASS | No `println!`, `eprintln!`, `dbg!`, or `console.log` in the crates or the Python package. The only `print` is the pyembroidery oracle's stdout protocol inside `crates/dst/tests/pyembroidery_oracle.rs`, which the test reads. It is not shipped library code. |
| Q16 Source maps | N/A | No release artifact was built or published. The `wasm` feature was not compiled, so no WASM source map exists. |
| Q17 JavaScript and performance | N/A | No JavaScript bundle. WASM was not built, so there is no bundle to measure. |
| Q18 Mobile layout | N/A | No layout. |
| Q19 Consistent spacing | N/A | No UI. The Rust code uses the existing types and millimetre units. |
| Q20 Working controls and links | N/A | No buttons or links. The C ABI and proposal schema are what the tests exercise. |

Known gaps and next action:

- `wasm32-unknown-unknown` is not installed, so `compileDst` / `validatePlan` / `scorePlan` were not built. Install the target before treating the WASM build as checked.
- Polygon clipping, holes, medial axis, variable-width satin, PES/EXP/JEF/VP3, and a fabric simulator are not implemented. See `docs/engine.md`.
- The 500-design benchmark has not been run. No physical stitch-out exists. See `docs/benchmarks.md`.

Release-ready claim: no. This is a local foundation. The applicable library checks that were run passed. The web checks do not apply. The WASM build and every production-quality claim remain open.
