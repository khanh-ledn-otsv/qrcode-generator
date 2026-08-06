# Ticket 10 penalty-oracle disagreement

**Recorded:** 2026-08-06
**State:** quarantined; not accepted fixture evidence

Pinned Nayuki QR Code Generator 1.8.0 and python-qrcode 8.2 agree on the
completed explicit-mask matrices but expose different penalty totals for those
same matrices. For the synthetic Version 2-Q stream used during ticket 10,
mask 0 scores 1107 through Nayuki `_get_penalty_score` and 387 through
python-qrcode `lost_point`. Several other candidates also disagree, although
both select mask 0 for this case.

The difference is concentrated in finder-like Rule 3 interpretation: Nayuki's
run-history implementation and python-qrcode's literal contextual-pattern
implementation count different occurrences. A 5,000-seed search at Versions
1-M and 2-Q did not find a stream for which all eight exposed totals agree.

Under `docs/research/qr-public-source-provenance.md`, majority vote and partial
agreement cannot accept the fixture. Ticket 10 remains unresolved until a
licensed ISO/IEC 18004:2024 audit or another owner-approved evidence path
resolves the scoring interpretation. Independent ZXing-C++ artifact decoding
also remains outstanding; the pinned checkout is not present locally and the
production render/decode harness is scheduled for later tickets.
