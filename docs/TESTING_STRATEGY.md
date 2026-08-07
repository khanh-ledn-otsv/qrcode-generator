# QR Code Generator — Testing Strategy

**Companion to:** `DEVELOPMENT_PLAN.md`  
**Status:** Development-ready  
**Primary objective:** Demonstrate standards correctness, deterministic rendering, scan reliability, browser behavior, and resilience to hostile input without trusting the implementation under test as its own oracle.

## 1. Quality model

A QR output is accepted only when all applicable layers agree:

1. **Conformance:** encoded bits, error-correction blocks, function patterns, mask, format/version information, and final matrix match normative rules and independently produced fixtures.
2. **Structural invariants:** every matrix cell has the correct ownership, quiet zones and fixed canvases are exact, and checked arithmetic prevents invalid state.
3. **Independent decoding:** production SVG and PNG output decode to the original bytes/text using a separately implemented decoder.
4. **Product behavior:** profiles, warnings, disabled states, downloads, accessibility, and privacy behave correctly in real browsers.
5. **Robustness:** property tests, mutation tests, fuzzing, and adverse-image tests expose mistakes not represented by examples.
6. **Physical validation:** representative phone, scanner, screen, and print environments pass a documented release checklist.

No single decoder result proves QR conformance. Conversely, an exact unstyled matrix does not prove that branded output remains usable.

## 2. Selected testing libraries and tools

All test tools must be pinned in manifests, lockfiles, or documented local tool setup. Updates are deliberate and run the full suite.

### 2.1 Rust development dependencies

| Library | Scope | Why selected |
|---|---|---|
| `proptest` | `qr-core`, `qr-render` | Generates and shrinks payloads, versions, ECC levels, matrices, styles, and dimensions. Use instead of hand-written random loops so failures reduce to a reproducible minimal case. |
| `wasm-bindgen-test` | `qr-web` browser boundary | Runs Rust tests on `wasm32-unknown-unknown` in a real headless browser. Covers Blob creation, object URLs, browser download adapters, and error conversion across WASM. |
| `resvg` | test support only | Rasterizes production SVG independently from the production renderer so the actual SVG artifact can be decoded and pixel-inspected. |
| `roxmltree` | test support only | Parses SVG for structural/security assertions: viewBox, dimensions, prohibited elements/attributes, external references, and deterministic path ordering. |
| `image` | test support only | Applies deterministic blur, rotation, perspective, contrast, and compression simulations to rendered fixtures. It must not be a production renderer. |
| `insta` | selected snapshots | Reviews normalized SVG structure, diagnostics, and error messages. Do not use snapshots as the primary QR matrix oracle. |
| `serde` + `serde_json` | fixture tooling | Reads a strict fixture manifest with source provenance and expected metadata. Production profile constants do not require JSON. |
| `sha2` | fixture tooling | Records artifact hashes and detects accidental golden changes. |
| `tempfile` | integration tests | Isolates generated artifacts and decoder subprocess inputs. |
| `criterion` | native benchmarks | Tracks encoder and renderer performance distributions outside correctness tests. Benchmarks do not use wall-clock assertions in ordinary unit tests. |

Pin `proptest` to the reviewed 1.11.x line initially. Pin all other crate versions when M0 creates the workspace; do not use unconstrained `*` or floating Git revisions.

### 2.2 Cargo and Rust toolchain utilities

| Tool | Purpose | Policy |
|---|---|---|
| `cargo-llvm-cov` | Line and region coverage | Report per crate and per file. Stable branch coverage is not assumed. Generated tables and test support code are excluded transparently. |
| `cargo-mutants` | Mutation testing | Required for `qr-core` and critical geometry code. Every surviving mutation is triaged; exclusions require a reason in config. |
| `cargo-fuzz` | Coverage-guided libFuzzer targets | Run manually with documented budgets. Crashes, panics, hangs, excessive allocation, and invariant failures are defects. Minimized regression inputs are committed. |
| Miri | Undefined-behavior and strict interpreter checks | Run manually for supported native core and render tests. Production code should contain no `unsafe`; dependencies are outside the project gate. |
| `cargo-audit` | RustSec advisories | Run manually before release, with any temporary advisory exception documented. |
| `cargo-bloat` | WASM/native size attribution | Trend report for release builds; helps validate the provisional bundle target. |

