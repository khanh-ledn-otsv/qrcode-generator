# 15 — Deliver the safe interactive QR workflow

**What to build:** Let a user enter an exact payload, choose a profile, and receive a responsive safe QR preview with accurate capacity and validation state entirely in the browser.

**Blocked by:** 11 — Expose and prove the standards-conformant encoder; 13 — Export deterministic safe SVG artifacts.

**Status:** resolved

- [x] Payload entry preserves every character and displays character count separately from UTF-8 byte count.
- [x] Every non-logo workflow requests fixed ECC M; ECC is visible in diagnostics but is not user-selectable in release 1.
- [x] Four profile choices derive maximum version, selected version, used/available data bits, data codewords, dimensions, and print guidance without changing ECC.
- [x] Native state tests prove that profile changes refit at ECC M, while the later logo transition changes to ECC H before fitting and disabling it restores ECC M.
- [x] Empty, over-limit, over-capacity, and internal failure states produce associated validation messages and disable export actions.
- [x] Control characters produce a deterministic caution without rewriting or rejecting otherwise valid plain text.
- [x] Debounced preview work uses latest-value-wins semantics and cannot replace current state with stale results.
- [x] State transitions are testable in native Rust and the user-visible workflow remains usable at supported desktop and mobile widths.

## Answer

Resolved on 2026-08-06. `qr-web` now separates browser-independent workflow
state from the Leptos view. Revisioned preview requests preserve the exact
payload, select ECC M for safe output (and H before the future logo refit),
classify user-facing failures without exposing payload content, and reject
stale completions. The responsive Leptos workflow adds exact character/UTF-8
byte counts, keyboard-native profile radios, cancellable 250 ms preview work,
deterministic SVG preview, capacity/geometry diagnostics, associated
validation, and deterministic control-character caution.

Ten native state tests cover all acceptance transitions, raw line-ending
preservation, and profile contracts. Verification passed: `cargo fmt --check`, `cargo check`, `cargo
test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check
--target wasm32-unknown-unknown`, and `NO_COLOR=true trunk build --release`.
Interactive desktop/mobile browser inspection could not run because no browser
connection was available in the execution environment; ticket 16 retains the
planned cross-browser end-to-end and accessibility coverage.
