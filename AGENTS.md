# Agent Execution Contract

## Scope and precedence

This file applies to the entire repository and is the primary agent router.

Use this precedence order:

1. User request.
2. This file.
3. The task-specific authoritative document named in the documentation router.
4. Existing code and tests.

If two repository documents conflict, stop and report the conflict. Do not
silently choose one. When implementation changes an accepted decision, update
the authoritative document in the same change.

## Product invariants: never violate

- Keep all payload processing and artifact generation in the browser.
- Preserve exact user input. Never trim, normalize, rewrite, log, or transmit a
  QR payload.
- Add no production network request, external font, analytics, or telemetry.
- Preserve dependency direction: `qr-web -> qr-render -> qr-core`; direct
  `qr-web -> qr-core` is allowed. Browser APIs belong only in `qr-web`.
- Keep encoding separate from rendering. Rendering must not change ECC,
  version, mask, or encoded modules.
- Treat ISO/IEC 18004:2024 as normative. Cite the applicable clause/table topic
  beside transcribed standard data. Oracle implementations are test evidence,
  never production code or copy sources.
- Preserve deterministic SVG/PNG bytes. Exclude timestamps, randomness,
  unstable ordering, and environment-dependent metadata.
- Do not panic on user-controlled input. Prefer typed errors, checked
  arithmetic, bounds-checked access, and focused modules. Do not add project
  `unsafe` without an explicit documented justification.
- Preserve semantic HTML, keyboard operation, visible focus, and associated
  validation messages.
- Preserve unrelated work in the worktree.

## Documentation router

Read only the rows activated by the task. Do not load every document by
default.

| Task changes or questions | Read before acting | Authority |
|---|---|---|
| architecture, dependency direction, QR behavior, profiles, product policy | `docs/DEVELOPMENT_PLAN.md` | accepted product and architecture decisions |
| test design, fixtures, or verification policy | `docs/agents/verification.md`, then relevant section of `docs/TESTING_STRATEGY.md` | executable test routing, then test-design rationale |
| release evidence, decoders, coverage, mutation, fuzzing, Miri | `docs/RELEASE_HARDENING.md` | specialized-gate triggers and commands |
| final release certification | `docs/RELEASE_READINESS.md` | clean-worktree release gate |
| oracle provenance or QR public-source evidence | `docs/research/qr-public-source-provenance.md`, `tests/oracles/README.md` | source/oracle policy |
| bundled logo asset or placement | `assets/README.md`, `docs/generated/logo-placement-policy.md` | asset provenance and generated placement contract |
| issue/spec work under `.scratch/` | `docs/agents/issue-tracker.md` | local issue workflow |
| domain terms or ADR conflict | `docs/agents/domain.md`; then `CONTEXT.md`/`docs/adr/` if present | domain vocabulary and decisions |

`docs/generated/*` is generated evidence. Modify its generator, then regenerate;
never hand-edit generated output.

## Agent workflow

1. Inspect `git status --short`. Treat existing changes as user-owned.
2. Read the minimum authoritative documentation selected above.
3. Inspect the smallest relevant source/test surface. Use `rg` for text and
   filenames; use the structural-navigation skills when syntax relationships
   matter.
4. Implement the smallest coherent change. Add or update tests for behavioral
   changes and bug fixes.
5. Select exactly one routine covering gate from the verification router.
6. Add a specialized gate only when its trigger table applies.
7. On failure, preserve the failure, iterate with the narrowest failing command,
   then rerun the required covering gate once.
8. Report behavior changed, checks run, checks skipped, and reasons.

## Verification: optimization objective

Minimize feedback time without reducing coverage of the changed behavior.

- Run the smallest gate that covers the final change, not the largest available
  gate.
- Do not run multiple routine gates “for confidence.” If `verify` passes, do not
  rerun its component commands.
- Do not rerun a passed gate after documentation-only edits. Validate only the
  later documentation or routing edits.
- Never run a slow specialized suite merely because it exists.
- User requests for a named suite, full release validation, or exhaustive
  testing override the default cost controls.

### Cost classes

