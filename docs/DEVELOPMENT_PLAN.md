# QR Code Generator — Development-Ready Plan

**Based on:** `qr-generator-spec.md`, Draft v3  
**Repository state reviewed:** 2026-08-05  
**Plan status:** Ready to implement under the recorded oracle policy below

The detailed test architecture, selected libraries, quality gates, fuzz/mutation budgets, and desktop Chromium gate are defined in [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md). That document is part of this development plan rather than optional follow-up guidance.

## 1. Review outcome

The product direction is coherent and the repository is an appropriate Leptos 0.8 CSR scaffold. The implementation should not start by building the UI. Correctness depends first on freezing encoding behavior and building independently verified core fixtures under the provenance policy below; access to the complete normative standard is a valuable later audit input, not an implementation gate.

The repository state originally reviewed contained one Leptos binary and no workspace crates, QR implementation, test suite, or approved logo. It also loaded Google Fonts at runtime, which conflicted with an offline/self-contained internal tool posture and had to be removed or replaced with a bundled asset.

Development can proceed with the decisions in this document. Physical validation is performed manually outside the repository and is not collected as release evidence.

## 2. Decisions resolved by this review

### 2.1 Standards and table provenance

- ISO/IEC 18004:2024 remains the normative source for QR Code Model 2 behavior, but a licensed complete copy is not a repository or implementation prerequisite.
- When the complete standard text is unavailable, non-table algorithm rules may be implemented under the public-source corroboration procedure in [`research/qr-public-source-provenance.md`](research/qr-public-source-provenance.md). Public sources are evidence and test oracles, never relabelled as normative text.
- Each such rule must record its intended ISO clause/table topic, agree across two pinned independently maintained encoders where both expose it, have an independently written local invariant or slow reference where practical, and pass independently decoded end-to-end fixtures. A disagreement blocks acceptance; majority vote is not allowed. A disagreement may be resolved only by a narrowly recorded owner decision that defines the exact chosen semantics, preserves the disagreeing observations, requires an independent local reference, and still passes exact completed-matrix and independent-decode gates.
- Stable capacity, block, remainder-bit, alignment-pattern, and character-count tables may be implemented from committed development fixtures only after two pinned, independently maintained QR generators agree on every value they expose.
- Values exposed by only one generator must also satisfy an independently implemented structural invariant (for example, matrix function-module accounting for remainder bits).
- Public implementations remain development/test oracles. They are not production dependencies, and their implementation code is not copied into production.
- Production comments identify the applicable standard clause/topic plus the public-source evidence, oracle fixture, and pinned versions. If the exact 2024 clause number has not been verified against a complete copy, mark that citation `2024 clause mapping pending audit` rather than inventing precision. Later comparison with a licensed standard is an audit task and must not silently rewrite accepted fixtures.
- A table-validation test must verify dimensions, totals, and invariants for every version/ECC row before encoder work is accepted.

**Phase 0 gate (accepted by the project owner on 2026-08-05 and expanded on 2026-08-06):** development-only QR generators and public first-party source material are permitted for fixture creation and algorithm corroboration under the policy above.

**Mask-penalty Rule 3 decision (accepted by the project owner on 2026-08-06):** score only literal `00001011101` and `10111010000` sequences wholly inside each completed matrix row or column. Do not add virtual quiet-zone modules. python-qrcode 8.2 exposes this interpretation and an independently written slow reference must match it. Nayuki 1.8.0's differing run-history totals remain recorded as a named oracle exception rather than being discarded or treated as a vote.

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

### 2.4 Safe-workflow error-correction policy

Use ECC M for every non-logo release-1 workflow. ECC is displayed in diagnostics but is not user-selectable. Output profiles define only canvas dimensions and a maximum version; they do not silently change ECC. For an exact payload and selected profile, the workflow requests ECC M with Version 1 as its minimum and chooses the first fitting version up to the profile ceiling.

The compiled selectable profiles are Inline (100 px SVG / 300 px PNG, through
Version 6), Content (120/360, through Version 8), Landing (150/450, through
Version 12), Print (160/480, through Version 13), and the separate non-default
Adaptive profile through Version 40. Adaptive does not replace or silently
select any fixed profile; without the logo it follows the same ordinary ECC-M
first-fit rule and derives dimensions only after selecting the version.

