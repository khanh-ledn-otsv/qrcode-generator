# 17 — Add approved color, contrast, and transparency behavior

**What to build:** Let users select only owner-approved foreground/background combinations and transparent output, with measurable safety classification and realistic surface cautions.

**Blocked by:** 16 — Complete diagnostics, downloads, accessibility, and privacy.

**Status:** resolved

- [x] The safe black-on-white preset remains the default; the accepted `#BD0F72`-on-white brand preset is selectable only when it satisfies the 4.5:1 opaque contrast rule and decode suite.
- [x] Unsafe opaque contrast is invalid and disables export with an associated explanation.
- [x] Optional transparency ships in release 1, is never the default, and is classified as a caution because effective placement contrast is unknown.
- [x] Transparent previews cover documented white, light-gray, dark, and patterned surfaces without modifying encoded modules.
- [x] SVG and PNG backgrounds, quiet zones, and surplus padding consistently implement opaque or zero-alpha behavior.
- [x] Every selectable color/background/profile tuple appears in generated structural, deterministic, and independent-decode tests.

## Answer

Resolved on 2026-08-07. Rendering now exposes only the compiled black and
`#BD0F72` foreground presets with opaque white or transparent backgrounds. A
measured 4.5:1 opaque contrast gate rejects unsafe output through a typed error;
transparent output remains exportable with an explicit caution and unknown
effective-placement contrast.

The Leptos workflow defaults to black on opaque white, adds keyboard-native
appearance controls and diagnostics, and shows the same transparent SVG over
white, light-gray, dark, and patterned preview surfaces. SVG omits its
background rectangle for transparency, while PNG quiet zones and fixed-canvas
padding use zero-alpha pixels.

Generated structural and byte-determinism tests enumerate all 16 approved
foreground/background/profile tuples. The manifest-pinned ZXing-C++ suites
independently decoded every tuple for both rasterized SVG and PNG on an
effective white placement surface. The complete Node 24 `pnpm run verify`
suite passed, including Rust, WASM, Python oracle, release-build, privacy,
desktop/mobile Chromium, and axe checks.
