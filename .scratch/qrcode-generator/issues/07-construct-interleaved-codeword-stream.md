# 07 — Construct the complete interleaved codeword stream

**What to build:** Split padded data into the standard block layout, add error correction, and produce the exact interleaved stream consumed by matrix placement.

**Blocked by:** 05 — Encode preserved payloads into fitted data codewords; 06 — Generate Reed–Solomon error-correction codewords.

**Status:** ready-for-agent

- [ ] Data codewords are consumed exactly once across both one-group and two-group block layouts.
- [ ] Every block receives the table-defined ECC length and short/long blocks interleave in the required order.
- [ ] Test-only de-interleaving reconstructs the original data and ECC blocks.
- [ ] A generated test exercises every distinct layout and all 160 version/ECC rows.
- [ ] Final stream length agrees with total codewords, while the following placement stage receives the correct remainder-bit count.
- [ ] Inconsistent table data or impossible lengths fail with typed errors rather than partial output or panic.
