# 32 — Simplify output and automate QR correctness

> Owner clarification (2026-08-10): compact rounded modules are the only output
> appearance, with no appearance control, and the bundled logo is enabled by
> default. The logo can be disabled to restore ECC M and ordinary first-fit
> sizing. These decisions supersede the narrower default-output wording below.

**What to build:** Fix the CI caches, add hosted correctness gates, automate the
independent decoder campaigns, add the documented secondary decoder, remove
transparent output completely, and make the ordinary output path more
scanner-conservative without changing payload bytes or QR encoding semantics.

**Blocked by:** none.

**Type:** task

**Status:** resolved

- [x] Fix pnpm caching so the workflow never asks `actions/setup-node` to use
  pnpm before pnpm is available. Set up pinned Node.js first, enable the
  package-manager version declared by `packageManager`, resolve the pnpm store
  path, and cache that store with a lockfile-, runner-, and architecture-aware
  key before `pnpm install --frozen-lockfile`.
- [x] Define the Trunk version once for the workflow. A restored or
  runner-provided binary may skip installation only when `trunk --version`
  exactly equals `trunk 0.21.14`; otherwise install the pinned version with
  `--locked --force`. Keep the cache key tied to the same single version
  source.
- [x] Add a least-privilege hosted correctness workflow for pull requests and
  pushes to `main`. It uses Node.js 24, pnpm 11.20.0, Rust 1.97.1, the locked uv
  project, the WASM target/runner, and Chromium, and gates changes on the
  repository's routine verification command. Deployment must not publish a
  revision that failed the applicable correctness gate.
- [x] Cache only reproducible development inputs and build outputs, with keys
  that include their controlling lockfile, toolchain, tool version, runner OS,
  and architecture. A cache miss or corrupt/incompatible restored tool must
  fall back to the pinned installation path rather than weakening a version
  check.
- [x] Add a hosted extended decoder job on `main`, on a schedule, or both. It
  builds and verifies the manifest-pinned ZXing-C++ checkout, then runs the
  seeded core decode, complete native-PNG, independently rasterized-SVG,
  bundled-logo, and adverse-transform campaigns with no retries. Cache the
  verified checkout/build by source commit and build inputs without treating a
  restored binary as trusted until its commit, worktree, submodules, and
  reported version pass the existing verification.
- [x] Upload decoder logs and relevant failure evidence when an extended job
  fails. Successful jobs must not implicitly regenerate or commit golden
  fixtures or release evidence.
- [x] Implement the documented test-only `quirc` secondary decoder for a
  representative synthetic ASCII raster set spanning ordinary, dense,
  unbranded, and branded outputs where applicable. Pin its source/version and
  provenance, compare exact decoded payload bytes, and keep it out of all
  production crates. ZXing-C++ remains authoritative for UTF-8/ECI metadata.
- [x] Reconcile decoder documentation with the executable manifest: remove the
  stale ZXing-C++ 2.3.0 claim and consistently document the currently pinned
  3.0.2 source commit, or deliberately update the manifest and complete decode
  evidence together.
- [x] Remove transparent background from the public product and internal
  selectable appearance model. Delete its UI control, workflow transitions,
  background variant, placement caution, alpha/compositing branches that exist
  only for transparent exports, approved-combination rows, tests, guidance,
  and stale generated evidence. SVG and PNG exports always use an opaque white
  background and retain exactly four quiet-zone modules per side.
- [x] Use the owner-approved ordinary output: compact rounded modules, standard
  square finders, opaque white background, and the bundled logo by default.
  Keep the bundled logo enabled by default and removable within its existing ECC-H,
  function-safe, decode-approved geometry. Do not add a second foreground,
  arbitrary styling, or another production encoder. If the product keeps a
  branded compact-dot path, present the conservative path as a single clear
  compatibility choice rather than a combinatorial style editor.
- [x] Recalculate the selectable output matrix and resource/adverse baselines
  after transparent output and any appearance rows are removed. Every retained
  PNG and SVG row must remain deterministic across native and WASM, preserve
  the encoded matrix, and pass the required independent decode policy.
- [x] Add exact tests proving that removing transparency and changing defaults
  never trims, normalizes, rewrites, logs, or transmits payloads and never
  changes mode, ECI, selected version, ECC, mask, or modules except for the
  already-defined logo transition from ECC M to ECC H.
- [x] Update `docs/DEVELOPMENT_PLAN.md`, `docs/TESTING_STRATEGY.md`, release
  runbooks, generated policy artifacts, browser guidance, and the implementation
  map so opaque-only output, the conservative default, hosted CI, cache policy,
  ZXing, and quirc have one consistent documented contract.
- [x] Run and record the relevant routine, independent-decode, adverse,
  coverage, mutation, deterministic-build, browser, privacy, and release
  readiness gates before resolving the ticket.

## Product intent

Correctness should be continuously enforced rather than depending on an owner
remembering to run local extended commands. The normal export uses opaque white,
Rounded ONE geometry, and the reviewed logo treatment by default; users can
disable the logo when they need ordinary ECC-M capacity without occlusion.

Removing transparent output is a deliberate simplification. It eliminates a
placement-dependent caution surface and makes the quiet zone, contrast, PNG
pixels, SVG background, logo knockout, previews, and decoder inputs share one
opaque-white contract.

## Implementation constraints

Preserve `qr-web -> qr-render -> qr-core`, with `qr-web -> qr-core` allowed.
Keep browser APIs out of `qr-core` and `qr-render`; keep all payload processing
in the browser; and do not add production network requests, telemetry, a
production QR implementation, or a general image stack. Rendering changes must
not alter encoded modules, ECC, version, or mask.

This ticket does not claim completion of the licensed ISO/IEC 18004:2024 audit;
that requires owner-provided licensed material. It also does not claim physical
camera/device validation, which remains manual owner work. The agent should
preserve the existing `public-corroborated, non-normative` labels until the
licensed audit is performed and must not substitute public implementation
agreement for normative proof.

## Comments

- Owner follow-up: Rounded ONE modules are unconditional and have no appearance
  control. The bundled logo is enabled by default and remains removable.

## Answer

Transparent output and the combinatorial appearance editor were removed. SVG
and PNG now always render deterministic 0.75-module Rounded ONE glyphs outside
standard square finders on opaque white. The bundled logo starts enabled; turning
it off restores ECC M and ordinary first-fit sizing. The approved matrix is 218
rows across five profiles and two logo states, with 145 accepted rows and 73
typed geometry rejections.

Hosted routine and extended correctness workflows now use pinned tools and
lockfile/toolchain-aware caches. Cached decoder source is reverified, including
recursive ZXing submodules. The extended job runs seeded core, PNG, rasterized
SVG, logo, adverse, and test-only pinned quirc campaigns and uploads evidence
only on failure.

Verification for commit `31e5f35` passed `pnpm verify`, the quirc campaign, the
218-row ZXing release-evidence campaign, all 29 adverse outcomes, and the clean
release-readiness gate including two reproducible builds and 20 zero-retry
Chromium tests. Coverage was run without weakening policy and reported 78.84%
line / 77.24% region against the existing 95% / 90% thresholds. The mutation
campaign was run against its 695-mutant baseline; it exposed missed BCH mutants
and repeated 20-second timeouts, so it was stopped rather than misreported as a
pass. Licensed ISO audit and physical-device validation remain owner work.