Use cost as a guardrail, not as a substitute for coverage.

| Class | Expected warm duration | Default agent policy |
|---|---:|---|
| A: instant | under 10 seconds | run when relevant |
| B: routine | 10–90 seconds | run one covering gate after final edits |
| C: extended | 1–8 minutes | run only when a specialized trigger applies |
| D: campaign | over 8 minutes | run only when explicitly requested, during release work, or when the changed mechanism cannot be validated more narrowly |

Durations are approximate. A cold toolchain/cache may be slower without
changing a command's class.

### Routine path router

Prefer an explicit gate when the impact is known. Otherwise run
`pnpm run verify:changed`; it applies the same path routing conservatively.

| Final changed surface | Required routine check | Class |
|---|---|---:|
| prose-only `*.md`, no command/path/policy change | `git diff --check`; validate edited links/paths | A |
| AGENTS/test-policy documentation | `pnpm run verify:meta` | A |
| `.github/**`, verification selectors, or shell routing only | `pnpm run verify:meta` plus affected selector cases | A |
| `tests/support/**`, Ruff/ty config only | `pnpm run verify:python` | B |
| isolated `qr-core` source/tests | `pnpm run verify:core` | B |
| isolated `qr-render` source/tests | `pnpm run verify:render` | B |
| isolated `qr-web`, Astro config/pages, HTML, CSS, e2e, Playwright config | `pnpm run verify:web` | B |
| package-script-only change | run the changed script's narrow test plus `pnpm run verify:meta` | A/B |
| production dependency/lockfile, workspace Cargo config, Trunk config, rust toolchain, shared fixture contract, WASM boundary, multiple product crates, or unknown path | `pnpm run verify` | B |

Escalate an otherwise isolated change to `pnpm run verify` when it changes QR
encoding semantics, public cross-crate types, deterministic artifact bytes,
shared fixtures, or native/WASM equivalence.

### Routine command coverage

| Command | Included coverage | Do not add separately |
|---|---|---|
| `pnpm run verify:meta` | selector/doc-validation, workflow action pins and CI build contract, documentation links/metadata, and shell syntax | product tests unless changed commands/runtime require them |
| `pnpm run verify:python` | Ruff, Ruff format, ty, Python unit tests | separate Python lint/test commands |
| `pnpm run verify:core` | Rust format, qr-core Clippy, all routine qr-core tests | workspace/web/render checks |
| `pnpm run verify:render` | Rust format, qr-render Clippy/native tests, WASM PNG renderer test | full web/browser gate |
| `pnpm run verify:web` | Rust format, web lint/format, qr-web Clippy/native/WASM tests, release build, Chromium | separate build or e2e rerun |
| `pnpm run verify` | all routine formatting, linting, native/Python/WASM tests, release build, Chromium | any routine component rerun |

### Specialized trigger router

These commands are additional to one routine gate, not substitutes for it.

| Changed behavior | Additional command | Class | Run rule |
|---|---|---:|---|
| approved profile/logo matrix, resource ceilings, approved-combination generator | `pnpm run test:approved` | C | required |
| PNG/SVG bytes, rasterization, logo decode geometry, decoder adapter, artifact-evidence collector | narrow affected ignored decoder test; use `pnpm run release:evidence` only if the complete evidence pipeline changed | C | required narrow check; full evidence only when pipeline-wide |
| `scripts/release-evidence.sh` reuse/input/output behavior | `bash scripts/release-evidence.sh --dist dist` after a verified build | C | required if existing-dist behavior or evidence hashes changed |
| `scripts/release-readiness.sh` or readiness collectors/validators | narrow script/collector tests first; `pnpm run release:readiness` only from a clean worktree | C/D | full gate only when clean and the end-to-end contract changed |
| dependency versions or audit policy | `pnpm run release:dependencies` | C | release/security task or explicit request; routine dependency changes still require `verify` |
| coverage script/threshold/exclusion | `pnpm run release:coverage` | C/D | required for coverage-policy changes |
| mutation config or critical core/geometry semantics | `pnpm run release:mutation` | D | explicit mutation/release work only |
| fuzz target/parser or bug found by fuzzing | single affected `cargo fuzz run ...` with a bounded diagnostic budget | D | targeted only; full campaigns explicit/release only |
| `unsafe`, memory-layout assumptions, or Miri policy | narrow `cargo +nightly miri test ...` | D | required when applicable; full Miri suite explicit/release only |
| final release certification | `pnpm run release:readiness` | D | explicit release request only; requires clean worktree |

