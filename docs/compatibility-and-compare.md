# EmbroideryOS Universal Compatibility and Compare Contract, draft 0.2

Date: 11 October 2026. Repository: FrankAsanteVanLaarhoven/DST-Embroidery-AI.
Inspected main commit: 98303601416a3b0d68eae37e939031732bd77c45.
Status: requirements and developer utility, NOT universal compatibility qualification, not an ISO standard, and not a merged application change.

**Checkout note, 11 October 2026.** This file is the supplied contract, unchanged in its obligations. The coverage register remains `not_qualified` for every row: recorded is not passed. This checkout also wires a local Compare workspace onto TwinLab M1. That workspace binds an original, the editable design, the exact Rust DST, a reopened PDF, and the machine twin to one revision. It does not qualify XP, Vista, a scanner, a printer, a tablet, another embroidery format, an AI provider, or physical scale. The M1 native compile path was executed in this checkout and is recorded in `docs/twinlab-m1.md`. Sentences below that say the native gate was not rerun describe the preparation package that produced this contract.

## 1. Basis and scope

The user's supplied DRAWings/Wings minimum and recommended hardware lists are preserved as a historical acceptance baseline. The exact product/version for those values was not established. Do not silently replace 800 MB with 1 GB, 128 MB with 512 MB, or the user's recommended 1.5 GHz with another version's value.

The official DRAWings 8 and current DRAWings PRO XII pages have different requirements [S1, S2]. The current XII page specifies 6 GB RAM and 1.5 GB disk, a Windows DirectX 11/1 GB graphics requirement, Windows 10/11 and recent macOS. These provide a separate comparison, not a correction to the user's pasted baseline. Hardware requirements do not establish the complete feature set or digitising quality of any competitor.

The user's existing project brief separately requires a native parametric document, deterministic Rust generation, bounded AI proposals, source assets, multiple import/export adapters and reproducible outcome evaluation. No requirement in this document removes the universal/general-purpose product goal.

Five independent compatibility dimensions must be recorded: (1) computer OS/architecture/browser/runtime, (2) peripheral driver/API, (3) embroidery machine/controller/attachment, (4) format/parser/exporter, and (5) AI provider/model/tool contract. Dell, Apple, Linux, Windows and an AI service are not equivalent kinds of compatibility identifier.

For this embroidery workflow, `DTS` is treated as a likely reference to `DST`, not as a new supported format. A real .dts upload must retain its original name and pass content identification; it must not be automatically renamed or executed.

## 2. Source-derived baseline: retain every line

| Requirement | Supplied minimum | Supplied recommended | Our explicit coverage route |
|---|---|---|---|
| Processor | 1.5 GHz Intel/AMD | 1.5 GHz Intel/AMD | Low-resource qualification target; CPU frequency alone is insufficient. Record ISA/model/core count and workload. |
| RAM | XP 1 GB; Vista/7 2 GB; listed 64-bit Vista/7/8/10 3 GB | XP 2 GB; Vista/7/8/8.1/10 3 GB; listed 64-bit versions 4 GB | Qualify restricted capture/review or lightweight 2D as appropriate; do not promise full local AI/3D at this floor. |
| Graphics | DX9, 128 MB dedicated, 32-bit colour; Aero for Vista/7 | DX10, 1 GB dedicated, 32-bit colour; Aero for Vista/7 | Preserve as test-device descriptors; do not make DirectX, Aero or discrete VRAM universal requirements. |
| Disk | 800 MB uncompressed | 800 MB uncompressed | Target restricted core/helper package; account separately for OS, caches, projects, temporary files, CAD assets, AI weights and updates. |
| Optical drive | 4x DVD-ROM | 16x DVD-ROM | Optional offline installer transport. Verify same package identity as downloaded installation. No runtime DVD dependency. |
| OS | XP, Vista, 7, 8, 10; latest service packs | XP, Vista, 7/8/8.1/10; stated 32-/64-bit variants | Direct operation only when qualified. Legacy capture/export bridge or approved offline handoff otherwise. |
| Scanner | TWAIN; 254 dpi for 1:1; 300 dpi actual size; more optical resolution for enlargement | Current Windows-compatible scanner, >=300 dpi | Specific driver/backend and scale qualification; preserve stated sampling values without treating them as accuracy guarantees. |
| Digitiser | WinTAB driver | Windows-compatible tablet | Wintab or native pointer bridge, model-specific input tests; preserve pointer-only operation where pressure unavailable. |
| Printer | Windows driver | Windows-compatible laser/inkjet | OS/driver-qualified print path and dimensioned PDF; physical scale/colour checks separate. |
| USB security key | One free port when ordered with key | Same | No mandatory dongle by design; optional authorised entitlement adapter is distinct from scanner/machine ports. No lock bypass. |

