# 04 — Establish and validate QR tables

Type: task

**What to build:** Supply the capacity, block, remainder-bit, alignment, and version data needed by all QR versions and ECC levels, with dual-oracle provenance and exhaustive invariant checks.

**Blocked by:** 01 — Establish the offline workspace baseline.

**Prerequisite:** The project owner accepts pinned independent QR implementations as development-only table oracles when a complete licensed standard is unavailable.

**Status:** resolved

- [x] Production data cites the applicable ISO/IEC 18004:2024 clauses and a committed non-normative fixture records two pinned independent oracle implementations without copying oracle implementation code.
- [x] All 40 versions and four ECC levels have validated total, data, ECC, and block-group values.
- [x] Remainder-bit counts and alignment coordinates are complete, ordered, unique, and consistent with matrix dimensions.
- [x] Generated invariant tests execute all 160 version/ECC rows and detect inconsistent totals, block sizes, coordinates, or version dimensions.
- [x] Version 40, all profile ceilings, and character-count band boundaries have explicit regression coverage.
- [x] Invalid lookup input returns a typed error and no user-controlled path relies on unchecked indexing.

## Answer

Implemented `qr-core::tables` with typed ECC/mode values, validated `Version`
lookups, typed raw-version errors, checked block-group expansion, complete
capacity/remainder/alignment data, and character-count widths. The compact
production tables cite the applicable ISO/IEC 18004:2024 locations and are
checked against a committed 160-row fixture produced by pinned `qrcodegen`
1.8.0 and `python-qrcode` 8.2 development-only oracles. Structural tests also
independently account for total codewords and remainder bits from version and
function-module geometry.

The development plan and testing strategy now retain ISO/IEC 18004:2024 as the
normative definition while allowing the accepted dual-oracle fixture workflow
when a complete licensed copy is unavailable. The project owner explicitly
approved that policy change on 2026-08-05. Fixture regeneration remains an
explicit `uv` command and never runs implicitly during Rust tests.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, full
Clippy with warnings denied, WASM target checking, `NO_COLOR=false trunk build
--release`, Python oracle unit tests, and the pinned table-fixture `--check`.