Start with the currently reviewed `cargo-llvm-cov` 0.8.x line, then pin exact versions when each local tool is introduced.

### 2.3 Browser and accessibility tools

| Tool | Scope |
|---|---|
| Playwright Test | End-to-end testing in Chromium, Firefox, and WebKit; viewport/device emulation; download verification; keyboard interaction; screenshots only where visual review is useful. |
| `@axe-core/playwright` | Automated accessibility rules integrated into Playwright. It supplements, but does not replace, keyboard and screen-reader-oriented checks. |

Use TypeScript only in `e2e/`. Keep business logic and expected QR calculations in Rust fixtures rather than duplicating the encoder in test JavaScript.

### 2.4 Source quality tools

| Tool | Scope | Policy |
|---|---|---|
| Node.js v24 + pnpm | Browser test toolchain | Declare Node.js v24 in `.nvmrc`, pin pnpm through `packageManager`, and commit `pnpm-lock.yaml`. Do not maintain an npm lockfile in parallel. |
| Oxlint + Oxfmt | TypeScript and browser configuration | Oxlint errors on correctness/suspicious findings and warnings; Oxfmt is the Oxc formatter and its check mode is part of verification. |
| Ruff | `tests/support` Python | Run both `ruff check` and `ruff format --check`; pin the executable in the oracle `uv.lock`. |
| ty | `tests/support` Python | Type check the complete support-script tree in the same locked `uv` environment as the Python oracle dependencies. |

### 2.5 Independent QR tools

- **ZXing-C++:** primary pinned decode oracle for PNG and rasterized SVG. Compare decoded Unicode text, raw bytes, ECI state, symbol version, and ECC metadata when exposed.
- **quirc:** secondary, implementation-diverse decoder for representative ASCII raster tests. Do not use it as the sole UTF-8/ECI oracle.
- **Nayuki QR Code Generator 1.8.0:** development-only generator for explicit-version/mask golden fixtures after owner approval.
- **`python-qrcode` 8.2:** separately maintained second generator for the same fixed byte-mode fixtures. Commit a golden only after exact agreement. Segno 1.6.6 was evaluated first and rejected after a concrete byte-aligned padding disagreement; the disputed matrix was not accepted.

None of these generators or decoders is linked into the production application.

### 2.6 Public-source corroboration when complete normative text is unavailable

Follow [`research/qr-public-source-provenance.md`](research/qr-public-source-provenance.md) for GF(256), Reed–Solomon, interleaving, function/data placement, BCH, mask predicates, and penalty rules implemented before a licensed ISO/IEC 18004:2024 audit is available.

- Pin every upstream source by immutable release tag and exact file/symbol in fixture or test metadata.
- Require agreement from Nayuki QR Code Generator and `python-qrcode` for behavior both encoders expose; do not copy either implementation into production.
- Add a locally written slow reference or structural invariant for arithmetic, ownership, length, and coordinate rules where practical.
- Compare explicit version/ECC/mask matrices before testing automatic mask selection.
- Decode completed artifacts with pinned ZXing-C++; a successful decode supplements but never replaces exact matrix and invariant checks.
- Mark the evidence `public-corroborated, non-normative` until a complete licensed 2024 text is audited.
- Treat any source, fixture, invariant, or decoder disagreement as a blocked test fixture requiring investigation and recorded resolution. A recorded owner resolution may select narrowly defined semantics only when it preserves every disagreeing result, is backed by an independently written reference, and retains exact completed-matrix and independent-decode gates; it is never a majority vote.

