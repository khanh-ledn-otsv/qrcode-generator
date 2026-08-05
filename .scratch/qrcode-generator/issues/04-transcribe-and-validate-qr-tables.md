# 04 — Transcribe and validate normative QR tables

**What to build:** Supply the standards-derived capacity, block, remainder-bit, alignment, and version data needed by all QR versions and ECC levels, with traceability and exhaustive invariant checks.

**Blocked by:** 01 — Establish the offline workspace baseline.

**Prerequisite:** A developer has licensed access to ISO/IEC 18004:2024.

**Status:** ready-for-agent

- [ ] Transcribed standard data cites the applicable ISO/IEC 18004:2024 clause or table beside the implementation.
- [ ] All 40 versions and four ECC levels have validated total, data, ECC, and block-group values.
- [ ] Remainder-bit counts and alignment coordinates are complete, ordered, unique, and consistent with matrix dimensions.
- [ ] Generated invariant tests execute all 160 version/ECC rows and detect inconsistent totals, block sizes, coordinates, or version dimensions.
- [ ] Version 40, all profile ceilings, and character-count band boundaries have explicit regression coverage.
- [ ] Invalid lookup input returns a typed error and no user-controlled path relies on unchecked indexing.
