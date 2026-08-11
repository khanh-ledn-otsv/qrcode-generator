# Repository Instructions

## Scope

These instructions apply to the entire repository.

This is a client-side QR code generator built with Rust 2024, Leptos, WebAssembly, Trunk, and Tailwind CSS. Keep payload processing and artifact generation entirely in the browser.

Before changing architecture, QR encoding, rendering, dependencies, or test policy, read:

- `docs/DEVELOPMENT_PLAN.md`
- `docs/TESTING_STRATEGY.md`

Treat those documents as authoritative. Update them when an implementation decision changes their assumptions.

## Architecture

- Preserve the planned dependency direction: `qr-web -> qr-render -> qr-core`, with `qr-web -> qr-core` also allowed.
- Keep browser APIs out of `qr-core` and `qr-render`.
- Keep encoding separate from rendering; rendering must not alter ECC, version, mask, or encoded modules.
- Treat ISO/IEC 18004:2024 as the normative QR source. Cite the relevant clause or table beside transcribed standard data.
- Use independent QR implementations only as development or test oracles. Do not copy them into production code.

## Implementation Rules

- Preserve user input exactly. Do not trim, normalize, rewrite, log, or transmit QR payloads.
- Do not add production network requests, external fonts, analytics, or telemetry.
- Prefer focused modules, typed errors, checked arithmetic, and bounds-checked access.
- Do not panic on user-controlled input. Avoid `unwrap`, `expect`, and unchecked indexing on user-facing paths.
- Do not add `unsafe` project code without an explicit documented justification.
- Keep generated SVG and PNG output deterministic; exclude timestamps, randomness, unstable ordering, and environment-dependent metadata.
- Preserve semantic HTML, keyboard operation, visible focus, and associated validation messages.
- Keep changes scoped and preserve unrelated work already present in the worktree.

## Dependencies

- Add the smallest dependency needed and separate production dependencies from test-only tooling.
- When adding or updating a dependency or development tool, try the latest stable version compatible with the project's toolchain and other constraints first. Fall back to an older version only after a concrete compatibility, build, or test failure demonstrates that the latest compatible candidate cannot be used; document the failure and selected fallback.
- Pin versions as required by the development plan; do not use wildcards or floating Git revisions.
- Do not add production QR encoders, Reed-Solomon libraries, general image stacks, SVG rasterizers, or scene renderers unless the development plan is deliberately revised.
- Keep browser-only crates and features scoped to `qr-web` after the workspace migration.

## Development Tooling

- Use the `.nvmrc`-declared Node.js v24 runtime and the
  `packageManager`-pinned pnpm version. Commit `pnpm-lock.yaml`; do not create an
  npm lockfile.
- Before running `pnpm`, Playwright, Trunk, or another Node-backed command,
  check `node --version`. A package-manager engine warning is not an acceptable
  substitute for using Node.js v24. If the active shell is not on v24, activate
  `.nvmrc` with the available version manager. In this workspace, the canonical
  non-interactive fallback is:

  ```sh
  fnm exec --using=.nvmrc node --version
  fnm exec --using=.nvmrc pnpm run verify
  ```

  The first command must report `v24.*`. When `fnm` is unavailable, use the
  installed version manager's equivalent and verify the version before
  continuing.
- Prefer repository `pnpm` scripts over spelling out their tool commands. They
  carry the pinned options and keep local and CI behavior aligned.
- Set `NO_COLOR=true` on every direct agent-run Trunk command. Prefer
  `pnpm run build` for the optimized release build and `pnpm run dev` for the
  development server; those scripts already set `NO_COLOR=true`.
- Use Oxlint for JavaScript/TypeScript linting and Oxfmt for formatting.
- Use Ruff for Python linting and formatting, and ty for Python type checking.
  Run both through the locked `tests/oracles` uv project.

## Verification

Choose verification from the behavior and dependency surface changed. Do not
run the complete gate merely because it exists, and do not rerun unrelated
checks after a focused gate succeeds. For an automatic conservative choice,
use:

```sh
pnpm run verify:changed
```

