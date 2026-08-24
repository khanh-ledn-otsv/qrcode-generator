# Verification Command Index

## Agent metadata

- **Purpose:** exact command inventory, escalation examples, and approximate
  cost for repository verification.
- **Read when:** selecting, changing, or debugging a test gate.
- **Authority:** `AGENTS.md` owns the rules; this file expands command details.
- **Do not use for:** product/QR decisions (`docs/DEVELOPMENT_PLAN.md`) or test
  design rationale (`docs/TESTING_STRATEGY.md`).

## Selection algorithm

```text
classify final changed paths
  -> select one routine gate
  -> evaluate specialized triggers
  -> run the routine gate once
  -> run only triggered specialized gates
  -> on failure, iterate narrowly and rerun the covering gate once
```

When unsure, use `pnpm run verify:changed`. Do not combine focused gates
manually; mixed surfaces route to `verify`.

## Path-to-gate map

The executable owner is `scripts/select-verification.sh`.

| Path class | Selector result | Command |
|---|---|---|
| no executable path; ordinary docs | `none` | no product gate |
| `AGENTS.md`, testing/verification policy docs | `meta` | `pnpm run verify:meta` |
| `.github/**`, selector/routing/doc-validation scripts | `meta` | `pnpm run verify:meta` |
| `tests/support/**`, `ruff.toml`, `ty.toml` | `python` | `pnpm run verify:python` |
| `crates/qr-core/**` only | `core` | `pnpm run verify:core` |
| `crates/qr-render/**` only | `render` | `pnpm run verify:render` |
| `crates/qr-web/**`, `e2e/**`, Playwright/Oxlint/Oxfmt config | `web` | `pnpm run verify:web` |
| two different non-`none` classes | `full` | `pnpm run verify` |
| dependencies, shared configuration, fixtures, assets, tools, unknown paths | `full` | `pnpm run verify` |

The agent may override `core`/`render`/`web` upward to `full` only for the
semantic escalation rules in `AGENTS.md`. Never override downward merely to
save time.

## Routine gates

Measured times are warm local observations from 2026-08-11 and are not pass/
fail thresholds.

| Command | Typical warm cost | Coverage boundary |
|---|---:|---|
| `git diff --check` | <1 s | whitespace errors only |
| `pnpm run verify:meta` | 1–3 s | selector cases, workflow action pins, documentation validation, and all shell syntax |
| `pnpm run verify:python` | 15–30 s | Python lint, format, types, unit tests |
| `pnpm run verify:core` | 15–35 s | qr-core format, Clippy, routine tests |
| `pnpm run verify:render` | 20–60 s | qr-render format, Clippy, native tests, WASM PNG test |
| `pnpm run verify:web` | 20–60 s | qr-web checks, native/WASM tests, release build, Chromium |
| `pnpm run verify` | 45–90 s | complete routine repository gate |

`verify` already includes formatting, linting, Rust native/WASM checks and
tests, Python checks/tests, one optimized build, and Chromium. A successful
`verify` makes all separate routine reruns redundant.

## Specialized gates

| Command | Observed/declared cost | Scope |
|---|---:|---|
| `pnpm run test:approved` | about 90 s | ordinary plus exhaustive 436-row approved/resource matrix |
| `pnpm run test:decode` | about 5 min warm | complete ZXing PNG/SVG/logo decode campaign |
| `pnpm run test:quirc` | variable, usually <1 min warm | representative secondary ASCII decode |
| `pnpm run test:adverse:decode` | about 6 s warm | ignored adverse-transform decode evidence |
| `pnpm run release:evidence` | about 6–8 min warm | build, approved matrix, all decoder evidence, hashes |
| `bash scripts/release-evidence.sh --dist dist` | about 6–8 min warm | same evidence, reuse verified `dist` |
| `pnpm run release:coverage` | multi-minute | enforced coverage thresholds |
| `pnpm run release:mutation` | campaign | mutation thresholds |
| `pnpm run release:fuzz` | at least 80 min declared | ten minutes per target |
| `pnpm run release:fuzz:deep` | at least 4 h declared | one hour per critical target |
| `pnpm run release:miri` | campaign | selected core/render interpreter checks |
| `pnpm run release:readiness` | multi-minute, clean only | final reproducible-build/release contract |

Consult the specialized trigger table in `AGENTS.md` before running any row in
this table.

## Examples

| Final change | Run | Do not run |
|---|---|---|
| typo in README | `git diff --check` | Cargo, browser, release suites |
| verification selector case | `pnpm run verify:meta` | product gates unless commands changed |
| one qr-core error variant | `pnpm run verify:core` | render/web gates |
| qr-core encoding bits or shared encoded fixture | `pnpm run verify` | separate focused gates after full passes |
| one qr-render geometry error with stable artifact bytes | `pnpm run verify:render` | browser/full evidence |
| approved profile/logo/artifact contract | `pnpm run verify` then `pnpm run test:approved` | full decoder evidence unless decoder/artifact pipeline changed |
| Leptos state or e2e assertion | `pnpm run verify:web` | separate build/e2e rerun |
| Cargo dependency update | `pnpm run verify`; audit only for release/security work | every specialized suite |
| release-evidence existing-dist behavior | `pnpm run verify:meta`, affected narrow test, then `bash scripts/release-evidence.sh --dist dist` | release readiness unless clean end-to-end readiness changed |

## Failure handling

1. Record the exact failing subcommand/test.
2. Rerun only that subcommand or test while fixing.
3. Do not clear `target/`, dependency caches, fixtures, or goldens.
4. Rerun the originally required covering gate once after the fix.
5. Stop after success unless a specialized trigger remains.

## Updating this index

When a command changes:

1. update `package.json` and its script implementation;
2. update `scripts/select-verification.sh` if routing changes;
3. add selector cases in `scripts/test-select-verification.sh`;
4. update `AGENTS.md` and this file;
5. run `pnpm run verify:meta`;
6. run the affected product gate only if command coverage/runtime assumptions
   changed.
