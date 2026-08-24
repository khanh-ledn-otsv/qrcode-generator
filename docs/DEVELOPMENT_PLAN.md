# QR Code Generator — Development-Ready Plan

## Agent metadata

- **Purpose:** accepted product behavior, architecture, dependency, and delivery
  decisions.
- **Read when:** changing QR semantics, rendering policy, architecture, public
  boundaries, dependencies, branding, or release assumptions.
- **Authority:** normative repository decision record. Update the affected
  section in the same change when implementation changes an accepted decision.
- **Execution warning:** this file does not select test commands. Use
  `docs/agents/verification.md`.

### Retrieval index

| Change topic | Read |
|---|---|
| standards, byte/ECI, segmentation, ECC | §§2.1–2.4 and §3 |
| PNG, branding, logo safety | §§2.5–2.7 |
| crate boundaries, APIs, errors | §4 |
| dependency choice | §5 |
| fixtures and independent oracles | §6 |
| delivery scope/status | §§7, 9–11 |
| verification architecture | §8, then `docs/agents/verification.md` |
| cited technical sources | §12 |

**Based on:** `qr-generator-spec.md`, Draft v3  
**Repository state reviewed:** 2026-08-05  
**Plan status:** Ready to implement under the recorded oracle policy below

The detailed test architecture, selected libraries, quality gates, fuzz/mutation budgets, and desktop Chromium gate are defined in [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md). That document is part of this development plan rather than optional follow-up guidance.

## 1. Review outcome

The product direction is coherent and the repository is an appropriate Leptos 0.8 CSR scaffold. The implementation should not start by building the UI. Correctness depends first on freezing encoding behavior and building independently verified core fixtures under the provenance policy below; access to the complete normative standard is a valuable later audit input, not an implementation gate.

The repository state originally reviewed contained one Leptos binary and no workspace crates, QR implementation, test suite, or approved logo. It also loaded Google Fonts at runtime, which conflicted with an offline/self-contained internal tool posture and had to be removed or replaced with a bundled asset.

Hosted Rust builds use pinned `sccache` 0.17.0 with the GitHub Actions backend;
local use is optional through `RUSTC_WRAPPER=sccache`. Development and test
profiles keep line-table debug information. Local builds retain Cargo
incremental compilation, while hosted jobs disable incremental artifacts in
favor of shared compiler-result and dependency caches. These choices reduce
link time and disk growth without changing release artifacts.

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

Use this deterministic text and ECI policy:

1. Preserve the exact input string; do not silently trim or normalize it.
2. Empty input is invalid.
3. Segment the payload across Numeric, Alphanumeric, and Byte modes with the
   version-aware minimum-bit policy in section 2.3.
6. For ASCII-only Byte payloads, emit the UTF-8/ASCII bytes without ECI.
7. For any non-ASCII payload, emit ECI assignment 26 followed by UTF-8 Byte mode.
8. In Byte mode, the character-count indicator contains the number of encoded bytes, not Unicode scalar values or grapheme count.
9. Reject input whose encoded bitstream cannot fit Version 40, and impose a defensive input limit of 4 KiB of UTF-8 before encoding.

This policy is standards-explicit for non-ASCII and avoids spending ECI bits for ASCII URLs. Scanner compatibility for ECI 26 is a release-test item; it is not a reason to emit ambiguous non-ASCII bytes.

### 2.3 Segmentation

Use version-aware dynamic programming to minimize the complete segment bit
length across Numeric, Alphanumeric, and Byte segments. The calculation
includes each mode indicator, the character-count width for the selected
version band, payload bits, and one UTF-8 ECI 26 control segment when the exact
input contains non-ASCII bytes. Segment boundaries remain UTF-8 boundaries.

Equal-bit plans resolve deterministically by fewer segments, then Numeric /
Alphanumeric / Byte mode priority, then the longer first segment; the same rule
recursively applies to the canonical suffix. Recompute the plan at the 9→10
and 26→27 version-band transitions. Pure whole-payload encodings retain their
previous codewords whenever that representation remains optimal. `EncodedQr`
exposes typed segment summaries and the web diagnostic reports `Mixed` rather
than attributing a mixed symbol to one mode.

### 2.4 Safe-workflow error-correction policy

Use ECC M for every non-logo release-1 workflow. ECC is displayed in diagnostics but is not user-selectable. Output profiles define only canvas dimensions and an approved version range; they do not silently change ECC. For an exact payload and selected profile, the workflow requests ECC M within that range.

