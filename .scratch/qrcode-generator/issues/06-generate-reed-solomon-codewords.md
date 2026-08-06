# 06 — Generate Reed–Solomon error-correction codewords

**What to build:** Generate deterministic QR error-correction codewords from data blocks using independently verified GF(256) arithmetic.

**Blocked by:** 04 — Establish and validate QR tables.

**Status:** resolved

- [x] GF(256) operations use the QR primitive polynomial and satisfy zero, inverse, cycle, multiplication, and division properties.
- [x] Optimized multiplication agrees with a deliberately simple test-only polynomial reference.
- [x] Generator polynomials and remainders are verified for every ECC degree required by the standard tables.
- [x] Constants, generator degrees, and remainder fixtures cite the pinned public-source evidence defined in `docs/research/qr-public-source-provenance.md` and are labelled `public-corroborated, non-normative` pending a complete 2024 audit.
- [x] Leading and trailing zero data, maximum supported blocks, and output lengths have explicit coverage.
- [x] Input buffers are not mutated, arithmetic is checked where applicable, and invalid requests return typed errors without panic.
- [x] Property and mutation tests can detect altered constants, shifts, loop boundaries, and comparison errors.

## Answer

Added the public `qr_core::reed_solomon` seam with checked GF(256)
multiplication/division, generator-polynomial construction, and deterministic
error-correction codeword generation. Unsupported QR ECC degrees, division by
zero, and blocks beyond the field limit return typed errors; input slices are
never mutated.

Committed fixtures cover all 13 ECC degrees used by the 160 QR table rows,
leading/trailing zero data, and the maximum table-defined block of 123 data plus
30 ECC codewords. An explicit development-only verifier requires exact
agreement between pinned Nayuki 1.8.0 and python-qrcode 8.2 outputs. Exhaustive
field checks and property tests compare production arithmetic and remainders
with independently written polynomial references.

Verification passed: formatting, workspace check/test/Clippy with warnings
denied, the pinned Python oracle tests and Reed–Solomon fixture check, and a
release Trunk build. Focused `cargo-mutants` 27.1.0 verification caught 55 of
57 viable mutants (96.5%, above the 90% critical-arithmetic target); 10 mutants
were unviable and none timed out after an isolated rerun. The two survivors were
triaged as equivalent for the public QR domain: changing the generator update
from XOR to OR produces identical coefficients at every one of the 13 supported
degrees, and extending the exponent-table loop through index 255 writes the
same repeated field-cycle value already assigned by the following loop.
