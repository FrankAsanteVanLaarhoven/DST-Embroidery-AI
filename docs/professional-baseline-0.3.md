# EmbroideryOS professional creative and production baseline 0.3

Prepared: 11 October 2026.
Workbook: [contracts/EmbroideryOS_Master_Capability_Register_0_3.xlsx](../contracts/EmbroideryOS_Master_Capability_Register_0_3.xlsx).
Sources: [contracts/sources.json](../contracts/sources.json).
Preserved conflicts: [contracts/claim-conflicts.json](../contracts/claim-conflicts.json).
Inherited 0.2 file: [contracts/inherited/compatibility-coverage-0.2.json](../contracts/inherited/compatibility-coverage-0.2.json).

Classification: requirements, implementation priorities and a local register checker. This is not an implemented platform release, a competitor-parity result, a machine certification, or an ISO standard.

## Checkout note

The supplied workbook was prepared against repository commit `98303601416a3b0d68eae37e939031732bd77c45`. This checkout's `main` is later. It contains the design studio, the TwinLab native path, and the Compare workspace. Those additions are narrower than this baseline.

The workbook text is unchanged, including its statement that the supplied M1 package had not passed a full native gate. Later TwinLab evidence in [docs/quality/gate.md](quality/gate.md) is not deleted and is not reclassified as acceptance of these 210 records. Every acceptance status in the 0.3 register remains `NOT_RUN`.

This delivery adds the register, the source notes, and `tools/audit_capabilities.py`. It adds no stitch generator, machine driver, font, design catalogue, scanner adapter, or AI model.

## 1. Evidence boundaries

Three classes stay distinct:

1. Source baseline. The supplied tables and screenshot keep their numbers, names, qualifications, and contradictions.
2. Outside verification. Official product and manual pages clarify identity and technical boundaries. Vendor descriptions remain vendor claims.
3. Proposed product requirements. Implementation contracts and targets do not become delivered features by appearing in the register.

The master register has 210 records: 54 inherited from compatibility baseline 0.2 and 156 new or expanded records. The 156 include engineering gates, aliases, overlapping tasks, and extensions. They are not 210 capabilities already implemented. The nine supplied line groups and fourteen named region families each have their own records.

Current register counts:

| Implementation status | Records | Meaning in this package |
|---|---:|---|
| PLANNED | 138 | Specified, not built as the named capability |
| UNQUALIFIED | 64 | Inherited or hardware rows, not passed |
| LIBRARY_SUBSET | 4 | Running, satin, fill, and the existing underlay controls only |
| CANDIDATE_SUBSET | 3 | Native gate, point editing, and playback alias |
| REFERENCE_UTILITY | 1 | The local proof utility, not an integrated universal viewer |

All 210 acceptance statuses are `NOT_RUN`. No scope is accepted.

## 2. Source conflicts

The six issues in `contracts/claim-conflicts.json` are deliberate.

| Issue | Treatment |
|---|---|
| 100, 130, and “hundreds” of fonts | Keep each wording. Do not add them or count glyphs as fonts. |
| Windows headline versus minimum-PC subtable | Preserve Windows 11/10/8.1/7 and Windows 7/8.1/10 separately. Neither qualifies EmbroideryOS. |
| 1 GHz / 512 MB / 300 MB versus the earlier DRAWings/Wings numbers | Separate historical profiles. Do not replace the 54-row baseline. |
| Unlabelled “approachable digitising” fragment | Retain the bitmap, vector, and sequencing requirements. Do not guess the vendor. |
| Hoop and export versions | Preserve 360 × 360 mm and 272 × 408 mm. A later frame or PES revision is separate evidence. |
| “Stitch Stimulation” | Preserve the source spelling as an alias. The requested function is playback, not calibrated physical simulation. |

Creator and DesignerPlus match BERNINA V9 product levels. That does not make the two tiers equivalent or identify every unlabelled fragment. WilcomWorkspace browser functions are not automatically EmbroideryStudio desktop tools. Embird's advertised format and brand counts are a breadth reference, not uniform read/write, machine acceptance, telemetry, or remote control.

## 3. One project, seven workflow modules

Seven responsibilities can live in one application. They do not require seven executables.

