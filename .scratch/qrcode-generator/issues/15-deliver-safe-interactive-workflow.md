# 15 — Deliver the safe interactive QR workflow

**What to build:** Let a user enter an exact payload, choose a profile, and receive a responsive safe QR preview with accurate capacity and validation state entirely in the browser.

**Blocked by:** 11 — Expose and prove the standards-conformant encoder; 13 — Export deterministic safe SVG artifacts.

**Status:** ready-for-agent

- [ ] Payload entry preserves every character and displays character count separately from UTF-8 byte count.
- [ ] Four profile choices derive maximum version, selected version, ECC, used/available data bits, data codewords, dimensions, and print guidance.
- [ ] Empty, over-limit, over-capacity, and internal failure states produce associated validation messages and disable export actions.
- [ ] Control characters produce a deterministic caution without rewriting or rejecting otherwise valid plain text.
- [ ] Debounced preview work uses latest-value-wins semantics and cannot replace current state with stale results.
- [ ] State transitions are testable in native Rust and the user-visible workflow remains usable at supported desktop and mobile widths.