The compiled selectable profiles are fixed artifact contracts: Small (100/300,
Versions 5–6), Standard (120/360, Versions 5–8), Primary CTA (160/480,
Versions 5–12), Hero / Campaign (200/600, Versions 8–12), Business card
(148/444, Versions 5–12), Flyer / Brochure (177/531, Versions 5–12), and
Poster / Package (236/708, Versions 5–12). The three print base dimensions are
25 mm, 30 mm, and 40 mm converted at 150 dpi and rounded to the nearest pixel.
The artifact policy does not guarantee physical print results; owners must test
their actual device, material, and surface.

The in-product practical guide records the exact maximum for typical ASCII
links that select whole-payload Byte mode. Total length includes scheme, host,
path, query, and fragment. Every retained fixed profile has an explicit limit;
Hero / Campaign has no bundled-logo capacity because its range begins at Version
8, while the centered logo is approved only at Version 6. The guide explains
that non-ASCII characters may occupy multiple UTF-8 bytes plus ECI overhead,
QR-alphanumeric-only input can sometimes fit more, and the exact preview result
remains authoritative.

The bundled logo is enabled by default and is the only release-1 choice that changes ECC: logo mode uses ECC H and an approved Version 6 minimum before version fitting, then recalculates the selected version and all capacity diagnostics. The selected version is the greater of the payload's first fit and the requested minimum, and an inverted minimum/maximum range is a typed error. Disabling the logo restores ECC M, the Version 1 minimum, and ordinary first fitting. The public `qr-core` encoder continues to accept all four ECC levels so conformance tests and future explicitly designed workflows are not constrained by the release-1 UI policy.

### 2.5 PNG renderer

Use a direct RGBA buffer renderer in `qr-render`, then serialize it with the Rust `png` crate. Do not use Canvas, browser SVG screenshots, or a general scene renderer for production PNG export.

- Standard output uses deterministic 8×8 coverage for centered 0.90-module
  dots in the selected approved foreground outside the square finder regions
  on opaque white. Logo knockout cells are exact opaque-white rectangles. The
  complete image is never resized.
- The bundled PNG logo uses deterministic 4×4 final-pixel coverage inside its presentation box so diagonal artwork edges remain smooth; finder and knockout edges remain pixel-sharp.
- Fixed profiles retain their compiled dimensions and 3× PNG relationship.
- Direct RGBA buffers have a target-independent defensive ceiling of 64 MiB; requests above it fail with a typed error before allocation.
- PNG encoder settings, filter, compression, color type, bit depth, and metadata policy are explicit and covered by a byte-for-byte determinism test.
- SVG is generated directly from the render model with stable path ordering
  and numeric formatting. Rounded ONE modules use true circular arc paths
  outside square finders. The document always contains an explicit opaque-white
  background rectangle.

This keeps the pixel geometry testable on native Rust and WASM and avoids browser-dependent rasterization.

### 2.6 Branding safety defaults

These release-1 defaults use the approved ONE treatment. The decode-backed
geometry below was accepted on 2026-08-09 for implementation by Tickets 24–29
and extended on 2026-08-10 by Ticket 30. The production renderer always uses
Rounded ONE modules with the unchanged fixed-profile Version 6 placement and
the decode-backed adaptive placements.

- Two QR foreground themes are approved on opaque white: ONE magenta `#BD0F72`
  and black `#000000`. ONE magenta remains the default. Black is a first-class
  selectable preset, not a hidden path or arbitrary color picker.
- Rounded ONE uses centered 0.90-module dots for non-finder modules while
  keeping standard square finders. Separator modules remain blank. Encoded
  values and coordinates are unchanged.
- The background is always opaque white; transparency is absent from the public
  product and internal selectable appearance model.
- Module strokes and decorative borders: excluded from the product. Surplus fixed-canvas padding remains background-only.
- Finder styling: standard square only.

**Launch decisions accepted by the project owner on 2026-08-07 and revised by
Tickets 32 and 40:** release 1 uses approved ONE magenta and black foreground
themes, an opaque-white background, rounded ONE modules with standard square
finders, the 4.5:1 contrast threshold, and the bundled ONE lettermark described
below. The logo is enabled by default and can be disabled; transparent output
is excluded.

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
  centered opaque-white knockout remains `15 × 7` modules and removes the
  cell range `(left 13, top 17, width 15, height 7)`, clears the nearest
  protected module by six modules, and obscures 105 data modules and
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
- Adaptive placement preserves the reviewed Version 6 center and the fixed
  six-module upward source offset for Versions 7–11. The source remains exactly
  13 modules wide; retaining the 15×7 knockout does not trigger a nearer
  placement or a smaller logo.
  The retained executable evidence in
  [`../crates/qr-render/tests/logo_geometry.rs`](../crates/qr-render/tests/logo_geometry.rs)
  and [`../crates/qr-render/tests/logo_decode.rs`](../crates/qr-render/tests/logo_decode.rs)
  rejects centered Version 10 placement before decoding when it intersects a
  protected alignment module, and verifies the selected function-safe
  placement through independent native-PNG/rasterized-SVG decoding. The
  generated selected-geometry table is
  [`generated/logo-placement-policy.md`](generated/logo-placement-policy.md).