The accompanying JSON register splits this table into 24 supplied-baseline obligations and independently records extension obligations. A recorded obligation is not an implemented or passed one.

## 3. Proposed execution tiers (not tested minimum requirements)

### L0: retained legacy workstation

Keep useful scanners, tablets, printers and machine-file transfer on old hosts. XP/Vista/7/8/8.1 and earlier computers require separately qualified legacy helpers or their existing vendor tools. A maintained gateway performs AI, authoritative compilation, rich rendering and verification. Earlier Windows/DOS, obsolete macOS/PowerPC or other early hardware are candidate file-exchange integrations, not blanket support promises.

No modern Node/Rust/web application is promised on these systems. Node 22.16.0's own platform matrix does not establish XP/Vista/7 support [S3]. Windows 10's ordinary support ended on 14 October 2025; exact ESU or LTSC/LTSB lifecycle applicability must be checked separately [S4]. A host's lifecycle is not inferred from its hardware brand.

Prefer offline capture/import or a controlled enclave with narrow exchange. Do not place cloud credentials on obsolete hosts, expose legacy remote desktop to the internet, enable SMB1 broadly, or disable driver signing to make a demo run. Quarantine and inspect file exchange, preserve output hashes, and use operator-mediated transfer. An isolated host is not automatically secure. No universal serial/floppy/punched-tape hardware adapter is assumed.

### L1: lightweight 2D client candidate

Proposed entry target: maintained OS/runtime, 2-4 GiB RAM and a functional 2D canvas, or a purpose-built helper. Use a 25,000-event and <=2-megapixel comparison fixture for the initial qualification. Defer expensive work to a qualified remote worker when configured; do not fabricate offline results. Keep original image, vector editing, exact exported trace, PDF review and printing usable without a discrete GPU. Older 1 GB hosts are capture/static-review candidates, not full modern-browser workstations.

Target bounded memory: active client increment <=512 MiB above the named browser/OS baseline on the declared fixture. Target limited helper/core installation <=800 MB uncompressed, excluding declared datasets/model packs; measure peak update and conversion disk usage separately. These budgets remain unmeasured.

### L2: interactive 3D studio candidate

Proposed entry target: >=8 GiB RAM and a tested GPU/WebGL2 path; 16 GiB recommended for larger scenes. CPU/driver/context and actual scene performance determine qualification, not a DirectX version string. WebGPU is optional. Fallback order is qualified GPU rendering, bounded software 3D, 2D, then static/remote review. GPU context loss must not lose the design or relax validation.

Use a declared 100,000-event / 250,000-triangle fixture for performance tests; level-of-detail can simplify display geometry but must not change the authoritative execution records or machine collision geometry. Simulation speed changes display time only.

### L3: engineering/local-AI workstation candidate

Start model-specific capacity planning at 16-32 GiB RAM, with GPU/unified memory determined by the selected model and measured workload. No universal VRAM minimum can cover all models. AI, large CAD conversion and calibrated physical solvers are optional compute services, not mandatory functions on the lowest tier. CUDA, Metal or other accelerated backends are individually qualified, not assumed interchangeable.

## 4. Modern OS and architecture matrix

