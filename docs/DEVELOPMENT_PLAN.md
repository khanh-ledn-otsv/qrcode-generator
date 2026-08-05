# QR Code Generator — Development-Ready Plan

**Based on:** `qr-generator-spec.md`, Draft v3  
**Repository state reviewed:** 2026-08-05  
**Plan status:** Ready to implement under the recorded oracle policy below

The detailed test architecture, selected libraries, quality gates, fuzz/mutation budgets, and browser matrix are defined in [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md). That document is part of this development plan rather than optional follow-up guidance.

## 1. Review outcome

The product direction is coherent and the repository is an appropriate Leptos 0.8 CSR scaffold. The implementation should not start by building the UI. Correctness depends first on acquiring the normative standard, freezing encoding behavior, and building independently verified core fixtures.

The repository state originally reviewed contained one Leptos binary and no workspace crates, QR implementation, test suite, or approved logo. It also loaded Google Fonts at runtime, which conflicted with an offline/self-contained internal tool posture and had to be removed or replaced with a bundled asset.

Development can proceed with the decisions in this document. Items explicitly marked **Owner gate** require confirmation before their associated phase, but do not block unrelated earlier work.

## 2. Decisions resolved by this review

### 2.1 Standards and table provenance

- ISO/IEC 18004:2024 remains the normative source for QR Code Model 2 behavior, but a licensed complete copy is not a repository or implementation prerequisite.
- Stable capacity, block, remainder-bit, alignment-pattern, and character-count tables may be implemented from committed development fixtures only after two pinned, independently maintained QR generators agree on every value they expose.
- Values exposed by only one generator must also satisfy an independently implemented structural invariant (for example, matrix function-module accounting for remainder bits).
- Public implementations remain development/test oracles. They are not production dependencies, and their implementation code is not copied into production.
- Production comments identify the applicable standard clauses plus the oracle fixture and pinned versions. Later comparison with a licensed standard is an audit task and must not silently rewrite accepted fixtures.
- A table-validation test must verify dimensions, totals, and invariants for every version/ECC row before encoder work is accepted.

**Phase 0 gate (accepted by the project owner on 2026-08-05):** development-only QR generators are permitted for fixture creation under the dual-oracle provenance policy above.

### 2.2 Text, byte mode, and ECI

Use this deterministic first-release policy:

1. Preserve the exact input string; do not silently trim or normalize it.
2. Empty input is invalid.
3. Select Numeric only when every character is ASCII `0`–`9`.
4. Otherwise select Alphanumeric only when every character is in the QR alphanumeric set (`0–9`, `A–Z`, space, `$%*+-./:`).
5. Otherwise use Byte mode.
6. For ASCII-only Byte payloads, emit the UTF-8/ASCII bytes without ECI.
7. For any non-ASCII payload, emit ECI assignment 26 followed by UTF-8 Byte mode.
8. In Byte mode, the character-count indicator contains the number of encoded bytes, not Unicode scalar values or grapheme count.
9. Reject input whose encoded bitstream cannot fit Version 40, and impose a defensive input limit of 4 KiB of UTF-8 before encoding.

This policy is standards-explicit for non-ASCII and avoids spending ECI bits for ASCII URLs. Scanner compatibility for ECI 26 is a release-test item; it is not a reason to emit ambiguous non-ASCII bytes.

### 2.3 Segmentation

Use one data mode for the complete payload in release 1. ECI is a control segment followed by the one data segment, not mixed data-mode optimization. Version selection must account for the actual ECI and data bits.

Mixed-mode dynamic programming is deferred. Add it later only if telemetry-free testing of representative internal URLs shows a meaningful reduction in rejected profiles or selected versions. Introducing it later changes output matrices but not decoded payloads, so deterministic golden fixtures must be versioned if it is added.

### 2.4 PNG renderer

Use a direct RGBA buffer renderer in `qr-render`, then serialize it with the Rust `png` crate. Do not use Canvas, browser SVG screenshots, or a general scene renderer for production PNG export.

- Square cells are filled by exact integer pixel rectangles.
- Rounded and dot cells use deterministic final-pixel coverage evaluation; the complete image is never resized.
- PNG encoder settings, filter, compression, color type, bit depth, and metadata policy are explicit and covered by a byte-for-byte determinism test.
- SVG is generated directly from the render model with stable path ordering and numeric formatting.

This keeps the pixel geometry testable on native Rust and WASM and avoids browser-dependent rasterization.