- Versions 1–5 remain below the branded minimum. Versions 12–40 return a typed
  unsafe-logo-geometry rejection until separately approved decode evidence is
  committed; users can disable the logo without changing the payload. Logo
  output stays classified as a caution on every valid profile/version row.
- The knockout must not intersect any function module: finder, separator, timing, alignment, format, version, or fixed-dark module. A conflict is `Invalid`, not merely a warning.
- Overlapped data and remainder modules are counted and reported. Logo mode remains a caution even when valid.
- The renderer compile-time embeds the sanitized project-owned ONE lettermark at
  `assets/RGB-one-lettermark-magenta.svg`. Renderers preserve the sanitized
  geometry and recolor that body only to the selected approved foreground, so
  magenta QR output uses a magenta logo and black QR output uses a black logo.
  No upload, arbitrary SVG, white-logo variant, or runtime logo request is
  accepted in release 1.
- Replacing or editing the lettermark requires recorded license/provenance, sanitization, and the complete structural, deterministic, geometry, and independent-decode logo suite.
- Logo mode requires an opaque white background and knockout.
- The bundled logo is enabled by default. Users may turn it off to select ECC M,
  the Version 1 minimum, and no occlusion.
- Exact centering remains mandatory for the four fixed profiles. Adaptive alone
  may use the reviewed deterministic nearby search. The selected-version
  dimensions and generated evidence are recorded in
  [`generated/logo-placement-policy.md`](generated/logo-placement-policy.md).
- If geometry is unsafe for the selected version, logo mode is disabled with a
  reason. The encoder applies the approved Version 6 branded minimum but never
  selects a still-larger version merely to search for logo space.

ECC percentages are not used as an occlusion budget. Decode testing is mandatory for every enabled logo/profile/version fixture.

Release evidence exhausts the selectable surface with 340 generated scenarios:
168 required-payload rows and 172 exact-version rows across profile, logo state,
and foreground theme. Native PNG and independently rasterized SVG artifacts
share one scenario identity and record deterministic hashes, safety, decode
outcome, foreground theme, and fixed logo geometry. The resulting policy has
244 accepted rows and 96 typed expected rejections.

The exported symbol always retains exactly four quiet-zone modules per side.
For Version $v$, its complete logical width is $4v + 25$ modules. Every
approved fixed range retains a centered integer module pitch and at least six
PNG pixels per module at its maximum version after quiet-zone and logo rules.
Decorative export borders, frames, labels, and strokes remain excluded; fixed
PNG canvas surplus is background-only.

## 3. Specification corrections required

These are implementation interpretations until merged back into the product specification.

1. **No border layer and explicit SVG sizing:** Decorative borders, frames, labels, and module strokes are excluded. PNG surplus padding remains blank/background-only. Fixed-profile SVG `width` and `height` equal the compiled base dimensions. Every SVG `viewBox` is the tight logical extent of the QR matrix plus exactly four quiet-zone modules on every side and contains no fixed-canvas surplus padding. Consumers scale the vector through `width` and `height`. No border types, render options, controls, errors, or tests should be scaffolded for possible future use.
2. **Capacity diagnostics:** Display exact `used data bits / available data bits`, data codewords, and whether the symbol uses one mode or a mixed segment plan. “Remaining capacity” is an estimate because an edit can change the optimal segmentation.
3. **Function-module protection:** Branding and logo knockout never modify function modules. The spec's general “protect function patterns” goal takes precedence over language that only makes finder overlap explicitly invalid.
4. **Mask evaluation:** Apply each mask only to data/remainder modules, write the corresponding format bits, then score the complete final matrix. Choose the lowest score and lower mask ID on a tie.
5. **Remainder bits:** Capacity tables and placement must explicitly include the standard remainder-bit count per version. Every non-function matrix cell must be assigned once, including remainder bits.
6. **Input safety:** Plain text is allowed; URL syntax is not required. The UI may identify likely URLs, but it must not rewrite them. Empty input and over-limit input are invalid. Control characters receive a caution unless product policy later forbids them.
7. **External network calls:** Production HTML must not request Google Fonts or other remote UI assets. Bundle approved assets or use the system font stack.
8. **Print guidance:** The 148 px, 177 px, and 236 px values are 150 dpi artifact conversions, not physical-size guarantees. Export remains SVG-first and the UI tells owners to test the final material, device, and surface.
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
    pub mode: EncodingMode,
    pub segments: Vec<EncodedSegment>,
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

