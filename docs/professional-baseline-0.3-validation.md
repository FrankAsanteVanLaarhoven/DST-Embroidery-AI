# Functional baseline 0.3 — validation record

Prepared 11 October 2026. Updated on this checkout after the register was added to the repository.

## Scope

This delivery is the requirements workbook, source-scope notes, preserved 0.2 baseline, and a local audit utility. It adds no stitch generator, machine driver, native browser integration, catalogue asset, or AI model.

The supplied workbook was written against commit `98303601416a3b0d68eae37e939031732bd77c45` and was copied here without editing its cells. This repository's later studio, TwinLab, and Compare work remains in the tree. It does not fill any of the 210 evidence cells, and it does not change a `NOT_RUN` status.

The supplied validation record reported 33 checker tests in its preparation package. This checkout's checker is `tests/capability_register/test_audit.py`. Its count is the count below. Do not add it to earlier transport or browser tests and call the sum a platform score.

## Observed checks

| Check | Observed result | Meaning |
|---|---|---|
| `.venv/bin/python -m unittest tests.capability_register.test_audit -v` | 29 passed, 0 failed | Tests of the register audit. No stitch quality or device qualification. Log: `evidence/baseline-0.3/ledger-tests.log`. |
| `.venv/bin/python tools/audit_capabilities.py` | Exit 0. Structural audit PASS. | 210 records; 54 inherited; 156 new or expanded; nine line groups; fourteen region families; six conflicts. |
| `.venv/bin/python tools/audit_capabilities.py --require-acceptance` | Exit 1, expected | `0 of 210 rows have accepted evidence`. |
| Preserved baseline | Byte-identical to `contracts/compatibility-coverage.json` | Snapshot: `contracts/inherited/compatibility-coverage-0.2.json`. Inherited identifiers, order, labels, and source text were compared. Original statuses remain `not_qualified`, `planned_not_qualified`, or `partial_developer_utility_only`. |
| Source conflicts | Six retained | Font totals, OS-table difference, hardware baselines, unattributed fragment, hoop and version scope, and “Stitch Stimulation”. |
| Workbook summary | Recomputed 210 / 54 / 156 / 0 accepted / 210 not accepted | Cached formula values match the requirement rows. Status caches are 138, 64, 4, 3, and 1. |
| Spreadsheet error scan | No `#REF!`, `#VALUE!`, `#DIV/0!`, `#NAME?`, `#N/A`, `#NULL!`, or `#NUM!` | Scan of the workbook XML. |
| Workbook visual render | Not run in a spreadsheet application | No LibreOffice or equivalent renderer was available. Sheet names, merged dashboard ranges, column widths, and cell text were inspected from the package. |
| Rust and native M1 for this addition | Not re-run | Earlier TwinLab evidence stays in the quality-gate record. It is not a 0.3 acceptance result. |
| Hardware, physical stitching, transfer, and AI | No tests | None of the new machine, process, or provider capabilities is qualified. |

`evidence/baseline-0.3/registry-audit.json`, `acceptance-gate.json`, and `workbook-check.json` hold the audit outputs. The audit report sets `authenticates_evidence`, `machine_release_allowed`, `iso_certification`, and `qualifies_platform` to false.

## Failure and refinement

The supplied package reported an early identifier collision between new editing rows and the inherited E-series. The workbook already uses ED01–ED28. The checker rejects a duplicate identifier, a missing inherited source string, and drift between the live 0.2 file and its snapshot.

Font totals and hardware variants were not resolved by guessing. The complete acceptance gate remains red because no new functional evidence was produced.

## Release conclusion

The package is suitable for requirements review and work planning. It is not a completed parity implementation, a certification claim, or a statement that every listed module runs. Native family coverage, lettering rights, decorative sewing, cutting, and physical qualification remain open.
