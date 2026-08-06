# 10 — Select masks using BCH and penalty scoring

**What to build:** Finalize each mask candidate with correct format/version information, score the complete matrix, and select one deterministic standards-conformant result.

**Blocked by:** 09 — Place data and remainder modules with explicit masks.

**Status:** resolved

- [x] Format BCH values are correct for every ECC/mask pair and version BCH values are correct for Versions 7 through 40.
- [x] Isolated tests cover row/column runs, 2×2 blocks, finder-like patterns with required context, and dark-module balance rounding.
- [x] Candidate scoring includes function patterns and candidate-specific format modules.
- [x] The minimum-penalty mask is selected, with the lower mask ID winning ties.
- [x] Explicit-mask golden fixtures and combined synthetic matrices catch predicate, coordinate, BCH, and penalty errors.
- [x] BCH, mask-predicate, and penalty fixtures cite the exact pinned public source files/symbols, agree between both encoders where exposed, and are labelled `public-corroborated, non-normative` pending a complete 2024 audit.
- [x] Repeated selection from identical input produces identical mask and matrix output.

## Answer

Added independently implemented BCH polynomial division for every format/ECC
combination and Version 7–40 information value. Checked finalization writes
both redundant format copies and both version-information copies only into
their reserved ownership regions after data placement.

Added all four complete-matrix penalty rules: row and column runs, uniform
2×2 blocks, contextual finder-like patterns, and five-percent dark-balance
steps. Automatic selection builds and finalizes all eight candidates, scores
every module including function and candidate-specific information modules,
chooses the minimum, and preserves the lower mask identifier on ties. The
typed result exposes the selected mask, score, and immutable matrix.

Committed a reproducible `public-corroborated, non-normative` fixture covering
all 32 format values, all 34 version values, 24 completed explicit-mask
candidates, three independently agreed automatic choices, and two combined
synthetic penalty matrices. Both pinned encoders agree on BCH values,
completed matrices, isolated agreement cases, and accepted selected masks;
candidate totals also match an independently written complete-matrix scorer.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, full
Clippy with warnings denied, strict fixture-manifest verification, all Python
support tests, and the pinned mask-selection oracle `--check`. `cargo-mutants`
was unavailable, so the focused mutation run could not be performed.
`trunk build --release` was not applicable because no web, HTML, CSS, WASM
boundary, build configuration, or dependency files changed.