Qualify named Windows releases on x86-64 and separately on ARM64; maintained macOS on Apple silicon and supported Intel hardware; named maintained Linux distributions on x86-64 and ARM64. Test packaged installation, native Rust, local service, browser engine, GPU driver, offline save/reopen, font rendering, printing and peripheral access. Build scripts and installers must work on each OS; a Bash-only acceptance script is not a Windows deliverable.

Test maintained Chromium, Firefox and Safari where available. A browser build does not imply identical USB, scanner, printing or GPU support. Prohibit user-agent-only qualification. Publish a dated matrix of exact tested OS, ISA, browser, engine build, GPU/driver, feature, workload and evidence hashes.

Release installers should not require users to install Cargo or a compiler. Developer build dependencies and end-user runtime requirements must be separate. Sign packages where supported, inventory dependencies, provide verified offline installation and define support expiry. Do not repackage third-party fonts or drivers without redistribution rights.

## 5. Peripherals and physical scale

Windows acquisition: separately qualified TWAIN Classic and WIA adapters. macOS acquisition: ImageCaptureCore. Linux acquisition: SANE backend. Network TWAIN Direct where the specific device implements it [S5-S8]. Cross-platform protocol design is not universal device support. Legacy 32-bit scanner sources may require an isolated 32-bit helper with a versioned interprocess contract; a 64-bit studio does not load a 32-bit driver directly.

WinTAB is a Windows tablet API [S9]. Preserve it through an appropriate helper; use browser pointer events/native pen paths for other environments, based on actual driver exposure [S10]. Test pressure, tilt, button mapping, coordinate range, DPI scaling, multi-monitor mapping, cancellation and stroke order. Pointer-only fallback remains available and clearly identified.

Printing: use each qualified OS print stack/driver, with IPP where supported. Preserve physical dimensions, colour-management intent and source fonts or outlined glyphs as permitted. Every actual-size proof has a 100 mm test bar, specified paper/crop dimensions, an Actual size/100% instruction and no silent fit-to-page shrink. Oversize designs require labelled tiled output or an explicit block. Monitor zoom cannot establish physical display scale without calibration.

Nominal sampling pitch is 25.4 / dpi mm/pixel. Thus 254 dpi gives 0.1 mm/pixel; 300 dpi gives approximately 0.08467 mm/pixel. A 2x enlargement halves effective dpi (300 becomes 150). These are sampling calculations, not scanner accuracy or embroidery-quality measurements. Missing/conflicting DPI, EXIF orientation, anisotropic scale and camera perspective require explicit handling. A photograph needs dimensional reference/control points; do not assume a 1:1 scan.

## 6. One Compare workspace; distinct evidence layers

Required panes/tabs: Original, Editable design, Machine file, PDF proof, Machine twin.

Original: immutable uploaded bytes plus safe derivative; source hash, rights, page/frame, DPI, colour profile, orientation and transform. Crops/background removal are new derivatives. Do not replace the original with an AI redraw.

Editable: native geometric/semantic objects, text and stitch parameters. Embroidery regions remain separate from exported machine movements. Typography/shaping and font substitution receive explicit warnings and review.

Machine-file view: decode the exact bytes for each selected format through its qualified adapter. Show stitch sequence, jumps, control records, dimensions and format-specific losses. Standard DST lacks the full native editable objects and the default export does not carry a complete source palette; retain sidecar provenance. Do not fabricate original colours or object semantics from a flat import. Extended data is preserved where the declared reader supports it.

PDF source and PDF proof are different artifact roles. PDF source uses page selection, page/crop boxes, rotation and a safe rendering derivative. PDF proof is generated from a frozen design/export/source binding and reopened through a PDF renderer to check the actual document [S11]. Neither is a machine-executable embroidery format. A PDF screenshot of artwork is not proof of DST correctness.

Machine twin: use the selected exact export trace, shared playback clock and machine-profile revision. No second frontend stitch engine. Raw record rehearsal and profile-interpreted commands remain distinguishable, especially trim/colour-stop semantics.

