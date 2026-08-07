# 20 — Harden all approved output combinations

**What to build:** Turn the complete approved configuration surface into enforceable release-quality evidence through exhaustive combination coverage and deeper robustness automation.

**Blocked by:** 18 — Add approved module and finder styling; 19 — Integrate the bundled logo safely.

**Status:** resolved

- [x] A generated matrix covers every selectable foreground, background, transparency, finder style, logo state, profile, and required payload class.
- [x] Each tuple records safety classification and independently decodes or produces an explicitly expected invalid geometry result.
- [x] Deterministic adverse transforms cover blur, scaling simulation, JPEG screenshots, rotation, perspective, contrast, brightness, backgrounds, dot gain, ink loss, and grayscale using recorded parameters.
- [x] Documented local and release commands exercise the table, golden, browser, fuzz, Miri, mutation, coverage, dependency, and artifact-evidence policies.
- [x] Coverage and mutation thresholds are enforced for stabilized critical core, geometry, rendering, and testable web-state code without broad exclusions.
- [x] Performance and allocation measurements establish reproducible baselines and flag regressions without flaky wall-clock unit assertions.

## Answer

Added a generated 96-row approved-output matrix spanning all six selectable
configuration dimensions and six required payload classes. The release evidence
records independently decoded artifacts and explicit typed rejections for
unsupported logo combinations. Added deterministic, manifest-driven adverse
transforms; coverage and mutation gates; eight fuzz targets; Miri, dependency,
and evidence commands; Criterion performance coverage; and deterministic
artifact/allocation ceilings.

The release-hardening runbook records the exact tools, commands, thresholds,
evidence locations. Automated
verification passed the routine Rust/WASM/browser suite, independent ZXing PNG,
SVG, logo, and adverse decoding, all enforced coverage scopes, the benchmark
smoke run, and fuzz-target compilation.

## Comments

- 2026-08-07: The approved matrix was reduced from 192 to 96 rows after rounded
  modules were removed. Bundle-size testing and evidence were removed from the
  release policy; artifact and allocation baselines remain.