| Module | Responsibilities |
|---|---|
| Design Studio | Layout, dimensioned shapes, curves, freehand, manual digitising, object and stitch editing, alignment, grouping, transformations, sequencing, and splitting. |
| Artwork Lab | Scan, camera, paste, and upload. Preserve originals. Register scale. Clean rasters, edit masks, convert vectors, and propose regions. |
| Pattern & Quilt Lab | Nine line groups, fourteen region families, programmable motifs, quilt backgrounds, echo, stipple, gradients, and fabric-aware presets. |
| Type Lab | Digitised fonts, TrueType outlines, custom font creation, glyph mapping, small text, monograms, name-drop, outlines, and transformations. |
| Library & Learn | Searchable original designs, templates, thumbnails, tutorials, skill lessons, rights, provenance, and versioned releases. |
| Machine Hub | Machine, controller, attachment, and hoop profiles. Split and rehoop instructions. Queued exports. Send, receive, and delivery evidence stay separate. |
| Sewing Pattern Lab | Original decorative sewing patterns, repeats, needle and feed constraints, and sewing-pattern export. This is not DST export. |

Compare and TwinLab remain shared workspaces. Original artwork, editable objects, the saved machine file, the PDF proof, and the selected machine trace stay separately inspectable. Safety findings and qualification status remain visible in a simplified mode.

The interface direction remains the existing content-first studio: contextual inspector, progressive disclosure, shared keyboard commands, undo and redo, visible units, and an explicit save and compile state.

## 4. Editing and digitising

Preserve original bytes. Colour tuning, masking, tracing, and vectorisation create derivatives. Scan metadata does not establish physical accuracy. A missing or conflicting scale requires a reference or an explicit user size. A camera image and a flatbed scan have different registration needs.

Photo-to-stitch needs a simplified workflow and a detailed workflow. Both expose output dimensions, palette, stitch budget, region assignments, and correction controls. An AI raster is not itself a digitised embroidery design.

Automatic digitising proposes editable regions, direction guides, stitch families, and parameters. The user can reject or repair one region. “Prompt any design” is an intake goal. Unsupported material or machine constraints return an explicit limitation.

The register covers thirty shape families, rectangle and circle, freehand and curves, manual punch, point editing, align and distribute, scale, rotate, flip, and circular, matrix, and mirror copies. Copies keep physical units and regenerate dependants.

It also covers region-to-line, stitch-to-outline, text-to-outline, direction-line editing, offsetting, overlap removal, merging, and holes. Reconstruction from imported stitches is an approximation. The imported bytes stay intact.

Density-preserving resize of native objects regenerates from geometry and parameters. Multiplying every coordinate is not preserved density. Sequence optimisation respects layering, applique and cutwork stages, required stops, and tool changes. Travel reduction is subordinate to those dependencies.

Human stitch-point edits are allowed with provenance and engine validation. An agent still may not bypass generation and review and submit machine commands. Robust clipping and offsetting come before expanding the family selector. A failed generator must not leave a partial approval or a silent substitution.

## 5. Stitch-family inventory

Nine line groups: Running, Zigzag, Triple, Motif, Stem, Candlewicking, E/V, Chain, and Cutting.

E/V remains one supplied group, with its variants distinguished in the implementation. Cutting is a tool operation, not a name for an ordinary running stitch. Decorative chain-like geometry does not imply a true chain-stitch mechanism.

Fourteen region families: Satin, Fill, Programmable fill, Motif, Piping, Cross Stitch, Concentric Circle, Radial, Spiral, Flexible Spiral, Stippling, Net Fill, Zigzag Net Fill, and Decorative Fill.

The source manual groups concentric, radial, and spiral in one explanatory item. The supplied table names fourteen families. The register keeps fourteen names.

Acceptance for each family requires editable parameters, deterministic seeded output where relevant, bounded generation cost, positive and adversarial geometry, maximum-stitch and hoop validation, native save and reopen, actual exported-file decode, and scoped physical quality evidence before a production claim. Test concave outlines, holes, narrow regions, close boundaries, excessive density, direction-field singularities, and cancellation.

A motif creator lets a user author, save, edit, and reuse a unit with entry and exit points, spacing, phase, repetition, and travel rules. A fixed thumbnail list is not that creator. Stamps, embossing, and engraving here are embroidery texture operations. Laser engraving and milling need their own process qualification.

The current engine supplies limited running, centreline satin, and single-ring fill. `LIBRARY_SUBSET` records that limit. The complete nine and fourteen families are not implemented.

## 6. Quilting, applique, cutting, and printing

