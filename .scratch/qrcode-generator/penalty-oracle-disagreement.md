# Ticket 10 penalty-oracle disagreement

**Recorded:** 2026-08-06
**State:** resolved and accepted under recorded owner interpretation

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

On 2026-08-06 the owner approved literal complete-matrix matching: count only
`00001011101` and `10111010000` windows wholly inside rows and columns, without
virtual quiet-zone padding. python-qrcode 8.2 and the independent slow
reference agree on that interpretation. Nayuki's differing totals remain
recorded as a named exception; they are not treated as a vote or hidden.

The manifest-pinned ZXing-C++ 3.0.2 checkout independently decoded exact bytes,
version, ECC and ECI-presence metadata for 132 seeded safe-style artifacts plus
Versions 2, 7, 10, 27 and 40 boundary representatives. The suite covers every
ECC level and every automatically selected mask, completing the retained
acceptance gate.
