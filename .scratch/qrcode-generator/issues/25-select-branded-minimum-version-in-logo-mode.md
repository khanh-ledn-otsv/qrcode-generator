# 25 — Select a branded minimum version in logo mode

**What to build:** Let logo mode select the approved minimum QR version needed for the reference-like center treatment while preserving exact payload bytes and leaving ordinary no-logo fitting unchanged.

**Blocked by:** 23 — Approve decode-backed branded geometry.

**Status:** ready-for-agent

- [ ] The bundled ONE logo option is selected by default on a new workflow, with opaque white selected and transparency unavailable until the user turns the logo off.
- [ ] Encoding accepts a checked minimum and maximum version, rejects an inverted range with a typed error, and otherwise selects the greater of the first fitting version and approved minimum.
- [ ] The minimum-version path changes only symbol capacity and placement; it never rewrites, pads, trims, normalizes, logs, or transmits user input.
- [ ] No-logo requests continue to select the smallest fitting version exactly as before.
- [ ] Enabling logo mode applies ECC H before fitting and selects at least the branded minimum version established by Ticket 23; disabling it refits using ordinary behavior.
- [ ] Capacity, selected-version, validation, and export diagnostics remain internally consistent and explain when branding caused a larger symbol version.
- [ ] Exact-fit, one-over, minimum-equals-maximum, naturally-larger, invalid-range, state-transition, WASM, and browser tests cover the complete behavior.