### 2.5 Initial branding safety defaults

These defaults make implementation testable while product-specific artwork is pending:

- Safe preset: black foreground, opaque white background, square data modules, square function modules, standard square finders, no border, no logo.
- Brand preset: `#BD0F72` foreground on opaque white; it must pass the contrast rule and decode suite.
- Rounded data modules: maximum radius 25% of a module cell. Function modules remain square except for an explicitly tested finder preset.
- Dot modules: deferred from release 1 unless the owner makes them a launch requirement before Phase 3.
- Transparent background: supported as a caution, with export evaluated against white, light gray, and the documented dark/patterned previews. It is never the default.
- Module strokes and decorative borders: excluded from the product. Surplus fixed-canvas padding remains background-only.
- Finder styling: standard square at launch; any rounded finder becomes a separately named preset and must pass the full decode matrix.

**Owner gate before Phase 3:** approve the launch preset list, contrast thresholds, and bundled logo asset.

### 2.6 Logo safety

- Enabling the logo sets ECC H before version selection, so capacity/version is recalculated first.
- The logo plus knockout is centered, uses one module of padding initially, and is at most 20% of matrix width.
- The knockout must not intersect any function module: finder, separator, timing, alignment, format, version, or fixed-dark module. A conflict is `Invalid`, not merely a warning.
- Overlapped data and remainder modules are counted and reported. Logo mode remains a caution even when valid.
- The renderer uses one bundled, sanitized asset. No upload or arbitrary SVG is accepted in release 1.
- On a transparent QR background, the knockout remains opaque white.
- If geometry is unsafe for the selected version, logo mode is disabled with a reason. The encoder must not force a larger version merely to create logo space.

ECC percentages are not used as an occlusion budget. Decode testing is mandatory for every enabled logo/profile/version fixture.

## 3. Specification corrections required

These are implementation interpretations until merged back into the product specification.

1. **No border layer:** Decorative borders, frames, labels, and module strokes are excluded. PNG surplus padding remains blank/background-only, and the SVG viewBox covers only the QR symbol including its quiet zone. No border types, render options, controls, errors, or tests should be scaffolded for possible future use.
2. **Capacity diagnostics:** Display exact `used data bits / available data bits` and data codewords. “Remaining capacity” means additional characters in the currently selected whole-payload mode, computed by the same fit function; label it as an estimate for edits that could change mode.
3. **Function-module protection:** Branding and logo knockout never modify function modules. The spec's general “protect function patterns” goal takes precedence over language that only makes finder overlap explicitly invalid.
4. **Mask evaluation:** Apply each mask only to data/remainder modules, write the corresponding format bits, then score the complete final matrix. Choose the lowest score and lower mask ID on a tie.
5. **Remainder bits:** Capacity tables and placement must explicitly include the standard remainder-bit count per version. Every non-function matrix cell must be assigned once, including remainder bits.
6. **Input safety:** Plain text is allowed; URL syntax is not required. The UI may identify likely URLs, but it must not rewrite them. Empty input and over-limit input are invalid. Control characters receive a caution unless product policy later forbids them.
7. **External network calls:** Production HTML must not request Google Fonts or other remote UI assets. Bundle approved assets or use the system font stack.
8. **Performance targets:** Treat the 250 KB compressed WASM target as a measurement target, not a release gate, until the Phase 1 browser spike measures Leptos plus PNG support.
9. **Print guidance:** The 160 px value is a design canvas, not a physical-size guarantee. Export remains SVG-first and the UI displays “place at 25–30 mm or larger; validate for the actual environment.”

## 4. Target architecture

Convert the repository to a Cargo workspace:

```text
.
├── Cargo.toml                 # workspace and shared dependency pins
├── crates/
│   ├── qr-core/
│   │   ├── src/bit_buffer.rs
│   │   ├── src/mode.rs
│   │   ├── src/tables.rs
│   │   ├── src/gf256.rs
│   │   ├── src/reed_solomon.rs
│   │   ├── src/matrix.rs
│   │   ├── src/mask.rs
│   │   ├── src/penalty.rs
│   │   └── src/lib.rs
│   ├── qr-render/
│   │   ├── src/profile.rs
│   │   ├── src/model.rs
│   │   ├── src/svg.rs
│   │   ├── src/raster.rs
│   │   ├── src/png.rs
│   │   └── src/lib.rs
│   └── qr-web/
│       ├── index.html
│       ├── input.css
│       ├── src/app.rs
│       ├── src/components/
│       ├── src/download.rs
│       ├── src/state.rs
│       ├── src/main.rs
│       └── assets/
├── tests/fixtures/            # provenance manifest + redistributable fixtures
├── fuzz/
└── Trunk.toml                 # root command entry point targeting qr-web
```