### 4.3 Preview worker boundary

The browser UI owns one lifecycle-scoped dedicated Web Worker for preview
generation. The main thread preserves form state, the 250 ms debounce, and the
authoritative revision check; after the debounce it sends the exact payload,
profile, logo choice, and revision to the worker. The worker's separate WASM
instance runs the existing `evaluate_preview` path, including encoding,
render-model construction, and deterministic SVG/PNG generation. QR behavior
is not duplicated in JavaScript.

Messages use an explicit version-local Rust protocol. Requests and response
metadata are JSON; PNG bytes use a transferable `ArrayBuffer` so they are not
expanded into JSON or retained in the worker after delivery. The main thread
reconstructs the existing typed preview and accepts it only when its revision
is still current. Malformed messages, worker startup/runtime errors, and failed
dispatches leave the workflow in the existing payload-free internal-error
state rather than pending indefinitely.

Trunk builds the worker as the local `qr-preview-worker` binary and emits its
loader and WASM beside the application. Worker creation happens once per app
owner, uses no remote resource, and termination on owner cleanup releases its
callbacks and WASM instance. All Worker and messaging APIs remain in `qr-web`;
the `qr-web -> qr-render -> qr-core` dependency direction is unchanged.

## 5. Recommended dependencies

Keep versions exact in the workspace manifest and update dependencies deliberately.

### Production

- `leptos = =0.8.20` with `csr` for the web crate.
- `wasm-bindgen`, `web-sys`, and `js-sys` only in `qr-web` for Blob/URL/download integration.
- `serde`/`serde_json` in `qr-web` for the explicit local worker-message
  protocol; compiled Rust constants remain preferred for the five profiles.
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
- Apply rounded ONE modules and standard square finders on opaque white.
- Integrate sanitized bundled logo, knockout, function-overlap validation, and overlap diagnostics.
- Add exhaustive approved-combination decode tests for logo-off and logo-on output.

**Exit:** every selectable combination passes its required decode suite; unsafe combinations cannot be selected or exported.

### M5 — Release hardening (1–2 weeks, risk: medium)

- Run sustained fuzzing and dependency/license review.
- Execute adverse raster transformations with documented thresholds.
- Complete the release runbook, user guidance, and local production-build privacy inspection.

**Exit:** all automated acceptance criteria have linked evidence; the local production build makes no payload/logo request and logs no payload. Manual product checks remain outside the evidence system.

**Total expected engineering effort:** roughly 8–13 developer-weeks, with core conformance and logo decode validation carrying most uncertainty. Removing borders saves a UI/configuration branch, a render layer, SVG security cases, and one dimension from the branding test matrix; it does not materially reduce encoder-core risk.

## 8. Local verification

The repository documents local commands for formatting, warnings-as-errors linting, native tests, WASM checking, and an optimized Trunk build. The extended suites in [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md) are run locally when their related implementation exists, with longer fuzz, mutation, browser, and adverse-image checks performed during release hardening.

Repository-owned hosted correctness automation selects a conservative focused
gate for each push to `main`: isolated core, render, web, and Python-support
changes use their own covering gates, while mixed, dependency,
shared-configuration, and unknown changes use the complete routine gate.
Extended decoder campaigns run
in a separate path-filtered workflow only when core, render, artifact, oracle,
or release-evidence inputs change. Pages publishing is limited to site and
build-input changes and only repackages the covering job's verified artifact
with the Pages base path before upload/deployment; both workflows retain manual
dispatch. Caches are keyed by the
controlling lockfiles, toolchain/tool versions, runner OS, and architecture;
restored tools and decoder checkouts are verified before use.

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
18. Approved fixed brand/white contrast and opaque previews.
19. Rounded ONE modules and standard square finders.
20. Bundled magenta ONE lettermark, knockout, overlap checks, and decode matrix.
21. Hardening, automated release evidence, and docs.

Tickets 3–11 should generally land sequentially because later code relies on earlier invariants. UI shell work may run in parallel after ticket 2, but export controls must not be presented as functional until the core and safe renderer pass their gates.

## 10. Owner coordination still needed

The implementation policy, launch presets, ECC behavior, contrast threshold,
opaque-white Rounded ONE styling with square finders, and bundled ONE lettermark are accepted. Access to
a licensed complete ISO/IEC 18004:2024 copy remains useful for a later audit but
is not an implementation gate under the public-source corroboration policy in
section 2.1.

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
