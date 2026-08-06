# 09 — Place data and remainder modules with explicit masks

**What to build:** Complete a QR matrix by placing the interleaved bitstream and classified remainder modules in the standard traversal, with deterministic explicit-mask support for conformance testing.

**Blocked by:** 07 — Construct the complete interleaved codeword stream; 08 — Build classified QR function matrices.

**Status:** claimed

- [ ] Zig-zag traversal skips timing column 6 and every function cell while assigning each writable cell exactly once.
- [ ] Payload bits and remainder bits retain distinct module classifications after placement.
- [ ] Every mask predicate can be applied only to data and remainder modules without changing function modules.
- [ ] Test-only explicit-mask construction supports all eight masks without exposing mask choice in the product UI.
- [ ] Ownership tests detect missing, duplicated, transposed, rotated, or mirrored placement.
- [ ] Explicit version/mask placement fixtures agree with both pinned public encoders, include local ownership/coverage invariants, and record the public-source provenance policy label.
- [ ] Stream-length or matrix-ownership mismatches return typed invariant errors without partial successful output.