Dependency direction is strictly:

```text
qr-web -> qr-render -> qr-core
qr-web ------------> qr-core
```

`qr-core` and `qr-render` compile and test natively. Browser-only types and download APIs stay in `qr-web`.

### 4.1 Core public boundary

Separate encoding from branded generation:

```rust
pub struct EncodeRequest<'a> {
    pub text: &'a str,
    pub ecc: ErrorCorrection,
    pub max_version: Version,
}

pub struct EncodedQr {
    pub version: Version,
    pub ecc: ErrorCorrection,
    pub mode: DataMode,
    pub mask: MaskId,
    pub data_bits_used: u32,
    pub data_bits_capacity: u32,
    pub modules: ModuleMatrix,
}

pub enum ModuleKind {
    Data,
    Remainder,
    Finder,
    Separator,
    Timing,
    Alignment,
    Format,
    Version,
    Dark,
}
```

`ModuleMatrix` owns a row-major immutable collection of `{ dark: bool, kind: ModuleKind }` after construction. Construction uses a checked mutable builder that rejects double writes and unfilled cells.

`qr-render` accepts `&EncodedQr` plus a validated `RenderOptions`. It cannot encode a payload or change ECC/version.

### 4.2 Error model

Use typed errors and checked arithmetic. At minimum:

- `EmptyPayload`
- `InputLimitExceeded`
- `PayloadTooLargeForProfile { required, maximum }`
- `PayloadTooLargeForQr`
- `InvalidProfile`
- `TableInvariantViolation`
- `MatrixInvariantViolation`
- `UnsafeContrast`
- `UnsafeLogoGeometry`
- `DimensionOverflow`
- `RenderFailure`

User errors return to Leptos as validation state. Internal invariant errors disable export and show a generic failure without logging payload data. No user-controlled path uses `unwrap`, unchecked indexing, or panics across WASM.

## 5. Recommended dependencies

Keep versions exact in the workspace manifest and update dependencies deliberately.

### Production

- `leptos = =0.8.20` with `csr` for the web crate.
- `wasm-bindgen`, `web-sys`, and `js-sys` only in `qr-web` for Blob/URL/download integration.
- `serde` only if configuration serialization is actually needed; compiled Rust constants are preferred for four profiles.
- `thiserror` for typed errors if its WASM size is acceptable; otherwise implement `Display` manually.
- `png` 0.18.x in `qr-render`, with only required features.
- A small timer utility for debounce only if Leptos/browser APIs do not already provide the needed lifecycle-safe timeout.

Do not add `image`, `tiny-skia`, `resvg`, a QR crate, or a QR Reed–Solomon crate to production dependencies.

### Development and test only

- `proptest` for fit, geometry, determinism, and matrix invariants.
- `cargo-fuzz`/libFuzzer for encoder and render entry points.
- `wasm-bindgen-test` for browser boundary tests.
- `resvg` for native test rasterization of exported SVG.
- `roxmltree` for SVG structure and security assertions.
- `image` for adverse-condition test transforms only.
- `insta` for normalized semantic snapshots, never as the primary matrix oracle.
- `criterion` for performance benchmarks without flaky wall-clock unit assertions.
- ZXing-C++ as the primary independent decode oracle.
- `quirc` as a second decoder for representative raster cases where its text/ECI behavior is applicable.
- Nayuki QR Code Generator 1.8.0 and `python-qrcode` 8.2 create development fixtures only after owner approval. Their explicit-version/mask outputs are compared, not linked into production or copied as implementation source. Segno 1.6.6 was evaluated and rejected for this role after its byte-aligned padding output disagreed with Nayuki; the rejected matrix was not committed.

Additional local verification may use `cargo-llvm-cov`, `cargo-mutants`, Miri, `cargo-audit`, Playwright Test, and `@axe-core/playwright`. See the testing strategy for the rationale and enforcement thresholds.

## 6. Test oracle and fixture strategy

Every committed fixture gets a manifest entry containing payload bytes (or a non-sensitive generated payload), mode/ECI policy, version, ECC, mask, source tool and version, generation command, and independent verification status.