The in-product practical guide records the exact maximum for typical ASCII
links that select whole-payload Byte mode. Total length includes scheme, host,
path, query, and fragment. The verified `(without logo / with logo)` limits are
Inline `106 / 58`, Content `152 / 58`, Landing `287 / 58`, Print `331 / 58`,
and Adaptive `2,331 / 137` bytes. Fixed branded limits reflect the approved
Version 6-only geometry; Adaptive branding reflects approval through Version
11. The guide explains that non-ASCII characters may occupy multiple UTF-8
bytes plus ECI overhead, QR-alphanumeric-only input can sometimes fit more,
and the exact preview result remains authoritative.

Enabling the bundled logo is the only release-1 transition that changes ECC: it changes the request to ECC H and an approved Version 6 minimum before version fitting, then recalculates the selected version and all capacity diagnostics. The selected version is the greater of the payload's first fit and the requested minimum, and an inverted minimum/maximum range is a typed error. Disabling the logo restores ECC M, the Version 1 minimum, and ordinary first fitting. The public `qr-core` encoder continues to accept all four ECC levels so conformance tests and future explicitly designed workflows are not constrained by the release-1 UI policy.

### 2.5 PNG renderer

Use a direct RGBA buffer renderer in `qr-render`, then serialize it with the Rust `png` crate. Do not use Canvas, browser SVG screenshots, or a general scene renderer for production PNG export.

- Finder cells and logo-knockout cells are filled by exact integer pixel
  rectangles. Every other visible module uses the approved centered circular
  glyph described below.
- PNG dots use deterministic 8×8 final-pixel coverage inside the approved
  0.45-module circular envelope. Fully covered pixels use exact `#BD0F72`;
  contour pixels blend with an opaque background or retain brand RGB beneath
  partial alpha on a transparent background. This preserves a solid brand core
  and a visibly round edge without introducing a second foreground token. The
  complete image is never resized.
- The bundled PNG logo uses deterministic 4×4 final-pixel coverage inside its presentation box so diagonal artwork edges remain smooth; finder and knockout edges remain pixel-sharp.
- Fixed profiles retain their compiled dimensions and 3× PNG relationship.
  Adaptive uses the selected matrix plus the four-module quiet zone to derive a
  four-pixel-per-logical-module SVG side and a six-pixel-per-logical-module PNG
  side. This keeps 0.45-module compact dots visible when Chromium displays the
  SVG at its declared size; the PNG retains an integer six-pixel module scale
  and needs no surplus padding.
- Direct RGBA buffers have a target-independent defensive ceiling of 64 MiB; requests above it fail with a typed error before allocation.
- PNG encoder settings, filter, compression, color type, bit depth, and metadata policy are explicit and covered by a byte-for-byte determinism test.
- SVG is generated directly from the render model with stable path ordering
  and numeric formatting. Compact modules remain true circular arc paths with
  the exact brand fill and normal edge antialiasing; forcing crisp-edge shape
  rendering is prohibited because it rasterizes small circles as diamonds.

This keeps the pixel geometry testable on native Rust and WASM and avoids browser-dependent rasterization.

### 2.6 Branding safety defaults

These release-1 defaults use the approved ONE treatment. The decode-backed
geometry below was accepted on 2026-08-09 for implementation by Tickets 24–29
and extended on 2026-08-10 by Ticket 30;
the production renderer applies the selected dot treatment, the unchanged
fixed-profile Version 6 placement, and the decode-backed adaptive placements.

- `#BD0F72` is the only QR foreground, on opaque white by default. There is no black-output preset or hidden release-1 configuration path.
- Visible data, remainder, timing, alignment, format, version, and fixed-dark
  modules use exactly centered circular glyphs with a diameter of `0.45`
  module. Encoded values and module coordinates are unchanged.
- All three 7×7 finder regions remain full-cell square patterns. Separator
  modules remain blank. The closer-reference non-finder dot treatment is
  approved; the conservative square-function treatment remains recorded as a
  passing experiment control but is not the selected branded appearance.
- Transparent background: supported as a caution, with export evaluated against white, light gray, and the documented dark/patterned previews. It is never the default.
- Module strokes and decorative borders: excluded from the product. Surplus fixed-canvas padding remains background-only.
- Finder styling: standard square only.

