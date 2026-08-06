# 05 — Encode preserved payloads into fitted data codewords

Type: task

**What to build:** Turn an exact user-provided string into a deterministically selected mode, version, and fully padded data-codeword sequence under the release-one whole-payload policy.

**Blocked by:** 04 — Establish and validate QR tables.

**Status:** resolved

- [x] Input is preserved exactly; empty input is invalid and UTF-8 input over 4 KiB is rejected before expensive encoding work.
- [x] Mode selection chooses Numeric, Alphanumeric, or Byte exactly as specified, with ECI assignment 26 for non-ASCII UTF-8 and no ECI for ASCII Byte payloads.
- [x] Byte-mode character counts use encoded byte length, and character-count widths change correctly at versions 10 and 27.
- [x] The first fitting version is selected under the caller’s maximum, accounting for actual ECI and data bits.
- [x] Terminator, byte alignment, and alternating pad codewords fill the selected data capacity exactly.
- [x] Exact-fit, one-over, profile-limit, Version 40, malformed-operation, and typed-failure cases are covered without panic.

## Answer

Added a checked MSB-first bit buffer and the public whole-payload encoding seam
`encode(EncodeRequest) -> Result<EncodedData, EncodingError>`. Encoding preserves
the input bytes, applies the release-one Numeric/Alphanumeric/Byte policy,
emits UTF-8 ECI assignment 26 only for non-ASCII Byte payloads, selects the
first fitting version for the requested ECC, and returns a fully padded data
codeword sequence plus capacity diagnostics.

Public tests cover dual-oracle ASCII codeword literals, a pinned ECI oracle
literal, whitespace preservation, the complete alphanumeric alphabet, all four
ECC levels, zero-room and truncated-terminator fits, one-over selection,
profile ceilings 5/8/12/13, character-count transitions at versions 10 and 27,
Version 40, the 4 KiB input boundary, and typed malformed-operation failures.
Boundary-biased property tests independently check bit lengths, all bit-buffer
widths, deterministic output, first-fit selection, maximum-version monotonicity,
same-mode growth, and exact capacity filling.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, full
Clippy with warnings denied, WASM target checking, `NO_COLOR=false trunk build
--release`, Python oracle unit tests, and the pinned QR-table fixture check.