## 3. Test repository structure

```text
crates/qr-core/
├── src/...                       # focused unit tests beside implementation
└── tests/
    ├── conformance.rs
    ├── golden_matrices.rs
    ├── capacity_boundaries.rs
    └── properties.rs
crates/qr-render/
└── tests/
    ├── profile_geometry.rs
    ├── svg_structure.rs
    ├── png_structure.rs
    ├── decode_roundtrip.rs
    ├── styling_matrix.rs
    └── determinism.rs
crates/qr-web/
└── tests/
    ├── state.rs
    └── wasm_browser.rs
tests/
├── fixtures/
│   ├── manifest.json
│   ├── matrices/
│   ├── payloads/
│   └── approved_assets/
├── support/                      # oracle runners and artifact inspection
└── adverse/                      # deterministic transform definitions
e2e/
├── playwright.config.ts
├── accessibility.spec.ts
├── downloads.spec.ts
├── privacy.spec.ts
├── profiles.spec.ts
└── validation.spec.ts
fuzz/
├── fuzz_targets/
└── corpus/
benches/
└── encode_render.rs
```

Test support code must be reusable but must not contain a second hand-written copy of production QR rules. Expected results come from committed provenance fixtures, independent tools, legally usable standards material, or simple invariants.

## 4. Fixture provenance

Every golden fixture must have a manifest record similar to:

```json
{
  "id": "v07-q-mask3-utf8-eci26-001",
  "payload_file": "payloads/utf8-001.bin",
  "payload_sha256": "...",
  "encoding": "utf-8",
  "eci_assignment": 26,
  "mode": "byte",
  "version": 7,
  "ecc": "Q",
  "mask": 3,
  "expected_matrix_file": "matrices/v07-q-mask3-utf8-eci26-001.txt",
  "expected_matrix_sha256": "...",
  "sources": [
    { "tool": "oracle-a", "version": "pinned-version", "command": "recorded command" },
    { "tool": "oracle-b", "version": "pinned-version", "command": "recorded command" }
  ],
  "verified_by": "review reference",
  "notes": "No sensitive production payload"
}
```

Rules:

- Fixtures contain generated, non-sensitive payloads only.
- Binary payload files prevent newline/normalization mistakes.
- Explicit mask and version are used for matrix comparison; automatic mask-selection tests are separate.
- Fixture regeneration is an explicit command and never occurs implicitly during a test.
- A golden change requires a human-readable matrix diff, metadata diff, oracle versions, and reviewer approval.
- Standards-derived fixture material is committed only when redistribution is permitted. Dual-oracle table fixtures record both pinned implementations and are labelled non-normative.
- Algorithm fixtures produced under the public-source policy record source tags/files, exact generation commands, local-reference or invariant coverage, and the label `public-corroborated, non-normative`.

## 5. `qr-core` test suite

### 5.1 Bit buffer and mode encoding

Unit-test:

- zero-length and cross-byte writes;
- exact bit order for 1–32 bit values;
- overflow/invalid-width errors;
- terminator truncation at capacity;
- zero padding to byte boundary;
- alternating `0xEC`/`0x11` pad codewords;
- Numeric groups of 1, 2, and 3 digits;
- Alphanumeric groups of 1 and 2 characters;
- full QR alphanumeric alphabet and rejection of lowercase/non-members;
- ASCII Byte and UTF-8 Byte byte counts;
- ECI assignment 26 indicator and payload;
- character-count widths at version transitions 9→10 and 26→27;
- exactly-fit and one-bit/character-over boundaries.

Property examples:

- encoded bit length equals the independently calculated formula for the selected mode;
- appending a same-mode character never reduces required bits;
- emitted data codewords always exactly fill the selected version's data capacity;
- a successful encode never exceeds the configured maximum version.

### 5.2 Capacity and QR tables

