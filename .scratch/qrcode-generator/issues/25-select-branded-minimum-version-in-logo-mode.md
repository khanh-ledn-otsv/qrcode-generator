# 25 — Select a branded minimum version in logo mode

**What to build:** Let logo mode select the approved minimum QR version needed for the reference-like center treatment while preserving exact payload bytes and leaving ordinary no-logo fitting unchanged.

**Blocked by:** 23 — Approve decode-backed branded geometry.

**Status:** resolved

- [x] The bundled ONE logo option is selected by default on a new workflow, with opaque white selected and transparency unavailable until the user turns the logo off.
- [x] Encoding accepts a checked minimum and maximum version, rejects an inverted range with a typed error, and otherwise selects the greater of the first fitting version and approved minimum.
- [x] The minimum-version path changes only symbol capacity and placement; it never rewrites, pads, trims, normalizes, logs, or transmits user input.
- [x] No-logo requests continue to select the smallest fitting version exactly as before.
- [x] Enabling logo mode applies ECC H before fitting and selects at least the branded minimum version established by Ticket 23; disabling it refits using ordinary behavior.
- [x] Capacity, selected-version, validation, and export diagnostics remain internally consistent and explain when branding caused a larger symbol version.
- [x] Exact-fit, one-over, minimum-equals-maximum, naturally-larger, invalid-range, state-transition, WASM, and browser tests cover the complete behavior.

## Answer

`qr-core` requests now carry checked minimum and maximum versions. Encoding
selects the greater of the payload's ordinary first fit and the request
minimum, reports inverted ranges through `InvalidVersionRange`, and records
whether the minimum actually enlarged the symbol. Version 1 remains the
explicit minimum at every existing no-logo and test call site.

The web workflow requests ECC H and Version 6 before logo fitting. Diagnostics
report the minimum and explain when branding raised the selected version;
profiles below Version 6 receive a specific validation message and disabled
exports. Disabling the logo restores ECC M and ordinary first fitting. Native,
WASM, Playwright, exact-capacity, independent-decode, and deterministic
download coverage exercise the complete transition without changing payload
bytes.