Do not run `test:approved`, decoder campaigns, release evidence, readiness,
coverage, mutation, fuzzing, or Miri for unrelated changes. Hosted extended
decoder CI remains the normal exhaustive backstop for matching core/render/
artifact/oracle paths.

### Failure loop

- Capture the first failing command and preserve its output.
- Diagnose with the narrowest command or single test target.
- Never weaken, ignore, delete, or regenerate a failing test to obtain green.
- After the fix, rerun the original required covering gate once.
- If the failure is environmental, verify the environment, report the exact
  skipped check, and do not substitute an unrelated suite.

## Tool and cache rules

- Use the globally installed `find-docs` skill (Context7) whenever work depends
  on current library, framework, SDK, API, CLI, or cloud-service documentation,
  including version-specific syntax, configuration, setup, migration, and
  library-specific debugging. Prefer Context7 over generic web search for that
  material. Repository-authoritative documents and product invariants still
  take precedence, and Context7 queries must not contain payloads, credentials,
  proprietary code, or other sensitive information.
- Before Node-backed commands, run `node --version`; it must be `v24.*`. Use
  `fnm exec --using=.nvmrc <command>` when the active shell is not Node 24.
- Use the `packageManager`-pinned pnpm. Commit `pnpm-lock.yaml`; never create an
  npm lockfile.
- Prefer repository `pnpm` scripts. They own pinned flags and orchestration.
- Prefer the repository's Astro-owned `pnpm run build` and `pnpm run dev`
  scripts for web builds and local development. `pnpm run build:astro` is a
  CI-only repackaging helper and assumes the current job already generated the
  WASM package.
- Use Oxlint/Oxfmt for JS/TS and Ruff/ty through the locked `tests/oracles` uv
  project for Python.
- Local sccache is optional: `RUSTC_WRAPPER=sccache`. CI owns the pinned shared
  backend and statistics.
- Keep Cargo on the default workspace `target/`. Do not set
  `CARGO_TARGET_DIR`, run `cargo clean`, or delete build caches during routine
  work. Only release-readiness isolation may override the target directory.
- Do not set `CI=true` locally unless reproducing hosted serialization.

## Dependencies

- Add the smallest dependency and keep test-only tooling out of production
  dependencies.
- Try the latest stable compatible version first. Use an older version only
  after a concrete build/test incompatibility; record the failure and fallback.
- Pin versions; no wildcards or floating Git revisions.
- Do not add a production QR encoder, Reed–Solomon library, general image stack,
  SVG rasterizer, or scene renderer unless `docs/DEVELOPMENT_PLAN.md` is
  deliberately revised.
- Keep browser-only crates/features scoped to `qr-web`.

## Tests and fixtures

- Test public behavior and invariants. The production encoder is never its own
  correctness oracle.
- Cover exact-fit and one-over capacity, malformed input, deterministic output,
  and function-module protection when relevant.
- Keep fixtures synthetic, non-sensitive, and provenance-recorded.
- Never regenerate goldens implicitly. Explicit generation must leave reviewable
  changes and follow `tests/oracles/README.md`.

## Structural navigation

- Use `rg`/`rg --files` for plain text and filenames.
- For an unfamiliar code surface, consider the `ast-grep-outline` skill before
  reading whole files.
- For syntax/relationship queries, load the `ast-grep` skill. Test non-trivial
  rules on a minimal example; use `stopBy: end` and `--debug-query` when needed.

## Handoff

Always report:

- changed behavior;
- verification commands and results;
- specialized checks intentionally not run and why;
- any environmental limitation;
- generated/deleted material and recoverability when applicable.
