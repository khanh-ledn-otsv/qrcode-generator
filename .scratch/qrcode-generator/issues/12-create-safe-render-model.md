# 12 — Create the safe render model and fixed-canvas placement

**What to build:** Transform an immutable encoded QR matrix and validated profile into deterministic render geometry without allowing rendering to alter encoding decisions or module ownership.

**Blocked by:** 02 — Define validated output profiles and canvas geometry; 11 — Expose and prove the standards-conformant encoder.

**Status:** ready-for-agent

- [ ] Rendering accepts an encoded QR value plus validated options and cannot change ECC, version, mask, module values, or module kinds.
- [ ] The safe preset is black on opaque white with square modules, standard finders, no logo, no border, and no strokes.
- [ ] Shared symbol geometry represents the matrix and exactly four quiet-zone modules per side once; artifact placement cannot change that geometry.
- [ ] SVG placement uses a tight logical matrix-plus-quiet-zone extent, while PNG placement uses the profile's fixed pixel canvas with symmetric background-only surplus padding.
- [ ] Branding geometry is structurally unable to overwrite function modules.
- [ ] Checked dimensions and allocation bounds reject impossible render requests with typed errors.
- [ ] Identical encoded input and options produce identical render models on native and WASM targets.