Quilting coverage includes echo and background fill, stippling, decorative fill, concentric, radial, and spiral patterns, flexible spiral, gradients, and programmable motifs. The graph preserves excluded areas, spacing, topology, material settings, and rehooping transforms. Hoop quilting is not universal longarm-controller support.

Applique represents placement, a material-placement stop, tack-down, and cover stages. A cut contour is a separate artifact with its own allowance, scale, and tool configuration. Cutwork states the cutting tool and the operator stages.

Printing and stitching bind a physical-scale PDF and an embroidery artifact to one registered design revision. Show fiducials and reference dimensions. Compensate only through a declared transform. Measure real print and hoop registration before promising accuracy.

FCM import and export belong to a cutting adapter. Brother's manual describes cutting geometry and CanvasWorkspace handoff limits. It does not grant a third-party API, and FCM is not a complete native embroidery plan.

## 7. Four artifact families

Immutable source assets, the native editable project, and the selected profiles feed one reviewed operation graph. That graph can emit four different artifacts:

| Artifact | Compiler | File result |
|---|---|---|
| Embroidery plan | The existing Rust embroidery compiler, then separately qualified adapters | The repository's declared DST subset exists. PES and other embroidery formats do not. |
| Decorative sewing plan | A qualified needle and feed pattern compiler | A sewing-pattern format. Not DST. |
| Cutting plan | A qualified cutter adapter | FCM or another declared cutting format. |
| Print proof | A PDF or print renderer | A proof, not a machine-executable embroidery file. |

Brother states that Stitch Design Factory creates decorative sewing patterns and cannot create embroidery patterns. Sewing Pattern Lab needs its own pattern model, needle-bight and feed representation, repeats, machine limits, and export qualification. A generic hoop simulation is not a feed-dog and needle-swing simulation.

Every artifact carries a kind, design revision, generator or adapter version, profile binding, byte hash, known losses, and evidence status. A viewer that cannot interpret the kind reports unsupported. A hash detects identity changes. It is not approval or release authority.

## 8. Lettering and library

Type Lab stores native text separately from derived glyph geometry and the stitch plan. Store Unicode text, shaping decisions, font identity and version, and glyph mapping. Diacritics and combining marks stay. A script-support claim requires tests.

TrueType import supplies outlines. It is not a library of qualified embroidery glyphs. Font Creator creates and edits glyph patterns and maps imported embroidery to characters. Monograms have layouts and ordering rules. Name-drop batches substitutions, fits text inside declared bounds, and checks every output. It must not stretch a long name into an unsafe width.

The supplied quantities — 100, 130, and hundreds of fonts, ten small-font entries, two monogram entries, more than 1,000 designs, and fifteen thread brands — are inventory obligations. Installed operating-system fonts do not satisfy a digitised catalogue. Publish digitised families, glyph coverage, small-text recipes, outline-font imports, monogram layouts, original designs, and catalogue versions separately.

Targets that exceed an incumbent's count require independently created or appropriately licensed assets. Count distinct designs and families, not recolours, glyphs, or duplicated templates. This package contains no third-party commercial font or design catalogue.

Library & Learn uses original tutorials, reproducible lesson projects, filters by skill, material, and hoop, thumbnails, saved collections, and dated releases. Mobile drawing should reopen the same native objects on the desktop. Offline editing is a proposed requirement, not a claim about an external application.

## 9. Interoperability and older systems

For every format and version, record read, write, view, metadata loss, supported commands, and machine acceptance separately. For every manufacturer, model, controller, firmware, and attachment, record send, receive, queue, telemetry, and control separately. A brand, a file extension, or a successful USB copy is not a qualification.

The selected hoop has a usable region, orientation, exclusions, and an attachment mode. The 360 × 360 mm and 272 × 408 mm cases are test cases, not a global artboard limit and not evidence for every machine with that nominal area. Multi-position hoops and separate rehooping need distinct transforms and a stated registration uncertainty.

Keep XP1, XE1, and XJ1 as separate candidate profiles. Use documented and available adapters. Wireless transfer in a vendor application does not mean this application has an unrestricted API. Transfer acknowledgement, machine acceptance, queued state, sewing start, and job completion are different states. Unknown receive functionality stays disabled even when send works.

The PE-Design-style profile is additional: 1 GHz, 512 MB minimum and 1 GB recommended, 300 MB storage, 1024 × 768 at 16-bit colour, two USB ports, and the two supplied operating-system lists. The original 54-row baseline remains unchanged. A 512 MB capture helper is not full on-device AI or CAD parity.

