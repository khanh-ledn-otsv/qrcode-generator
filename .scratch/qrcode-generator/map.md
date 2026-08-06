# QR Code Generator — Implementation map

## Decisions-so-far

- [Ticket 02](issues/02-define-output-profiles-and-geometry.md): Output profiles
  are compiled constants; QR versions belong to `qr-core`; and `qr-render`
  calculates checked, centered, background-only fixed-canvas geometry.
- [Ticket 03](issues/03-establish-fixtures-and-independent-oracles.md): Strict
  fixture provenance and hashes are checked by a development-only Rust tool;
  pinned Nayuki and `python-qrcode` generators must agree on explicit matrices;
  and pinned ZXing-C++ independently checks raster payload bytes and metadata.
- [Ticket 04](issues/04-transcribe-and-validate-qr-tables.md): Complete QR Model
  2 capacity, block, remainder, alignment, and character-width data is exposed
  through typed checked lookups and exhaustively validated against a committed
  160-row fixture from pinned independent development oracles.
- [Ticket 05](issues/05-encode-payloads-into-data-codewords.md): Exact input is
  encoded as one deterministic Numeric, Alphanumeric, or Byte segment with
  UTF-8 ECI 26 when required, first-fit version selection, typed failures, and
  fully padded data codewords.
- [Ticket 08](issues/08-build-classified-function-matrices.md): Checked matrix
  construction classifies every function region for all 40 versions, validates
  reservations and finalization, and is covered by exact all-version invariants
  plus dual-oracle readable fixtures for Versions 1, 2, 7, and 40.
- [Ticket 09](issues/09-place-data-remainder-and-explicit-masks.md): Checked
  zig-zag placement preserves data/remainder ownership, protects every function
  module, supports all eight explicit masks through a typed core API, and
  matches dual-oracle fixtures at Versions 1, 2, 7, and 40.
- [Ticket 12](issues/12-create-safe-render-model.md): A borrowed immutable
  render model preserves encoded ownership and diagnostics, centralizes the
  exact quiet-zone extent, separates tight SVG from fixed-canvas PNG placement,
  checks allocation bounds, and exposes only typed data/remainder branding
  targets.
- [Ticket 13](issues/13-export-safe-svg-artifacts.md): A dependency-free
  production exporter emits compact deterministic safe SVG with exact logical
  placement; parsed all-profile/version structure, independent rasterization,
  and pinned ZXing decode prove the artifact boundary.
- [Ticket 14](issues/14-export-safe-png-artifacts.md): A pinned direct PNG
  exporter paints checked RGBA rectangles, emits deterministic metadata-free
  artifacts, matches a native/WASM hash contract, and passes parsed-pixel plus
  pinned ZXing decode gates across every supported profile and version.
## Notes

- Implementation tickets are tracked under `issues/`.

## Fog

- [Ticket 10 penalty-oracle disagreement](penalty-oracle-disagreement.md):
  Nayuki 1.8.0 and python-qrcode 8.2 agree on completed matrices but disagree
  on exposed Rule 3 penalty totals, blocking fixture acceptance under the
  public-source provenance policy.
