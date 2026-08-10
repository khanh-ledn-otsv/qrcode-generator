# 34 — Complete coverage and mutation gates

**What to build:** Make release-readiness evidence account for the documented coverage and mutation policies.

**Blocked by:** none.

**Type:** task

**Priority:** deferred improvement

- [ ] Re-run coverage and identify the highest-value gaps behind the recorded
  78.84% line / 77.24% region result.
- [ ] Add focused tests until each applicable threshold in
  `docs/TESTING_STRATEGY.md` is met, or record an explicit owner-approved policy
  revision with narrow exclusions.
- [ ] Kill or prove equivalent the known surviving BCH mutations.
- [ ] Classify mutation timeouts separately from caught, missed, and unviable
  mutants without inflating the score.
- [ ] Add coverage and mutation summaries to release-readiness evidence and
  fail the release gate when the accepted policy is not met.
- [ ] Add validator tests proving missing, stale, or failing reports cannot be
  presented as a passing readiness result.

## Context

Routine verification and decoder campaigns pass, but Ticket 32 records
coverage below the strategy targets and an incomplete mutation campaign.
`scripts/release-readiness.sh` currently does not consume either result.