For all 40 versions × 4 ECC levels:

- data codewords plus ECC codewords equal total codewords;
- group block counts and lengths expand to the declared totals;
- all blocks for a row differ in data length by at most one where the standard defines two groups;
- remainder-bit count is in the allowed range and agrees with the matrix-cell accounting;
- alignment coordinates are ordered, unique, in bounds, and omit finder conflicts;
- version size is exactly `21 + 4 × (version - 1)`;
- character-count widths are valid for the version band;
- Version 40 and all profile ceilings have explicit regression cases.

Table tests are mandatory even when the values were copied correctly: a single table typo can produce plausible but undecodable symbols only for rare version/ECC combinations.

### 5.3 GF(256) and Reed–Solomon

Test:

- exponent/log table cycles under primitive polynomial `0x11D`;
- zero identities and multiplication/division inverses;
- multiplication against a slow, test-only polynomial reference implementation;
- generator polynomial vectors for every ECC degree used by the standard tables;
- remainder vectors from normative or dual-oracle fixtures;
- leading/trailing zero data;
- maximum block length;
- no mutation of input buffers and correct output length.

Proptest compares the optimized GF multiplication and RS remainder against deliberately simple test references. The references should prioritize clarity and algebraic independence over speed.

### 5.4 Block construction and interleaving

Cover every distinct block-layout shape, including both one-group and two-group rows:

- data split consumes each data codeword once;
- each block receives the correct ECC length;
- short and long blocks interleave at the correct positions;
- ECC interleaving follows data interleaving;
- de-interleaving in test code reconstructs the original blocks;
- the final stream length equals total codewords plus remainder bits.

A generated meta-test iterates all 160 version/ECC rows so no table row remains unexecuted.

### 5.5 Matrix construction

For all versions:

- finder, separator, timing, alignment, format, version, and dark-module coordinates match fixtures/invariants;
- versions below 7 contain no version information region;
- format/version reservation does not collide incorrectly with other patterns;
- every writable cell is assigned exactly once;
- no function cell is overwritten by data or masking;
- zig-zag traversal skips column 6 correctly;
- final dark/light value and `ModuleKind` agree;
- remainder modules are classified separately from payload data;
- rotations/mirroring are not accidentally introduced by row/column indexing.

Use small human-reviewable coordinate fixtures for Versions 1, 2, 7, and 40, plus generated invariant tests for all versions.

### 5.6 Masks, BCH, and penalty scoring

For Rule 3, the owner-approved interpretation is literal complete-matrix
matching: count `00001011101` and `10111010000` windows wholly inside rows and
columns, without virtual quiet-zone padding. Tests preserve Nayuki 1.8.0's
differing run-history totals as an explicit oracle exception and require the
production result to match both python-qrcode 8.2 and an independently written
slow reference.

Test each mask predicate across a coordinate grid with explicit expected truth tables. Test format BCH for all 4 ECC × 8 masks and version BCH for Versions 7–40.

Each penalty rule has isolated synthetic matrices:

- runs in rows and columns;
- 2×2 blocks;
- finder-like `1:1:3:1:1` patterns with required light context in both orientations;
- dark-module balance and exact rounding behavior.

Then test combined matrices and tie behavior. For each automatic-mask fixture:

- score all eight completed candidate matrices;
- confirm the chosen mask has the minimum score;
- confirm the lower mask ID wins equal scores;
- confirm scoring includes function and candidate format modules while masking changes only data/remainder modules.

## 6. `qr-render` test suite

### 6.1 Profile geometry

Exhaustively iterate every version permitted by each profile:

