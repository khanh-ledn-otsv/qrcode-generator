# 11 — Expose and prove the standards-conformant encoder

**What to build:** Offer a focused public encoding boundary that returns immutable classified matrices and diagnostics, backed by independent conformance evidence across the supported QR space.

**Blocked by:** 03 — Establish fixture provenance and independent QR oracles; 10 — Select masks using BCH and penalty scoring.

**Status:** resolved

- [x] The public request accepts exact text, ECC, and a maximum version and returns version, mode, mask, bit-capacity diagnostics, and an immutable classified matrix.
- [x] User failures and internal invariant failures are typed and no user-controlled path panics, unwraps, or performs unchecked indexing.
- [x] Representative golden matrices cover required version boundaries, all ECC levels, all masks, all supported modes, and UTF-8 ECI.
- [x] Properties verify first-fit version selection, complete cell ownership, deterministic output, monotonic limits, and exact success/failure boundaries.
- [x] Random safe-style artifacts independently decode to identical text or bytes, including exposed ECI metadata where supported.
- [x] Documented local commands replay committed core fuzz regressions, and a native diagnostic example exposes no payload through logs or metadata.

## Implementation summary

Added the focused root `EncodeRequest -> Result<EncodedQr, EncodeError>`
boundary. It composes the separately tested data encoder, block/interleaving,
placement, BCH finalization and owner-approved mask selection while exposing
only immutable diagnostics and classified modules.

The public properties cover typed boundaries, first-fit/monotonic behavior,
complete ownership and deterministic output. New dual-oracle composed goldens
cover Versions 1, 2, 6, 7, 9, 10, 26, 27 and 40, every ECC level and mask,
all supported modes, UTF-8 ECI, and the version-band boundary pairs. The
seeded pinned ZXing-C++ suite independently proves exact bytes and exposed
metadata across the same public boundary. Added a
committed libFuzzer target/corpus replay and a stdin-driven diagnostic example
that prints diagnostics but never the payload.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, Clippy
for all targets/features with warnings denied, strict eight-fixture manifest
validation, all Python oracle support tests, explicit pinned ZXing decode,
the standalone fuzz target build, and `trunk build --release`. `cargo-fuzz` is
not installed locally, so the documented open-ended smoke command was not run;
the committed corpus replay and target compilation both passed.
