# 36 — Reconcile standards and logo evidence

**What to build:** Make production citations and adaptive-logo documentation point only to retained, auditable evidence.

**Blocked by:** none.

**Type:** task

**Priority:** deferred improvement

**Status:** resolved

- [x] Audit exact ISO/IEC 18004:2024 clause references in production comments.
- [x] For mappings not verified against licensed text, add the required
  `2024 clause mapping pending audit` label while retaining public-source and
  fixture provenance.
- [x] Resolve the missing
  `docs/generated/adaptive-branded-placement-policy.json` reference in the
  authoritative development plan.
- [x] Either restore machine-readable adaptive placement campaign evidence
  with provenance and hashes, or revise the plan to cite the retained evidence
  and remove unsupported outcome counts.
- [x] Add a documentation/link check that catches missing committed evidence
  artifacts referenced by authoritative plans.

## Context

The adaptive placement documentation now cites retained executable geometry
and decoder tests plus `docs/generated/logo-placement-policy.md`; unsupported
campaign counts and the missing JSON reference are gone. The repository-owned
documentation validator recursively checks authoritative Markdown links and
therefore rejects missing committed evidence references.

The remaining work is only the standards citation audit. Several production
comments still state exact 2024 clauses without the plan's pending-audit
qualifier. Add the qualifier now wherever licensed verification is not
recorded. Final confirmation or correction of exact clause mappings is blocked
on owner-provided licensed ISO/IEC 18004:2024 text; public-source agreement must
not be substituted for that audit.

## Answer

Audited every ISO/IEC 18004:2024 reference in production Rust comments. Exact
clause and annex references for version sizing, data encoding, QR tables, and
the four-module quiet zone now carry the required
`2024 clause mapping pending audit` label. Existing public-source identities,
fixture paths, and non-normative provenance remain intact. Comments that
already carried the qualifier were retained, and the Reed–Solomon and
interleaving module wording was made consistent with the required label.

No licensed standard text was available, so this resolves the repository
provenance-labeling task without claiming that the cited 2024 clause numbers
have been confirmed. A later licensed-text audit may confirm or correct those
mappings as a separate reviewed change.
