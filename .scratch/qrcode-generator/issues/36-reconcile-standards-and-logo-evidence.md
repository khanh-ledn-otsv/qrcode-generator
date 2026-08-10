# 36 — Reconcile standards and logo evidence

**What to build:** Make production citations and adaptive-logo documentation point only to retained, auditable evidence.

**Blocked by:** none.

**Type:** task

**Priority:** deferred improvement

- [ ] Audit exact ISO/IEC 18004:2024 clause references in production comments.
- [ ] For mappings not verified against licensed text, add the required
  `2024 clause mapping pending audit` label while retaining public-source and
  fixture provenance.
- [ ] Resolve the missing
  `docs/generated/adaptive-branded-placement-policy.json` reference in the
  authoritative development plan.
- [ ] Either restore machine-readable adaptive placement campaign evidence
  with provenance and hashes, or revise the plan to cite the retained evidence
  and remove unsupported outcome counts.
- [ ] Add a documentation/link check that catches missing committed evidence
  artifacts referenced by authoritative plans.

## Context

The current behavior is extensively tested, but several production comments
state exact 2024 clauses without the plan's pending-audit qualifier, and the
plan links to an adaptive placement experiment artifact that is no longer in
the repository.

