# 10 — Select masks using BCH and penalty scoring

**What to build:** Finalize each mask candidate with correct format/version information, score the complete matrix, and select one deterministic standards-conformant result.

**Blocked by:** 09 — Place data and remainder modules with explicit masks.

**Status:** resolved

- [x] Format BCH values are correct for every ECC/mask pair and version BCH values are correct for Versions 7 through 40.
- [x] Isolated tests cover row/column runs, 2×2 blocks, finder-like patterns with required context, and dark-module balance rounding.
- [x] Candidate scoring includes function patterns and candidate-specific format modules.
- [x] The minimum-penalty mask is selected, with the lower mask ID winning ties.
- [x] Explicit-mask golden fixtures and combined synthetic matrices catch predicate, coordinate, BCH, and penalty errors.
- [x] BCH, mask-predicate, and penalty fixtures cite the exact pinned public source files/symbols, agree between both encoders except for the recorded owner-approved Rule 3 interpretation, preserve that exception verbatim, and are labelled `public-corroborated, non-normative` pending a complete 2024 audit.
- [x] Repeated selection from identical input produces identical mask and matrix output.

## Implementation progress

Added independently implemented BCH polynomial division for every format/ECC
combination and Version 7–40 information value. Checked finalization writes
both redundant format copies and both version-information copies only into
their reserved ownership regions after data placement.

Added all four complete-matrix penalty rules: row and column runs, uniform
2×2 blocks, contextual finder-like patterns, and five-percent dark-balance
steps. Automatic selection builds and finalizes all eight candidates, scores
every module including function and candidate-specific information modules,
chooses the minimum, and preserves the lower mask identifier on ties. Stream
version/ECC and placement mask metadata are carried through the typed state so
final format information cannot disagree with the encoded candidate.

The owner resolved the Rule 3 interpretation in favor of literal complete
11-module windows without virtual quiet-zone padding. The accepted fixture
matches python-qrcode and an independent slow reference while preserving
Nayuki's differing run-history totals beside every candidate. Both encoders
still agree on completed matrices and selected masks, and the pinned ZXing-C++
suite independently decodes representative selected artifacts.

Verification is recorded with ticket 11's combined full-suite handoff.
