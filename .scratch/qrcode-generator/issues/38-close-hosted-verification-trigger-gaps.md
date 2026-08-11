# 38 — Close hosted verification trigger gaps

**What to build:** Ensure the hosted correctness workflow runs before merge
when appropriate and cannot skip repository verification-policy changes.

**Blocked by:** none.

**Type:** task

**Priority:** focused CI correctness improvement

- [ ] Add `AGENTS.md`, `README.md`, and `docs/**` to the Correctness workflow's
  path trigger so policy and authoritative-link changes reach the existing
  selector.
- [ ] Route policy documents through `verify:meta` and keep ordinary prose-only
  validation in the cheapest covering lane.
- [ ] Add selector/workflow tests proving policy-only pushes cannot bypass the
  hosted metadata gate.
- [ ] Confirm whether this repository uses pull requests. If it does, add a
  `pull_request` trigger that computes a safe merge-base diff and runs the same
  conservative selector; if it does not, record the direct-to-main decision
  and do not add an unused trigger.
- [ ] Preserve main-only Pages deployment and prevent pull-request runs from
  receiving deployment permissions or publishing artifacts.

## Context

The local path selector maps `AGENTS.md`, `docs/TESTING_STRATEGY.md`, and
`docs/agents/verification.md` to `verify:meta`, but the hosted workflow's path
filter does not include those files, so a policy-only push starts no job. The
workflow also runs only on pushes to `main` and manual dispatch. That is safe
for an explicitly direct-to-main repository, but it detects failures after
merge if pull requests are part of the development workflow.
