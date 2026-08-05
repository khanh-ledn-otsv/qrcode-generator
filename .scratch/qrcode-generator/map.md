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

## Notes

- Implementation tickets are tracked under `issues/`.

## Fog

- None recorded.