- SVG `width` and `height` equal the profile base dimensions;
- SVG `viewBox` is exactly `0 0 N N`, where `N` is the matrix width plus eight modules for the four-module quiet zone on each side; it contains no fixed-canvas surplus padding;
- PNG dimensions equal exactly 3× base dimensions;
- complete symbol includes four quiet modules per side;
- module scale is the largest positive even scale that fits;
- rendered symbol dimensions and outer padding use checked integer arithmetic;
- direct RGBA allocation lengths above the target-independent 64 MiB ceiling are rejected identically on native and WASM;
- outer padding is symmetric and integral;
- maximum profile versions use at least 6 px/module;
- surplus padding contains only the selected background treatment and no artwork;
- transparent surplus padding has zero alpha and opaque surplus padding exactly matches the configured background;
- unsafe logo geometry is rejected before rendering.

Include explicit expected cases for all four profile ceilings and for transitions where module scale decreases.

### 6.2 SVG artifact tests

Parse every generated SVG and assert:

- exact profile-base `width` and `height` and the tight matrix-plus-quiet-zone `viewBox` defined above;
- valid XML with no scripts, events, remote URLs, external stylesheets, foreign objects, or payload metadata;
- background rectangle is present/absent according to opacity;
- quiet zone remains unpainted by QR modules and branding;
- no frame, label, stroke, or path exists outside the QR symbol geometry;
- paths stay inside their cells and within checked bounds;
- function modules retain their approved conservative geometry;
- the sanitized magenta ONE lettermark is embedded from `assets/RGB-one-lettermark-magenta.svg` with unchanged geometry, the reviewed `180 180 640 240` presentation box, and no external reference;
- logo knockout geometry is opaque white, outside the four-module quiet zone, function-safe, deterministic, and independently decoded for every enabled H-level profile/version row;
- stable element/path ordering and normalized number formatting;
- identical request produces identical UTF-8 bytes.

Use `insta` only for normalized semantic snapshots. Exact fixture hashes remain the determinism gate.

Rasterize SVG with pinned `resvg` at the intended dimensions and feed the pixels to ZXing-C++. This tests the artifact, not an internal render model shortcut.

### 6.3 PNG artifact tests

Decode the emitted PNG as a file and inspect:

- valid PNG signature and chunk structure;
- exact width/height, RGBA, 8-bit depth;
- no timestamp or payload-bearing text chunks;
- configured, deterministic metadata/chunk policy;
- quiet-zone and outer-padding pixels;
- no non-background pixel exists in surplus outer padding;
- exact square-module rectangles with no intermediate colors in safe mode;
- approved edge coverage only for rounded/dot styles;
- byte-for-byte equality for repeated requests on native and WASM where encoder output is specified to be cross-target identical.

Decode the resulting pixels through ZXing-C++; do not declare success merely because the same `png` crate can read its own output.

### 6.4 Branding combination matrix

Testing every control independently is insufficient because failures interact. Build a generated list from compiled approved presets and require that every selectable tuple appears in the test report.

For each approved tuple of foreground, background/transparency, module style, finder style, logo state, and profile:

- render at least a short URL, a dense URL near the profile ceiling, Numeric, Alphanumeric, ASCII Byte, and UTF-8+ECI payload;
- test safe baseline versions across all allowed versions;
- emphasize Versions 1, 2, 5, 6, 7, 8, 12, and 13 for styling/function-pattern transitions;
- for logo mode, construct payloads selecting every H-level version allowed by each profile and assert either valid decode or intentional geometry rejection;
- record warning/invalid classification alongside decode results.

The generated coverage test must fail if a new approved enum variant is not included in the matrix.

### 6.5 Adverse-image tests

Keep transforms deterministic with named parameters and seeds. Start with separate transforms before combining them:

- Gaussian blur at increasing radii;
- down/up display simulation without changing exported artifact requirements;
- JPEG screenshot compression at selected qualities;
- rotations at small angles;
- four-point perspective distortion;
- reduced contrast and brightness shifts;
- light and dark/patterned backgrounds for transparent output;
- simulated print dot gain, ink loss, and grayscale conversion.