The selector inspects the working-tree paths, chooses a focused gate, and falls
back to the full gate for unknown or cross-crate changes. Use the smallest
explicit gate when the impact is already known:

```sh
pnpm run verify:core    # qr-core-only changes
pnpm run verify:render  # qr-render changes, including its WASM renderer test
pnpm run verify:web     # qr-web, HTML, CSS, and ordinary web changes
pnpm run verify:python  # tests/support Python-only changes
pnpm run verify:meta    # workflow/routing-script-only changes
```

Run the complete repository gate only when the change can affect multiple
crates or the shared build/runtime contract, including production dependency or
lockfile changes, Cargo/Trunk configuration, QR core/render behavior, WASM
boundaries, release behavior, or an unknown path:

```sh
pnpm run verify
```

Workflow documentation, CI path filters, and verification-routing script edits
do not automatically require every product test. Run `verify:meta`, exercise
the affected selector cases, and run a product gate only if those edits change
that product gate's commands or runtime assumptions. If a complete gate already
passed and subsequent edits are limited to documentation, workflow YAML, or
verification routing, validate only those later edits rather than rerunning the
complete gate.

`pnpm run verify` is the authoritative routine gate. It already runs Rust
formatting, native and WASM checks, warnings-as-errors Clippy, native/Python/WASM
tests, the optimized release build, and Chromium tests. Do not rerun those same
commands separately after a successful complete gate unless diagnosing a
failure or satisfying a more specialized release check.

When a direct Trunk build is genuinely needed, use the repository's colorless
form:

```sh
NO_COLOR=true trunk build --release
```

Keep routine Cargo commands on the default workspace `target/` layout so local
incremental compilation and the hosted Rust cache remain effective. Do not set
`CARGO_TARGET_DIR`, run `cargo clean`, or delete cached build outputs before
routine verification unless a documented release script deliberately requires
an isolated target directory.

Do not set `CI=true` for normal local verification: the optimized local gate
parallelizes independent work. Set it only when intentionally reproducing the
hosted CI execution mode, which serializes resource-heavy test lanes.

If a gate fails, preserve the failure, diagnose it, and rerun the narrowest
failing command while iterating. Run the required covering gate again after the
fix. If required tooling is unavailable, report the skipped check and reason.

Do not run release evidence, extended decoder campaigns, coverage, mutation,
fuzzing, or Miri unless the change touches the behavior those suites cover or
the user explicitly requests them. Hosted extended decoder CI is path-filtered
to core, render, artifact, oracle, and release-evidence inputs.

## Tests and Fixtures

- Add tests for behavior changes and bug fixes.
- Test public behavior and invariants; do not use the production encoder as its own correctness oracle.
- Cover exact-fit and one-over capacity boundaries, malformed input, deterministic output, and function-module protection when relevant.
- Keep fixtures synthetic and non-sensitive, with recorded provenance.
- Never regenerate golden fixtures implicitly during tests.
- Do not weaken or skip a failing test to make a change pass; fix the cause or document an intentional policy change.

## Handoff

Summarize changed behavior, list verification performed, and identify any checks that could not run.

## Agent skills

### Structural code navigation

- When exploring unfamiliar source or planning a focused edit, consider loading
  the `ast-grep-outline` skill and running `ast-grep outline` before reading entire
  files. Use its line-numbered structure to narrow subsequent source reads.
- Use `rg` for filenames and plain-text searches. When the question depends on
  syntax or relationships between code constructs, load the `ast-grep` skill
  and prefer an AST-aware `ast-grep run` or `ast-grep scan` query.
- Test non-trivial ast-grep rules against a minimal example before scanning the
  repository. Follow the skill guidance for relational rules, including
  `stopBy: end`, and use `--debug-query` when the parsed structure is unclear.
- After modifying several source files, consider `ast-grep outline` on the
  changed files to review the resulting public surface and module structure.

### Issue tracker

Issues are tracked as local Markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Domain docs

This repository uses a single-context domain documentation layout. See `docs/agents/domain.md`.
