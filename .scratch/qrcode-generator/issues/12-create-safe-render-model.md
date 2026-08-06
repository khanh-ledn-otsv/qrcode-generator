# 12 — Create the safe render model and fixed-canvas placement

**What to build:** Transform an immutable encoded QR matrix and validated profile into deterministic render geometry without allowing rendering to alter encoding decisions or module ownership.

**Blocked by:** 02 — Define validated output profiles and canvas geometry; 11 — Expose and prove the standards-conformant encoder.

**Status:** resolved

- [x] Rendering accepts an encoded QR value plus validated options and cannot change ECC, version, mask, module values, or module kinds.
- [x] The safe preset is black on opaque white with square modules, standard finders, no logo, no border, and no strokes.
- [x] Shared symbol geometry represents the matrix and exactly four quiet-zone modules per side once; artifact placement cannot change that geometry.
- [x] SVG placement uses a tight logical matrix-plus-quiet-zone extent, while PNG placement uses the profile's fixed pixel canvas with symmetric background-only surplus padding.
- [x] Branding geometry is structurally unable to overwrite function modules.
- [x] Checked dimensions and allocation bounds reject impossible render requests with typed errors.
- [x] Identical encoded input and options produce identical render models on native and WASM targets.

## Implementation summary

Added a borrowed, immutable `EncodedQr + RenderOptions -> RenderModel` boundary
with a single validated safe preset. Shared symbol geometry owns the matrix
extent and four-module quiet zone once; SVG placement exposes that tight
logical view box, while PNG placement exposes the checked profile canvas,
integer scale, exact matrix origin, symmetric background-only padding, and
prevalidated RGBA allocation length under a target-independent 64 MiB ceiling.

Render cells preserve the encoder's values and ownership. Function modules are
reported as protected, while the separately typed branding-target iterator can
only yield data and remainder cells and has no public constructor. Native tests
cover the public behavior and deterministic reconstruction, and the crate also
checks successfully for `wasm32-unknown-unknown`.

Verification passed: `cargo fmt --check`, `cargo check`, `cargo test`, Clippy
for all targets/features with warnings denied, `cargo check -p qr-render
--target wasm32-unknown-unknown`, and `trunk build --release`. The release build
used `NO_COLOR=true` because Trunk 0.21.14 rejects the ambient `NO_COLOR=1`.
