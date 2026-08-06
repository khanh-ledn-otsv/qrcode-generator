# Public-source provenance for QR Model 2 implementation

**Recorded:** 2026-08-06  
**Scope:** QR Model 2 encoding rules needed by tickets 06–10 when a complete licensed ISO/IEC 18004:2024 copy is unavailable.

## Authority and limits

[ISO/IEC 18004:2024](https://www.iso.org/standard/83389.html) is the normative authority. ISO's public catalogue confirms that the standard covers symbol formats, data encoding, dimensional characteristics, error-correction rules, reference decoding, production quality, and user-selectable parameters, but it does not publish the complete requirements text.

The sources below therefore corroborate an implementation; they do not become a substitute normative standard. Production code must be written independently, not copied or mechanically translated from an oracle. Evidence derived through this policy is labelled `public-corroborated, non-normative` until audited against a complete licensed 2024 edition.

## Public first-party evidence

### QR Code steward material

- DENSO WAVE's [error-correction overview](https://www.qrcode.com/en/about/error_correction.html) confirms four selectable levels, Reed–Solomon error correction, the capacity/robustness tradeoff, and identifies Level M as the typical general choice. This supports the release-1 safe-workflow ECC M decision, not detailed field arithmetic.
- DENSO WAVE's [version overview](https://www.qrcode.com/en/about/version.html/index.html) confirms Versions 1–40, the `21 + 4 × (version - 1)` side-length progression, and that capacity depends on data type and error-correction level.
- DENSO WAVE's [FAQ](https://www.qrcode.com/en/faq.html) states that ordinary QR Code use follows JIS/ISO, calls for a four-module surrounding margin in its practical example, and warns that colors, illustrations, transparent media, printers, and readers require real-environment testing.

These pages are authoritative high-level product guidance from the QR Code owner, but they do not expose all bit-level encoding rules.

### Pinned encoder oracle A: Nayuki QR Code Generator 1.8.0

Use the tagged Rust source [`rust/src/lib.rs`](https://github.com/nayuki/QR-Code-generator/blob/v1.8.0/rust/src/lib.rs), recording these symbols in fixture metadata:

- `add_ecc_and_interleave`, `reed_solomon_compute_divisor`, `reed_solomon_compute_remainder`, and `reed_solomon_multiply` for GF(256), Reed–Solomon, block splitting, and interleaving;
- `draw_function_patterns`, `draw_finder_pattern`, `draw_alignment_pattern`, and `set_function_module` for function regions;
- `draw_codewords` and `apply_mask` for zig-zag placement, remainder behavior, function protection, and the eight masks;
- `draw_format_bits` and `draw_version` for BCH construction and placement;
- `get_penalty_score` and `FinderPenalty` for automatic-mask scoring.

The repository is MIT-licensed and explicitly supports all 40 Model 2 versions and all four ECC levels. Its code is an oracle only.

Executable fixtures may use the pinned `qrcodegen==1.8.0` Python distribution,
which is the same release's official language port, from tagged
[`python/qrcodegen.py`](https://github.com/nayuki/QR-Code-generator/blob/v1.8.0/python/qrcodegen.py).
Fixture metadata must distinguish that executed source and its underscore-prefixed
`_reed_solomon_*` symbols from the Rust source/symbols above that provide the
recorded public-source evidence. Both must remain pinned to release 1.8.0.

### Pinned encoder oracle B: python-qrcode 8.2

Use the tagged sources, recording exact file and symbol:

- [`qrcode/base.py`](https://github.com/lincolnloop/python-qrcode/blob/v8.2/qrcode/base.py): `Polynomial` and `RS_BLOCK_TABLE` for independent Reed–Solomon/block-layout results;
- [`qrcode/main.py`](https://github.com/lincolnloop/python-qrcode/blob/v8.2/qrcode/main.py): `makeImpl`, the `setup_*` functions, `map_data`, and `best_mask_pattern` for function patterns, BCH placement, data placement, and mask selection;
- [`qrcode/util.py`](https://github.com/lincolnloop/python-qrcode/blob/v8.2/qrcode/util.py): `BCH_type_info`, `BCH_type_number`, `mask_func`, and `lost_point` for BCH, masks, and the four penalty rules.

This project is separately maintained from Nayuki. Agreement is useful corroboration, not proof of standards conformance; shared historical ancestry or the same mistake remains possible.

### Independent decode oracle: ZXing-C++ 2.3.0

Use the tagged decoder sources [`QRBitMatrixParser.cpp`](https://github.com/zxing-cpp/zxing-cpp/blob/v2.3.0/core/src/qrcode/QRBitMatrixParser.cpp), [`QRDataBlock.cpp`](https://github.com/zxing-cpp/zxing-cpp/blob/v2.3.0/core/src/qrcode/QRDataBlock.cpp), and [`QRDecoder.cpp`](https://github.com/zxing-cpp/zxing-cpp/blob/v2.3.0/core/src/qrcode/QRDecoder.cpp) to contextualize parser, de-interleaving, and Reed–Solomon decode behavior. The project is Apache-2.0 licensed and is used as the pinned artifact decoder, not as an encoder implementation source.

Successful decode cannot prove the exact required matrix: many valid encodings and some damaged symbols decode. Exact explicit-mask matrices and structural invariants remain mandatory.

## Acceptance procedure for tickets 06–10

For each rule or fixture:

1. Record the intended ISO/IEC 18004:2024 clause/table topic. If its exact 2024 number has not been checked against a complete edition, say `2024 clause mapping pending audit`.
2. Pin the two encoder versions above and record their exact executed source
   files/symbols, public-evidence source files/symbols, and generation commands.
3. Generate synthetic, non-sensitive results independently. Accept only exact agreement for behavior both expose.
4. Add a locally written slow reference or invariant where practical: field cycles/inverses, block totals and de-interleaving, single ownership of every module, complete placement, BCH remainder properties, and isolated penalty truth tables.
5. Compare explicit version/ECC/mask matrices before automatic mask-selection results.
6. Decode representative completed artifacts with pinned ZXing-C++ and compare exact decoded bytes/text plus exposed metadata.
7. If any public source, fixture, invariant, or decoder disagrees, quarantine the fixture and investigate. Do not choose by majority vote and do not weaken the failing gate.
8. Label accepted evidence `public-corroborated, non-normative`. A later licensed-standard audit records any difference as an explicit reviewed migration.

## Independence caveats

- Nayuki and `python-qrcode` are separate projects, but repository independence does not guarantee independent interpretation of the standard.
- ZXing-C++ exercises the reverse path and is implementation-diverse, but decode success is a weaker assertion than exact encoding agreement.
- Locally authored references must use a deliberately simple formulation and must not call production helpers; otherwise production would become its own oracle.
- No public implementation is linked into `qr-core`, `qr-render`, or `qr-web`, and no upstream implementation body is copied into production.
