# 07 — Construct the complete interleaved codeword stream

**What to build:** Split padded data into the standard block layout, add error correction, and produce the exact interleaved stream consumed by matrix placement.

**Blocked by:** 05 — Encode preserved payloads into fitted data codewords; 06 — Generate Reed–Solomon error-correction codewords.

**Status:** resolved

- [x] Data codewords are consumed exactly once across both one-group and two-group block layouts.
- [x] Every block receives the table-defined ECC length and short/long blocks interleave in the required order.
- [x] Split and interleave fixtures agree between the pinned public encoders where exposed and record the public-source provenance policy label.
- [x] Test-only de-interleaving reconstructs the original data and ECC blocks.
- [x] A generated test exercises every distinct layout and all 160 version/ECC rows.
- [x] Final stream length agrees with total codewords, while the following placement stage receives the correct remainder-bit count.
- [x] Inconsistent table data or impossible lengths fail with typed errors rather than partial output or panic.

## Answer

Added the public `qr_core::codeword_stream::construct` seam. It validates the
data-codeword count for the requested version/ECC row, splits data across the
declared one- or two-group block layout, generates each block's Reed–Solomon
codewords, interleaves short/long data blocks followed by ECC blocks, and
returns the exact codeword stream plus the version's remainder-bit count.

Committed one-group Version 1-M and two-group Version 5-Q fixtures agree
byte-for-byte between pinned Nayuki 1.8.0 and python-qrcode 8.2. The strict
algorithm-fixture manifest records the executed and evidence sources, supporting
python-qrcode symbols, artifact hash, commands, and local invariant coverage.
An independent test-only de-interleaver reconstructs and revalidates every data
and ECC block across all 160 version/ECC rows.

Verification passed: formatting, workspace check/test/Clippy with warnings
denied, strict manifest validation, QR table/Reed–Solomon/interleaving oracle
checks, and focused `cargo-mutants` 27.1.0 verification. All 10 viable
interleaving mutants were caught; four additional mutants were unviable.