Use four layers:

1. **Reference vectors:** Values from legally usable standards material or committed dual-oracle fixtures with pinned provenance.
2. **Golden matrices:** Generate explicit-version, explicit-ECC, explicit-mask matrices with two independent generators. Commit only fixtures on which both agree, or document why representation differs.
3. **Independent decoding:** Decode production PNG and rasterized SVG with pinned ZXing-C++. Compare decoded Unicode text and, where available, raw bytes/ECI metadata.
4. **Invariant/property tests:** Verify table totals, block lengths, reserved-cell ownership, full placement, format/version bits, profile geometry, deterministic bytes, and random round trips.

Golden coverage should include:

- Versions 1, 2, 6, 7, 9, 10, 26, 27, and 40.
- L/M/Q/H across the set, with every ECC represented at version boundaries.
- Every mask ID through explicit-mask core test hooks unavailable to product UI.
- Numeric, alphanumeric, ASCII byte, and UTF-8+ECI payloads.
- Boundary payloads at exactly-fit and one-unit-over capacity for each character-count version band.

The independent decode suite tests the safe preset exhaustively across all allowed profile versions. Styled combinations use a pairwise matrix plus explicit high-risk cases; the final approved launch combinations are fully enumerated.

## 7. Delivery milestones

Estimates are engineering effort for one experienced Rust developer and include tests/review, not calendar promises.

### M0 — Standards and repository foundation (2–4 days, risk: medium)

- Complete the Phase 0 oracle-policy gate.
- Convert the scaffold to the three-crate workspace.
- Pin toolchain and dependency versions; document local formatting, Clippy, native test, WASM check, and production-build commands.
- Remove remote font/network assets and scaffold local approved assets.
- Add the fixture provenance format and architecture decision records for ECI, segmentation, and oracle policy.

**Exit:** native and WASM skeletons build locally; no runtime network request is present.

### M1 — Standards-conformant encoder core (3–5 weeks, risk: high)

Order the work so each layer has an independent gate:

1. Version/ECC tables, invariants, bit buffer, modes, ECI, and exact fit calculation.
2. GF(256) arithmetic and Reed–Solomon vectors.
3. Block split/interleave and remainder bits.
4. Function pattern placement and classified matrix builder.
5. Data placement, all masks, format/version BCH, penalty rules, and deterministic selection.
6. Full golden, boundary, property, and decoder round-trip suite.

Add a small native CLI example for diagnostics, not as a shipped product.

**Exit:** all 40 versions and four ECC levels pass tables/invariants; representative golden matrices agree with two oracles; random safe-style rasters decode independently; no panics under fuzz smoke testing.

### M2 — Profiles and safe rendering (1.5–2.5 weeks, risk: medium)

- Implement compiled profile definitions and validate them at test time.
- Create the shared render model and exact canvas-placement calculation.
- Implement safe-preset consolidated SVG output.
- Implement direct RGBA and deterministic PNG serialization.
- Add quiet-zone, centering, integer-scale, dimensions, determinism, and decode tests.
- Prototype WASM bundle size and renderer performance; record actual baselines.

**Exit:** safe SVG/PNG outputs for every profile and allowed version satisfy geometry assertions and decode gates; maximum versions retain 6 px/module.

### M3 — Functional Leptos workflow (1–2 weeks, risk: low)

- Payload input with character and byte counts.
- Four profile cards and derived ECC/version/capacity state.
- Debounced preview and accessible validation announcements.
- Diagnostics panel with exact geometry and warnings.
- SVG and PNG Blob downloads with fixed safe filenames.
- Export disabled on invalid state; errors do not expose payloads to logs or DOM metadata.

**Exit:** the complete safe-preset workflow works offline in supported desktop/mobile browsers and passes keyboard/screen-reader smoke tests.

### M4 — Approved branding and logo (1.5–3.5 weeks, risk: high)

- Apply the owner-approved launch preset list.
- Implement contrast classification using the approved measurable thresholds.
- Add rounded modules/finder preset only as approved.
- Integrate sanitized bundled logo, knockout, function-overlap validation, and overlap diagnostics.
- Add transparency surface previews and exhaustive approved-combination decode tests.

**Exit:** every selectable combination passes its required decode suite; unsafe combinations cannot be selected or exported.

### M5 — Release hardening (1–2 weeks plus manual test logistics, risk: medium)

