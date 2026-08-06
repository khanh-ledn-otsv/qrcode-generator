# 09 — Place data and remainder modules with explicit masks

**What to build:** Complete a QR matrix by placing the interleaved bitstream and classified remainder modules in the standard traversal, with deterministic explicit-mask support for conformance testing.

**Blocked by:** 07 — Construct the complete interleaved codeword stream; 08 — Build classified QR function matrices.

**Status:** resolved

- [x] Zig-zag traversal skips timing column 6 and every function cell while assigning each writable cell exactly once.
- [x] Payload bits and remainder bits retain distinct module classifications after placement.
- [x] Every mask predicate can be applied only to data and remainder modules without changing function modules.
- [x] Test-only explicit-mask construction supports all eight masks without exposing mask choice in the product UI.
- [x] Ownership tests detect missing, duplicated, transposed, rotated, or mirrored placement.
- [x] Explicit version/mask placement fixtures agree with both pinned public encoders, include local ownership/coverage invariants, and record the public-source provenance policy label.
- [x] Stream-length or matrix-ownership mismatches return typed invariant errors without partial successful output.

## Answer

Added checked data placement to `qr-core::matrix`. `place_data` validates that
its input is the canonical unplaced function matrix, verifies the interleaved
stream plus remainder-bit length against writable ownership, traverses the
matrix in the standard two-column zig-zag while skipping timing column 6 and
all function modules, and returns a new immutable completed matrix. Data and
remainder modules retain distinct `ModuleKind` values.

Added the validated `MaskId` domain type and all eight explicit predicates.
Masks apply only while data/remainder cells are written; function cells are
compared before and after placement and remain unchanged. Already-placed,
malformed-ownership, length-overflow, stream-length, and incomplete-traversal
states return typed errors without a partial successful result.

Committed dual-oracle fingerprints for all eight masks at Versions 1, 2, 7,
and 40, plus readable classified maps for one mask at each version. The
development verifier instruments Nayuki 1.8.0 and python-qrcode 8.2 placement
routines, requires exact traversal/matrix agreement, and records the evidence
as `public-corroborated, non-normative`. Generated tests also cover ownership
and function preservation for every version.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, full
Clippy with warnings denied, strict fixture-manifest verification, all Python
support tests, and the pinned placement-matrix oracle `--check`. `trunk build
--release` was not applicable because no web, HTML, CSS, WASM boundary, build
configuration, or dependency files changed.