Compare controls: linked pan/zoom in millimetres, independent view option, alpha overlay, wipe, flicker, control-point registration, colour-layer isolation, command timeline and difference diagnostics. Changing source/registration changes the proof identity and relevant artwork approval; changing the design/profile invalidates the compiled artifact and replay. A cached preview never masquerades as a current export.

Difference displays are diagnostic, not single-number physical-quality scores. Compare intended coverage and boundaries with declared tolerances; needle-down traces and a filled source image are not pixel-identical representations. Separate raster/vector deviation, export quantisation and physical stitch-out error.

## 7. Format-specific coverage

Initial display/import targets: PNG, JPEG, WebP, TIFF, BMP; safe SVG; paged PDF; native JSON. The delivered proof utility handles selected raster formats and a PDF page derivative only. It does not implement a native studio asset library or SVG vector editing.

Initial machine export/replay target: existing Rust DST. Existing M1 remains native-gate blocked in the inspected evidence. EXP, PES/PEC, JEF, VP3, HUS, XXX, SEW, U01 and additional formats each require independent import, decode-view, export and model-acceptance status. Keep reserved/unrecognised commands visible or block them; never silently drop them. An imported format being readable does not certify its writer.

AI, EPS and DXF are separate adapter tasks. PDF-compatible AI is a possible subset, not universal .ai support. Rasterising a vector file for reference is not editable import. Font/layer/colour losses must be reported. Proprietary ART/EMB or other source formats require lawful access/implementation and declared subsets; no compatibility promise from a filename.

Files enter a bounded conversion worker with content validation, decompression/pixel/page limits, resource/time budgets, scripting disabled and external references/network fetches disabled by default. SVG/PDF active content, path traversal, zip bombs, malicious metadata and mixed-extension files are test cases. The delivered local proof CLI is NOT this hardened ingestion worker.

## 8. AI-platform interoperability

Use versioned HTTP/tool contracts and provider adapters, not screen automation of every chat product. Candidate providers include OpenAI, Anthropic, Google, xAI and local model runtimes. The local studio can call the xAI responses API for one geometry proposal when the studio process has `XAI_API_KEY`. That call is not a capability acceptance, writes no stitches, and is skipped when the key is absent. No other provider is integrated. Capability discovery must state image input, image generation, segmentation, vector proposals, structured output, streaming, limits, costs, retention policy and cancellation behaviour per configured model/version.

AI receives a source reference, target dimensions, rights scope, machine/material constraints and the engine capability manifest. It emits constrained proposals only. Validate source/object IDs, allowed stitch types, unsupported functions, budgets and stale revisions. Provider outages leave manual editing available. Tests must include malformed proposals, tool-output prompt injection, unsupported stitch families, timeouts and cost ceilings. Provider identity or self-reported confidence never grants approval or physical machine control.

## 9. Standards and conformance

Working project name: EmbroideryOS Compatibility and Interoperability Profile, draft 0.2. This is a project specification, not an ISO publication or universal certification. ISO 23247-1:2021 provides manufacturing-twin framework context; ISO/IEC 25010:2023 provides a product quality model [S12, S13]. Public abstracts support that mapping, not clause-by-clause conformity. Formal standardisation requires an appropriate consensus process.

Qualification status must be assigned per operation: documented, implemented, software-tested, reference-file-tested, device-tested, physically-validated. Calibration qualification includes instance/configuration, instruments, uncertainty, domain and expiry; none follows from file parsing. A CPU brand, `.dst` extension, TWAIN label or successful demo is not sufficient evidence.

## 10. Experiment, status and next gates

Hypothesis: separating authoring/authoritative computation from device-specific I/O, and binding every view to its actual artifact, preserves legacy workflow access while reducing source/export confusion.

Design: differential golden-file tests; OS/ISA/browser/GPU/peripheral coverage; device-profile negative tests; independent source/scale registration tests; low-resource memory/performance trials; paired operator review of original, native design, export, proof and stitch-out. Hold software correctness separate from physical quality. Record hardware/driver versions, test corpus, engine revision and failures.

