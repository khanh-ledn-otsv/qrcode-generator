# 06 — Generate Reed–Solomon error-correction codewords

**What to build:** Generate deterministic QR error-correction codewords from data blocks using independently verified GF(256) arithmetic.

**Blocked by:** 04 — Establish and validate QR tables.

**Status:** ready-for-agent

- [ ] GF(256) operations use the QR primitive polynomial and satisfy zero, inverse, cycle, multiplication, and division properties.
- [ ] Optimized multiplication agrees with a deliberately simple test-only polynomial reference.
- [ ] Generator polynomials and remainders are verified for every ECC degree required by the standard tables.
- [ ] Constants, generator degrees, and remainder fixtures cite the pinned public-source evidence defined in `docs/research/qr-public-source-provenance.md` and are labelled `public-corroborated, non-normative` pending a complete 2024 audit.
- [ ] Leading and trailing zero data, maximum supported blocks, and output lengths have explicit coverage.
- [ ] Input buffers are not mutated, arithmetic is checked where applicable, and invalid requests return typed errors without panic.
- [ ] Property and mutation tests can detect altered constants, shifts, loop boundaries, and comparison errors.