Operating system, architecture, graphics path, peripheral driver, embroidery machine, and AI provider are independent dimensions. Dell is not an operating-system API. An Apple-inspired layout does not make the application Apple-only.

This package does not create a legacy executable, a scanner driver, a peripheral adapter, or a machine connection. Older isolated equipment may use a measured gateway or file exchange, and that limitation must be visible. Competitor USB-key distribution is a source reference. EmbroideryOS uses its own installer and entitlement design. No copied dongle mechanism and no security bypass is part of this specification.

## 10. Beyond a feature checklist

Cloud viewing, automatic digitising, and simulation are already advertised by supplied competitors. The proposed differentiation is editable semantic intent, exported-byte fidelity, capability-aware multi-process planning, and physical outcome evidence.

Retained extensions include bounded provider-neutral AI proposals, local repair suggestions, an offline core, cross-platform access, shared reviews, parser isolation, compute budgets, calibration evidence, original/PDF/file/twin comparison, and a versioned outcome corpus. A prediction states whether it is illustrative, model-derived, or validated for the selected configuration.

An AI proposal still does not write authoritative stitch commands, approve itself, or mark a capability verified. Rust remains the only embroidery stitch and DST authority.

ISO 23247-1 is a manufacturing-twin framework reference. It is not an embroidery certification. No clause-by-clause conformity assessment was done.

## 11. Delivery sequence and gates

The sequence is staged work, not a promise that one person finishes a commercial suite on a fixed week count.

| Stage | Responsibility |
|---|---|
| M1 | Native integration on one reviewed commit. Absent-engine and fixture-substitution cases fail. |
| M2 | Preserve original assets and show original, native, exact machine file, actual PDF, and twin together. Stale revisions block current approval. |
| M3 | Clipping, holes, and bounded offsets, then family generators and programmable motifs. No placeholder families. |
| M4 | Glyphs, palette and mask correction, simple and detailed photo modes, and checked batch names. |
| M5 | Quilting, applique, printing, cutting, and decorative sewing through separate adapters. |
| M6–M8 | Original or licensed content, model-specific formats, and host and device profiles as evidence appears. |

| Gate | Proposed checkpoint |
|---|---|
| Native integrity | Required native gates pass on the same reviewed commit. |
| Traceability | All 210 records have an owner and an acceptance design. Six source conflicts stay until they are actually resolved. |
| Multi-artifact correctness | Thirty cases bind original, native, export, PDF, and twin. No stale artifact is accepted as current. |
| Stitch-family coverage | Every one of the nine line groups and fourteen region families has positive and negative geometry, exported-byte tests, and scoped physical-quality tests before a parity claim. |
| Catalogue integrity | Every counted asset has documented rights, identity, category, coverage, and QC scope before a quantitative library or font claim. |
| Interactive edit latency | Target p95 at or below 200 ms over at least 200 edits on a declared 25,000-event fixture and named hardware. |
| Workflow benefit | A week-24 pilot may target a lower median preparation time only if a pre-registered blind physical-quality non-inferiority criterion passes. |

These are proposed targets. They are not measurements already achieved by this package.

## 12. Evaluation limits

Separate software conformance from human usability and physical process quality. Use fixed licensed or original logos, photos, small text, hole-rich shapes, quilt blocks, and multi-hoop cases. Compare them with a declared manual baseline at the same size, material, and configuration. Split calibration from held-out evaluation. The register checker does not measure stitch quality.

The audit checks the shape of a scoped evidence record. Names, reviewer identifiers, and SHA-256 strings in an editable JSON document are not digital signatures. The checker does not connect to a device, inspect artifact contents, verify an account, or authorise a job. `ACCEPTED_SCOPED` would mean the configured operations passed on at least one named profile. It would not mean all machines, operating systems, or formats are supported. No row has that status.

## 13. What this checkout verified

Run the checker from the repository root:

```text
.venv/bin/python -m unittest tests.capability_register.test_audit -v
.venv/bin/python tools/audit_capabilities.py
.venv/bin/python tools/audit_capabilities.py --require-acceptance
```

The structural audit is expected to exit 0. The acceptance gate is expected to exit 1, with zero of 210 rows accepted. Details of the run on this checkout are in [professional-baseline-0.3-validation.md](professional-baseline-0.3-validation.md).
