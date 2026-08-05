# 10 — Select masks using BCH and penalty scoring

**What to build:** Finalize each mask candidate with correct format/version information, score the complete matrix, and select one deterministic standards-conformant result.

**Blocked by:** 09 — Place data and remainder modules with explicit masks.

**Status:** ready-for-agent

- [ ] Format BCH values are correct for every ECC/mask pair and version BCH values are correct for Versions 7 through 40.
- [ ] Isolated tests cover row/column runs, 2×2 blocks, finder-like patterns with required context, and dark-module balance rounding.
- [ ] Candidate scoring includes function patterns and candidate-specific format modules.
- [ ] The minimum-penalty mask is selected, with the lower mask ID winning ties.
- [ ] Explicit-mask golden fixtures and combined synthetic matrices catch predicate, coordinate, BCH, and penalty errors.
- [ ] Repeated selection from identical input produces identical mask and matrix output.