Define a baseline pass envelope before launch from real approved outputs. Do not invent universal thresholds. Safe presets should meet a stronger envelope than logo/rounded caution presets. Store transform parameters and decoder outcomes as machine-readable release evidence.

## 7. Web and WASM tests

### 7.1 Native state tests

Keep form state and validation derivation in plain Rust where possible. Unit-test:

- every input-to-derived-state transition;
- profile changes recalculate version, limits, sizes, logo availability, and warnings;
- logo toggling changes ECC to H before version selection;
- invalid states always disable both exports;
- stale debounced work cannot overwrite newer input state;
- warning ordering and severity are deterministic;
- payload text never appears in filenames, metadata, logs, or accessible preview labels.

### 7.2 `wasm-bindgen-test`

Run browser tests for:

- Blob creation and MIME type;
- object URL lifecycle and revocation;
- exact downloaded byte content returned by adapters;
- DOM/browser error conversion without panic;
- debounce timers and disposal;
- repeated generation without leaked object URLs or unbounded retained buffers.

Run these locally in headless Chromium, and add Firefox where the harness supports it before release.

### 7.3 Playwright end-to-end suite

Test through the user-visible UI:

- entering Numeric, Alphanumeric, URL, ASCII Byte, and UTF-8 payloads;
- character count versus UTF-8 byte count;
- all profile selections and displayed diagnostics;
- version/ECC changes, including logo-triggered H;
- invalid capacity and unsafe-style states;
- transparent/background warnings;
- keyboard-only operation and visible focus;
- rapid typing/debounce with latest-value wins;
- SVG and PNG downloads, filenames, dimensions, hashes, and independent decode;
- reload and back/forward behavior if state persistence is added;
- responsive layouts at supported desktop and mobile viewports.

Run Chromium during feature verification. Run Chromium, Firefox, and WebKit during release hardening. Playwright projects use pinned browser binaries matching the pinned Playwright version.

### 7.4 Accessibility

Automated axe checks run on the default, caution, invalid, logo, and transparent states at desktop and mobile widths. Explicit tests also verify:

- unique programmatic labels;
- semantic fieldset/group relationships;
- warning text is not color-only;
- validation announcements use an appropriate live region without announcing each keystroke;
- focus is not stolen during preview refresh;
- export-disabled reasons are available to assistive technology;
- profile cards and style controls work with keyboard and expected roles;
- preview has a useful label that excludes sensitive payload text;
- contrast of the application UI, independent of QR output contrast.

Manual release testing includes VoiceOver on Safari and one Windows screen-reader/browser combination selected by the owner.

### 7.5 Privacy and security behavior

Playwright intercepts all network requests after initial navigation. Fail if generation, preview, style changes, logo use, or download causes any request outside the static application origin. Also assert:

- no Google Fonts or other third-party runtime asset requests;
- no payload in URL, history, document title, filename, SVG metadata, console output, or storage;
- no remote reference in exported SVG;
- browser security policy blocks script/style sources outside the approved local application policy;
- arbitrary text containing XML/HTML metacharacters cannot alter SVG structure or DOM;
- large input is rejected before expensive allocation.

## 8. Property testing

Use custom `proptest` strategies that favor boundaries rather than uniformly random data:

- version bands: 1, 9, 10, 26, 27, 40 and neighbors;
- profile ceilings: 5, 8, 12, 13 and one-over cases;
- payload lengths around each capacity boundary;
- mode-changing characters such as lowercase, space, `:`, non-ASCII, and multi-byte UTF-8;
- all ECC and mask values;
- transparent/opaque backgrounds and allowed styles;
- malformed internal configuration in test-only constructors.

Core properties:

- encode success implies all matrix cells assigned and all dimensions valid;
- encode failure is typed and never panics;
- encode → safe render → independent decode returns identical bytes/text;
- version selected is the first fitting version under the request limit;
- increasing the maximum version cannot turn a successful request into capacity failure;
- identical requests produce identical matrix, SVG, PNG, diagnostics, and hashes;
- branding never changes encoded matrix values or module classification;
- render output stays within allocation and dimension bounds;
- profile output always obeys fixed dimensions and integer geometry.

