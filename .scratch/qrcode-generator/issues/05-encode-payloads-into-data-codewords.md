# 05 — Encode preserved payloads into fitted data codewords

**What to build:** Turn an exact user-provided string into a deterministically selected mode, version, and fully padded data-codeword sequence under the release-one whole-payload policy.

**Blocked by:** 04 — Transcribe and validate normative QR tables.

**Status:** ready-for-agent

- [ ] Input is preserved exactly; empty input is invalid and UTF-8 input over 4 KiB is rejected before expensive encoding work.
- [ ] Mode selection chooses Numeric, Alphanumeric, or Byte exactly as specified, with ECI assignment 26 for non-ASCII UTF-8 and no ECI for ASCII Byte payloads.
- [ ] Byte-mode character counts use encoded byte length, and character-count widths change correctly at versions 10 and 27.
- [ ] The first fitting version is selected under the caller’s maximum, accounting for actual ECI and data bits.
- [ ] Terminator, byte alignment, and alternating pad codewords fill the selected data capacity exactly.
- [ ] Exact-fit, one-over, profile-limit, Version 40, malformed-operation, and typed-failure cases are covered without panic.
