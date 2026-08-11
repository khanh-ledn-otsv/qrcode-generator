# 34 — Complete coverage and mutation gates

**What to build:** Make release-readiness evidence account for the documented coverage and mutation policies.

**Blocked by:** none.

**Type:** task

**Priority:** deferred improvement

- [x] Re-run coverage and identify the highest-value gaps behind the original
  78.84% line / 77.24% region result.
- [x] Add focused tests until each applicable threshold in
  `docs/TESTING_STRATEGY.md` is met, or record an explicit owner-approved policy
  revision with narrow exclusions.
- [ ] Kill or prove equivalent the known surviving BCH mutations.
- [x] Classify mutation timeouts separately from caught, missed, and unviable
  mutants without inflating the score.
- [ ] Add coverage and mutation summaries to release-readiness evidence and
  fail the release gate when the accepted policy is not met.
- [ ] Add validator tests proving missing, stale, or failing reports cannot be
  presented as a passing readiness result.

## Context

Ticket 37 reran the complete coverage and mutation gates without weakening the
policy. Current evidence passes every documented threshold: qr-core coverage is
97.16% line / 94.14% region, critical core is 98.14% / 95.85%, qr-render is
95.86% / 91.98%, render geometry is 99.66% / 96.68%, and plain-Rust web state
is 87.97% / 84.64%. Mutation scores are 95.32% for qr-core, 95.59% for critical
core, and 90.91% for render geometry. Timeout outcomes are kept out of caught
and missed scores and require explicit triage.

Three original BCH operator mutations still survive. In addition,
`scripts/release-readiness.sh` does not consume coverage or mutation summaries,
and the threshold evidence does not yet bind itself to a release-candidate
commit. The remaining work is therefore BCH triage plus fresh, commit-bound
summary collection and readiness validation; it is not another general
coverage push.
