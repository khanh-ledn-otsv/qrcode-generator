# 03 — Establish fixture provenance and independent QR oracles

**What to build:** Create a reproducible, development-only fixture and oracle workflow that can prove QR output without treating production code as its own correctness oracle.

**Blocked by:** 01 — Establish the offline workspace baseline.

**Prerequisite:** The owner has approved development-only QR generators and the required independent tools can be pinned for the development environment.

**Status:** ready-for-agent

- [ ] Every fixture records a synthetic payload, hashes, encoding and ECI metadata, version, ECC, mask, source tool versions, generation commands, and independent verification state.
- [ ] Explicit-version and explicit-mask fixtures are compared across two independently maintained generators before acceptance.
- [ ] Fixture regeneration is an explicit developer action and never occurs implicitly during tests.
- [ ] Golden changes produce human-reviewable matrix and metadata differences with updated provenance.
- [ ] A pinned independent decoder can inspect production raster artifacts and compare decoded text or bytes plus exposed QR metadata.
- [ ] No production crate links to or copies an oracle implementation.