Initial results: see VALIDATION.md. Only the policy and local proof utility were exercised. M1 native gate was not rerun. Failures: float-exact test assumption and low-contrast PDF footer inherited state; both corrected and revalidated, without relaxing byte-binding or machine-release conditions.

Refinement: use numerical tolerances for derived physical units; explicitly set drawing state for proof annotations; keep source and export provenance labels visible. Further experiments: real native export proof; interactive Compare integration; OS/peripheral labs; held-out physical tests. New insight: transport hashes can bind a supplied trace to record bytes without independently proving that its reported coordinates were decoded correctly. A test explicitly preserves that limitation; the Rust native gate is still required.

Strength: existing deterministic kernel and trace binding. Weakness: native/OS/peripheral qualification remains missing. Opportunity: retained legacy workflow access with shared exact-file review. Threat: driver obsolescence, proprietary interfaces, licensing constraints and inflated support claims.

Proposed SMART gates, relative to approved sprint start:

1. Week 1: all 24 supplied-baseline obligations have an ID, route, owner role and acceptance evidence requirement; none marked passed without evidence.
2. Week 2 after M1 native clearance: source + native + exact DST + actual PDF + twin share correct revision bindings on 30 cases; every stale/mismatched case blocks current approval.
3. Week 4: 100 golden DST cases replay to the expected quantised coordinates on each declared target architecture; other formats remain gated individually.
4. Week 6: L1 client's incremental peak memory <=512 MiB and p95 interactive response <=150 ms on declared 25,000-event/2-MP fixtures; no whole-OS memory claim from a process limit.
5. Week 6: restricted core/helper installation <=800 MB uncompressed; temporary/update peaks separately measured on declared 800 MB-free test conditions or reported infeasible.
6. Week 8: Windows x64, macOS ARM64 and Linux x64 acceptance suites pass; Intel Mac/Windows ARM64/Linux ARM64/legacy hosts require separate evidence, not inherited badges.
7. Before any production compatibility badge: each advertised peripheral/model/format operation has applicable evidence; digital 100 mm bars are correct, and real print/scan dimensions meet a declared measured tolerance (initial target +/-0.5 mm over 100 mm, uncertainty included).

Final conclusion: requirements can cover every requested class without asserting one executable delivers every capability on every old host. Direct-use parity at 1 GB RAM remains unproven and is not satisfied merely by gateway-assisted access. Record that distinction openly.

## Sources checked on 11 October 2026

S1. DRAWstitch, DRAWings 8 system requirements: https://www.drawstitch.com/Products/DRAWings_8/System_Requirements/
S2. DRAWstitch, DRAWings PRO XII system requirements: https://www.drawstitch.com/Products/DRAWingsXII/System_Requirements/
S3. Node.js 22.16.0, BUILDING.md: https://raw.githubusercontent.com/nodejs/node/v22.16.0/BUILDING.md
S4. Microsoft, Windows 10 release information: https://learn.microsoft.com/en-gb/windows/release-health/release-information
S5. TWAIN Working Group: https://twain.org/
S6. Microsoft, Windows Image Acquisition: https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wia/-wia-startpage
S7. Apple, ImageCaptureCore: https://developer.apple.com/documentation/imagecapturecore
S8. SANE project: https://sane-project.gitlab.io/website/
S9. Wacom, Wintab overview: https://developer-docs.wacom.com/docs/icbt/windows/wintab/wintab-overview/
S10. Wacom developer support, web pen input: https://developer-support.wacom.com/hc/en-us/articles/12844524637975-Wintab
S11. Mozilla, PDF.js: https://mozilla.github.io/pdf.js/
S12. ISO 23247-1:2021 public catalogue: https://www.iso.org/standard/75066.html
S13. ISO/IEC 25010:2023 public catalogue: https://www.iso.org/standard/78176.html
Repository baseline and M1 validation report were read separately. The original competitor-oriented brief is a requirements source, not independent proof of competitor limitations or of this platform's performance.
