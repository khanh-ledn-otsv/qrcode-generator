# 11 — Expose and prove the standards-conformant encoder

**What to build:** Offer a focused public encoding boundary that returns immutable classified matrices and diagnostics, backed by independent conformance evidence across the supported QR space.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 10 — Select masks using BCH and penalty scoring.

**Status:** ready-for-agent

- [ ] The public request accepts exact text, ECC, and a maximum version and returns version, mode, mask, bit-capacity diagnostics, and an immutable classified matrix.
- [ ] User failures and internal invariant failures are typed and no user-controlled path panics, unwraps, or performs unchecked indexing.
- [ ] Representative golden matrices cover required version boundaries, all ECC levels, all masks, all supported modes, and UTF-8 ECI.
- [ ] Properties verify first-fit version selection, complete cell ownership, deterministic output, monotonic limits, and exact success/failure boundaries.
- [ ] Random safe-style artifacts independently decode to identical text or bytes, including exposed ECI metadata where supported.
- [ ] Documented local commands replay committed core fuzz regressions, and a native diagnostic example exposes no payload through logs or metadata.
