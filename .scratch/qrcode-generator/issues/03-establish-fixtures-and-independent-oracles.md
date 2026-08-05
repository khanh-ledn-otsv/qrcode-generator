# 03 — Establish fixture provenance and independent QR oracles

Type: task

**What to build:** Create a reproducible, development-only fixture and oracle workflow that can prove QR output without treating production code as its own correctness oracle.

**Blocked by:** 01 — Establish the offline workspace baseline.

**Prerequisite:** The owner has approved development-only QR generators and the required independent tools can be pinned for the development environment.

**Status:** resolved

- [x] Every fixture records a synthetic payload, hashes, encoding and ECI metadata, version, ECC, mask, source tool versions, generation commands, and independent verification state.
- [x] Explicit-version and explicit-mask fixtures are compared across two independently maintained generators before acceptance.
- [x] Fixture regeneration is an explicit developer action and never occurs implicitly during tests.
- [x] Golden changes produce human-reviewable matrix and metadata differences with updated provenance.
- [x] A pinned independent decoder can inspect production raster artifacts and compare decoded text or bytes plus exposed QR metadata.
- [x] No production crate links to or copies an oracle implementation.

## Answer

Resolved on 2026-08-05. A development-only `fixture-tool` now strictly
validates committed fixture provenance, payload and matrix hashes, QR matrix
dimensions, dual-generator identity and agreement, and accepted verification
state. Two synthetic byte-mode fixtures at explicit versions, ECC levels, and
masks were generated independently by pinned Nayuki 1.8.0 and
`python-qrcode` 8.2 and accepted only after byte-identical matrix comparison.

Regeneration is an explicit action in a locked uv-managed Python environment. It
produces a unified matrix diff on disagreement and marks written fixtures
`pending` until their readable manifest and matrix changes are reviewed.
Ordinary Rust and Python tests never regenerate goldens.

The ZXing-C++ 3.0.2 adapter pins the immutable source commit, rejects a different
or tracked-modified checkout, verifies the decoder binary version, compares
exact decoded bytes, and checks exposed QR version, ECC, and ECI-presence
metadata. The oracle packages and decoder are absent from all production crate
dependency graphs.

Verification passed:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check --target wasm32-unknown-unknown`
- `NO_COLOR=false trunk build --release`
- pinned dual-oracle regeneration check for both committed fixtures
- Python dual-oracle comparison unit tests