**Launch decisions accepted by the project owner on 2026-08-07 and revised on
2026-08-09:** release 1 uses only the magenta ONE foreground, the 4.5:1 opaque
contrast threshold, optional no-logo transparency as a caution, 0.45-module
centered dots outside the full-size square finder regions, and the bundled ONE
lettermark described below. Rounded modules remain excluded. The complete
candidate evidence is committed in
[`generated/branded-geometry-policy.json`](generated/branded-geometry-policy.json).

### 2.7 Logo safety

- Enabling the logo sets ECC H before version selection, so capacity/version is recalculated first.
- Geometry is selected after H-level fitting in module coordinates. Logo mode
  selects at least Version 6: candidate minima 4 and 5 admitted at most a
  11-module centered source width, while Version 6 was the first to admit the
  requested 13-module visual hierarchy. The payload is preserved byte-for-byte;
  version selection alone changes.
- Version 6 uses the unchanged `180 180 640 240` asset presentation box at
  exactly `13 × 4.875` modules, centered on both matrix axes at
  ten-thousandth-module precision. Its outward-snapped
  opaque-white knockout is `(left 13, top 17, width 15, height 7)`, clears the
  nearest protected module by six modules, and obscures 105 data modules and
  zero remainder modules. Every integer source width from 10 through 13
  modules decoded 48/48 native-PNG and SVG-rasterized samples across Inline,
  Content, Landing, and Print. Inline uses a 100 px SVG / 300 px PNG canvas and
  a Version 6 ceiling, retaining a six-pixel PNG module scale for the branded
  symbol. Every integer width from 14 through
  18 exceeded the checked 40%-of-matrix knockout bound. A 13-module source is therefore the
  largest admitted centered ONE treatment.
- Inline, Content, Landing, and Print retain that exact Version 6-only policy.
  Versions 7–13 on those fixed profiles intentionally reject branding because
  exact centering intersects protected central alignment geometry.
- Adaptive admits Versions 6–11 with dimensions derived from the selected
  version. Version 10 uses a 260 px SVG / 390 px PNG canvas.
  Version 10 has a 57-module matrix, a 65-module logical extent including the
  quiet zone, a six-pixel PNG module scale, a 390 px rendered symbol, and no
  surplus padding. Its Version 10 placement keeps the
  13×4.875-module source horizontally centered and shifts it six modules upward
  to `(left 22, top 20.0625)`, with a function-safe `(21, 19, 15, 7)` knockout.
  Version 11 uses 276/414 dimensions and the same source size shifted six
  modules upward at `(24, 22.0625)`, with knockout `(23, 21, 15, 7)`.
- Adaptive placement tries the exact center first, then searches integer module
  offsets in increasing Euclidean distance with a stable center/above/below/
  left/right tie order. It considers source widths from 13 down through the
  reviewed 10-module legibility floor and selects the largest safe candidate.
  Versions 7–11 select the nearest placement six modules upward; upward wins
  the equal-distance tie with the equally decodable placement below center.
  The focused committed experiment in
  [`generated/adaptive-branded-placement-policy.json`](generated/adaptive-branded-placement-policy.json)
  records 10–13-module candidates above, centered, and below: all 112
  function-safe native-PNG/rasterized-SVG artifacts decoded, while every
  centered Version 10 candidate was rejected before decoding for protected
  alignment-module overlap.
- Versions 1–5 remain below the branded minimum. Versions 12–40 return a typed
  unsafe-logo-geometry rejection until separately approved decode evidence is
  committed; users can disable the logo without changing the payload. Logo
  output stays classified as a caution on every valid profile/version row.
- The knockout must not intersect any function module: finder, separator, timing, alignment, format, version, or fixed-dark module. A conflict is `Invalid`, not merely a warning.
- Overlapped data and remainder modules are counted and reported. Logo mode remains a caution even when valid.
- The renderer compile-time embeds the sanitized project-owned ONE lettermark at `assets/RGB-one-lettermark-magenta.svg`. No upload, arbitrary SVG, white-logo variant, or runtime logo request is accepted in release 1.
- Replacing or editing the lettermark requires recorded license/provenance, sanitization, and the complete structural, deterministic, geometry, and independent-decode logo suite.
- Logo mode requires an opaque white background and knockout; transparency remains available only without the logo.
- The bundled logo option is selected by default. Users may turn it off to restore ECC M and transparent-background availability.
- Exact centering remains mandatory for the four fixed profiles. Adaptive alone
  may use the reviewed deterministic nearby search. The selected-version
  dimensions and generated evidence are recorded in
  [`generated/logo-placement-policy.md`](generated/logo-placement-policy.md).