Routine local runs use a stable committed RNG seed plus persisted failure cases. Extended runs add several recorded rotating seeds. A failing seed and minimized input become a permanent regression test.

## 9. Fuzzing

Create separate targets so failures are attributable and corpora stay useful:

1. `encode_utf8`: arbitrary byte input converted through lossy and valid UTF-8 paths, ECC, and max version.
2. `encode_valid_text`: structured Unicode strings emphasizing mode and version boundaries.
3. `bit_buffer`: operation sequences and requested widths.
4. `reed_solomon`: bounded data blocks and supported ECC lengths compared with the slow reference.
5. `matrix_build`: valid encoded streams plus test-only malformed builder operations.
6. `render_svg`: encoded matrices and bounded validated/malformed render options.
7. `render_png`: same, with dimension/allocation assertions and PNG parse-back.
8. `fixture_parser`: malformed fixture manifests used only by developer tooling.

Fuzz assertions include no panic, abort, hang, unchecked overflow, out-of-bounds access, excessive allocation, invalid successful output, or invariant violation.

Budgets:

- Routine: replay the committed corpus; no open-ended fuzzing.
- Extended: at least 10 minutes per target, sharded where useful.
- Deep: at least 60 minutes per critical target (`encode_utf8`, `reed_solomon`, `matrix_build`, `render_png`).
- Release: a documented extended run with zero unresolved findings.

Sanitize and minimize crash artifacts before committing them. Fuzz payloads must remain generated and non-sensitive.

## 10. Mutation testing and coverage gates

### 10.1 Coverage

Initial gates after M1 stabilizes:

| Scope | Line coverage | Region coverage |
|---|---:|---:|
| `qr-core` total | ≥95% | ≥90% |
| GF/RS, matrix, mask, BCH, penalty files | ≥98% | ≥95% |
| `qr-render` total | ≥90% | ≥85% |
| Profile/geometry code | ≥98% | ≥95% |
| Testable plain-Rust `qr-web` state | ≥85% | ≥80% |

Generated constant tables, exhaustive-match boilerplate, and browser-only glue may be excluded only through reviewed configuration. Coverage regressions must be investigated before accepting a change. Coverage is a missing-test signal, not proof of correctness.

### 10.2 Mutation score

Run `cargo-mutants` on changed critical files during focused verification and on all of `qr-core` plus profile geometry during release hardening.

- Critical arithmetic/placement/mask/BCH code target: at least 90% caught mutations.
- Whole `qr-core` target: at least 85% caught mutations.
- Profile geometry target: at least 90% caught mutations.
- Any survivor involving a comparison boundary, table index, mask predicate, bit shift, coordinate, or error path must be killed by a new test or explicitly proven equivalent.
- Timeouts and unviable mutants are reported separately and do not inflate the score.

Do not add broad mutation exclusions simply to meet the threshold.

## 11. Performance and resource tests

Use Criterion to benchmark:

- mode selection and fit calculation;
- Version 1, profile ceilings, and Version 40 encoding;
- each mask candidate and total mask selection;
- safe SVG generation;
- safe and rounded PNG generation at all four canvas sizes;
- logo overlap analysis;
- full request-to-artifact path.

Report median and tail distributions and track them over time. Correctness tests use generous hang/allocation guards, not millisecond assertions. Run performance comparisons in a stable local environment and investigate regressions.

Test defensive resource limits explicitly:

- 4 KiB input is accepted/rejected through normal capacity logic without excessive allocation;
- one-byte-over input limit fails before encoding;
- malformed dimensions/configuration fail checked arithmetic;
- repeated preview/download operations do not grow browser heap without bound;
- SVG path/PNG buffer size stays below a documented bound for every profile.