- Run sustained fuzzing, dependency/license review, performance profiling, and bundle analysis.
- Execute adverse raster transformations with documented thresholds.
- Complete the named browser/device/scanner/printer/placement matrix.
- Produce print samples and validate 25 mm and 30 mm placements.
- Complete the release runbook, user guidance, and local production-build privacy inspection.

**Exit:** all acceptance criteria have linked evidence; manual exceptions are signed off; the local production build makes no payload/logo request and logs no payload.

**Total expected engineering effort:** roughly 8–13 developer-weeks, with core conformance and logo decode validation carrying most uncertainty. Removing borders saves a UI/configuration branch, a render layer, SVG security cases, and one dimension from the branding test matrix; it does not materially reduce encoder-core risk.

## 8. Local verification

The repository documents local commands for formatting, warnings-as-errors linting, native tests, WASM checking, and an optimized Trunk build. The extended suites in [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md) are run locally when their related implementation exists, with longer fuzz, mutation, browser, adverse-image, and physical checks performed during release hardening.

Repository-owned automation and publishing are intentionally deferred. The owner will configure them separately later.

## 9. Development tickets in execution order

The following ticket slices are small enough for review and preserve dependency order:

1. Workspace conversion and offline local-verification baseline.
2. Profile types, validation, and geometry-only tests.
3. Bit buffer, mode detection, ECI 26, and character-count widths.
4. ISO capacity/block/remainder/alignment tables with invariants.
5. Version fit API and boundary fixtures.
6. GF(256) and Reed–Solomon generator/remainder implementation.
7. Block splitting and interleaving.
8. Classified matrix builder and function patterns.
9. Data/remainder placement and ownership checks.
10. Mask formulas, BCH values, penalties, and tie breaking.
11. End-to-end encoder API, goldens, properties, and fuzz targets.
12. Render model and fixed-canvas placement.
13. Safe consolidated SVG renderer.
14. Direct RGBA and PNG encoder.
15. Independent SVG/PNG decode harness.
16. Leptos state model, payload/profile workflow, and preview.
17. Diagnostics, validation, accessibility, and downloads.
18. Approved color/contrast and transparency previews.
19. Rounded/finder presets if approved.
20. Bundled logo, knockout, overlap checks, and decode matrix.
21. Hardening, performance, manual validation, release evidence, and docs.

Tickets 3–11 should generally land sequentially because later code relies on earlier invariants. UI shell work may run in parallel after ticket 2, but export controls must not be presented as functional until the core and safe renderer pass their gates.

## 10. Owner approvals still needed

These choices cannot be inferred safely from the technical specification:

1. Confirm purchase/access for ISO/IEC 18004:2024.
2. Confirm development-only generator libraries are permitted to create and cross-check fixtures.
3. Supply the sanitized, licensed launch logo and approve its white knockout appearance.
4. Approve the launch style list: recommended release 1 is Square + Rounded data modules, standard finder, and no Dot. Borders and module strokes are excluded.
5. Approve measurable color policy. Recommended starting rule: WCAG relative luminance contrast ratio at least 4.5:1 for selectable opaque presets, with final acceptance determined by decoding tests. Transparency is always a caution because effective contrast is unknowable.
6. Confirm that transparency is a real launch requirement; otherwise defer it and materially reduce the validation matrix.
7. Name the supported browser, iOS/Android device, scanner app, printer, stock/material, and placement environments.
Work through M2 can begin after approvals 1–2. M3 can use the safe preset. M4 cannot finish until approvals 3–7 are resolved.

## 11. Definition of done

A milestone is not complete when code merely produces a scannable sample. It is complete only when:

- normative rules have traceable tests;
- exact boundary behavior is tested;
- matrix ownership and render geometry invariants hold;
- output is decoded by the independent gate where applicable;
- native and WASM builds pass;
- deterministic outputs are verified;
- user-input failures are typed and non-panicking;
- privacy and no-runtime-network constraints are verified; and
- documentation and acceptance evidence are linked from the ticket or release record.

## 12. Technical references checked for this review

- [ISO/IEC 18004:2024 catalogue entry](https://www.iso.org/standard/83389.html)
- [ZXing-C++ repository and decoder documentation](https://github.com/zxing-cpp/zxing-cpp)
- [Nayuki QR Code Generator repository](https://github.com/nayuki/QR-Code-generator)
- [Rust `png` crate documentation](https://docs.rs/png/latest/png/)