- If geometry is unsafe for the selected version, logo mode is disabled with a
  reason. The encoder applies the approved Version 6 branded minimum but never
  selects a still-larger version merely to search for logo space.

ECC percentages are not used as an occlusion budget. Decode testing is mandatory for every enabled logo/profile/version fixture.

Release evidence exhausts the selectable surface with 436 generated scenarios:
120 required-payload rows and 316 exact-version rows. Native PNG and independently
rasterized SVG artifacts share one scenario identity and record deterministic
hashes, safety, decode outcome, and fixed/adaptive logo geometry. The resulting policy
has 254 accepted rows and 182 typed expected rejections. Deterministic adverse
evidence separately records 39 outcomes across explicit safe opaque-Print,
transparent-caution, centered-logo-caution, and Adaptive-Version-10 and
Version-11 long-URL caution pass envelopes; it does not imply
that every compact-dot density passes every transform.

The exported symbol always retains exactly four quiet-zone modules per side.
Decorative export borders, frames, labels, and strokes remain excluded; fixed
PNG canvas surplus is background-only.

## 3. Specification corrections required

These are implementation interpretations until merged back into the product specification.

1. **No border layer and explicit SVG sizing:** Decorative borders, frames, labels, and module strokes are excluded. PNG surplus padding remains blank/background-only. Fixed-profile SVG `width` and `height` equal the compiled base dimensions; Adaptive dimensions are derived from the selected logical extent. Every SVG `viewBox` is the tight logical extent of the QR matrix plus exactly four quiet-zone modules on every side and contains no fixed-canvas surplus padding. Consumers scale the vector through `width` and `height`. No border types, render options, controls, errors, or tests should be scaffolded for possible future use.
2. **Capacity diagnostics:** Display exact `used data bits / available data bits` and data codewords. “Remaining capacity” means additional characters in the currently selected whole-payload mode, computed by the same fit function; label it as an estimate for edits that could change mode.
3. **Function-module protection:** Branding and logo knockout never modify function modules. The spec's general “protect function patterns” goal takes precedence over language that only makes finder overlap explicitly invalid.
4. **Mask evaluation:** Apply each mask only to data/remainder modules, write the corresponding format bits, then score the complete final matrix. Choose the lowest score and lower mask ID on a tie.
5. **Remainder bits:** Capacity tables and placement must explicitly include the standard remainder-bit count per version. Every non-function matrix cell must be assigned once, including remainder bits.
6. **Input safety:** Plain text is allowed; URL syntax is not required. The UI may identify likely URLs, but it must not rewrite them. Empty input and over-limit input are invalid. Control characters receive a caution unless product policy later forbids them.
7. **External network calls:** Production HTML must not request Google Fonts or other remote UI assets. Bundle approved assets or use the system font stack.
8. **Print guidance:** The 160 px value is a design canvas, not a physical-size guarantee. Export remains SVG-first and the UI displays “place at 25–30 mm or larger; validate for the actual environment.”
9. **Variant-choice guidance:** The practical guide distinguishes the four
   predictable fixed-dimension contracts from Adaptive's payload-derived
   dimensions. It explains each fixed profile's intended placement and ceiling,
   Adaptive's first-fit version and four-module-quiet-zone sizing, and the logo
   placement tradeoff: centered at Version 6, shifted six modules upward for
   Adaptive Versions 7–11, and rejected at Version 12 or higher until approved.

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
pub struct EncodeRequest<'a> { /* private fields */ }

impl<'a> EncodeRequest<'a> {
    pub const fn first_fit(
        text: &'a str,
        ecc: ErrorCorrection,
        max_version: Version,
    ) -> Self;
    pub const fn with_version_range(
        text: &'a str,
        ecc: ErrorCorrection,
        min_version: Version,
        max_version: Version,
    ) -> Self;
}