## 12. Local and release test suites

### Routine local verification

- formatting and warnings-as-errors Clippy;
- native unit and integration tests;
- table, golden, exact-boundary, and safe-render tests;
- deterministic property tests and committed fuzz-corpus replay;
- WASM check and optimized Trunk build;
- browser smoke tests when web behavior changes.

### Extended local verification

- full property case count and approved branding tuples;
- exhaustive profile/version geometry and independent decoding;
- Chromium, Firefox, and WebKit browser tests;
- coverage, mutation, Miri, adverse-image, size, and performance checks as applicable.

### Release validation

- clean rebuild with pinned tools and artifact hashes;
- complete automated suite with no retry-hidden failures;
- named real-device, scanner, browser, and printer matrix;
- print samples at 25 mm and 30 mm;
- local production-build privacy and network inspection;
- signed evidence report mapping every acceptance criterion to tests and results.

Repository-owned automation and publishing are intentionally deferred and are not specified here.

## 13. Flake and failure policy

- Correctness tests get no automatic retry in required verification suites.
- A flaky test is quarantined only with an owner, linked defect, expiration date, and equivalent release-risk mitigation.
- Never weaken a matrix/hash assertion because an oracle disagrees; isolate the discrepancy and determine which rule or representation differs.
- Persist Proptest regressions and fuzz crashes in the repository after sanitization.
- Store Playwright traces, screenshots, downloads, decoder logs, JUnit output, coverage, and mutation reports on failure.
- Time-based tests use controlled clocks where practical. Random tests record seeds. Image transforms use fixed parameters.
- Tool crashes and oracle timeouts fail the job; they are not interpreted as product passes.

## 14. Test review checklist for each feature

A feature change is incomplete unless reviewers can answer yes to the applicable questions:

- Does it add exact success, boundary, and typed-failure tests?
- Is the expected result independent of production logic?
- Does it cover all enum/config variants or enforce exhaustive generation?
- Does it preserve native/WASM determinism?
- Does it add or update independent decode coverage?
- Could a one-character, one-bit, one-module, or one-pixel error survive?
- Does Proptest favor its new boundaries?
- Is its input surface represented by an existing fuzz target?
- Would mutation testing detect reversed comparisons and altered constants?
- Are accessibility, privacy, and logging effects tested?
- Are fixture changes provenance-recorded and human-reviewable?
- Are performance/allocation bounds affected?

## 15. Testing implementation order

1. Add `tests/fixtures/manifest.json` schema and oracle provenance rules.
2. Document local test commands and add coverage reporting when the implementation is ready for it.
3. Build dual-oracle explicit-mask fixture generator outside production crates.
4. Implement table invariant tests before populating all tables.
5. Add unit/reference/property tests with each `qr-core` module, not afterward.
6. Establish core coverage and mutation baselines at M1 completion.
7. Build independent SVG/PNG inspection and ZXing decode harness before adding branding.
8. Add profile-exhaustive geometry and safe-style decode tests.
9. Add native web-state tests, WASM browser tests, then Playwright E2E.
10. Generate the branding tuple matrix from approved configuration.
11. Add adverse transforms, documented fuzz budgets, Miri, and performance tracking.
12. Produce the release evidence template before manual validation starts.

## 16. References checked

- [Proptest API documentation](https://docs.rs/proptest/latest/proptest/)
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/)
- [`wasm-bindgen-test` guide](https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/usage.html)
- [cargo-llvm-cov repository](https://github.com/taiki-e/cargo-llvm-cov)
- [cargo-mutants documentation](https://mutants.rs/)
- [Playwright browser documentation](https://playwright.dev/docs/browsers)
- [Playwright accessibility testing](https://playwright.dev/docs/accessibility-testing)
- [`resvg` documentation](https://docs.rs/resvg/latest/resvg/)
- [ZXing-C++ repository](https://github.com/zxing-cpp/zxing-cpp)