pub struct EncodedQr {
    pub version: Version,
    pub ecc: ErrorCorrection,
    pub mode: DataMode,
    pub mask: MaskId,
    pub data_bits_used: u32,
    pub data_bits_capacity: u32,
    pub minimum_version_increased_selection: bool,
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
- `serde` only if configuration serialization is actually needed; compiled Rust constants are preferred for the five profiles.
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
- ZXing-C++ as the primary independent decode oracle.
- `quirc` as a second decoder for representative raster cases where its text/ECI behavior is applicable.
- Nayuki QR Code Generator 1.8.0 and `python-qrcode` 8.2 create development fixtures only after owner approval. Their explicit-version/mask outputs are compared, not linked into production or copied as implementation source. Segno 1.6.6 was evaluated and rejected for this role after its byte-aligned padding output disagreed with Nayuki; the rejected matrix was not committed.

Additional local verification may use `cargo-llvm-cov`, `cargo-mutants`, Miri, `cargo-audit`, and Playwright Test. See the testing strategy for the rationale and enforcement thresholds.

Browser tooling uses the `.nvmrc`-declared Node.js v24 runtime and the
`packageManager`-pinned pnpm release. TypeScript is linted with Oxlint and
formatted with Oxfmt. Development-only Python support code is linted and
formatted with Ruff and type checked with ty; those tools are exact-pinned in
the oracle `uv.lock`. These checks must run without weakening QR oracle or
fixture policies.

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

**Exit:** safe SVG/PNG outputs for every profile and allowed version satisfy geometry assertions and decode gates; maximum versions retain 6 px/module.

### M3 — Functional Leptos workflow (1–2 weeks, risk: low)

- Payload input with character and byte counts.
- Five profile cards and derived version/capacity state; diagnostics show the fixed safe ECC M or logo-triggered ECC H.
- Debounced preview and accessible validation announcements.
- Diagnostics panel with exact geometry and warnings.
- SVG and PNG Blob downloads with fixed safe filenames.
- Export disabled on invalid state; errors do not expose payloads to logs or DOM metadata.

**Exit:** the complete safe-preset workflow works offline in desktop Chromium and passes keyboard smoke tests.

### M4 — Approved branding and logo (1.5–3.5 weeks, risk: high)

- Apply the owner-approved launch preset list.
- Implement contrast classification using the approved measurable thresholds.
- Apply the single 0.45-module non-finder-dot treatment with standard square
  finders.
- Integrate sanitized bundled logo, knockout, function-overlap validation, and overlap diagnostics.
- Add transparency surface previews and exhaustive approved-combination decode tests.

**Exit:** every selectable combination passes its required decode suite; unsafe combinations cannot be selected or exported.

### M5 — Release hardening (1–2 weeks, risk: medium)

- Run sustained fuzzing and dependency/license review.
- Execute adverse raster transformations with documented thresholds.
- Complete the release runbook, user guidance, and local production-build privacy inspection.

**Exit:** all automated acceptance criteria have linked evidence; the local production build makes no payload/logo request and logs no payload. Manual product checks remain outside the evidence system.

**Total expected engineering effort:** roughly 8–13 developer-weeks, with core conformance and logo decode validation carrying most uncertainty. Removing borders saves a UI/configuration branch, a render layer, SVG security cases, and one dimension from the branding test matrix; it does not materially reduce encoder-core risk.

## 8. Local verification

The repository documents local commands for formatting, warnings-as-errors linting, native tests, WASM checking, and an optimized Trunk build. The extended suites in [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md) are run locally when their related implementation exists, with longer fuzz, mutation, browser, and adverse-image checks performed during release hardening.

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
19. Compact 0.45-module non-finder dots with standard square finders.
20. Bundled magenta ONE lettermark, knockout, overlap checks, and decode matrix.
21. Hardening, automated release evidence, and docs.

Tickets 3–11 should generally land sequentially because later code relies on earlier invariants. UI shell work may run in parallel after ticket 2, but export controls must not be presented as functional until the core and safe renderer pass their gates.

## 10. Owner coordination still needed

The implementation policy, launch presets, ECC behavior, contrast threshold, transparency behavior, styling set, and bundled ONE lettermark are accepted. Access to a licensed complete ISO/IEC 18004:2024 copy remains useful for a later audit but is not an implementation gate under the public-source corroboration policy in section 2.1.

Manual browser, device, scanner, printer, material, and placement checks are owner-operated outside the repository and do not block the automated M5/ticket 21 evidence gate.

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
