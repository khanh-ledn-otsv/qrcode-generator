#!/usr/bin/env python3
"""Generate or verify the non-normative QR Model 2 table fixture."""

from __future__ import annotations

import argparse
import csv
import importlib.metadata
import io
import pathlib


QRCODEGEN_VERSION = "1.8.0"
PYTHON_QRCODE_VERSION = "8.2"


def require_version(distribution: str, expected: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != expected:
        raise RuntimeError(f"expected {distribution} {expected}, got {actual}")


def normalized_groups(blocks: list[tuple[int, int]]) -> list[tuple[int, int, int]]:
    counts: dict[tuple[int, int], int] = {}
    for total, data in blocks:
        shape = (total, data)
        counts[shape] = counts.get(shape, 0) + 1
    return [
        (counts[shape], shape[0], shape[1])
        for shape in sorted(counts, key=lambda item: item[1])
    ]


def render_fixture() -> str:
    require_version("qrcodegen", QRCODEGEN_VERSION)
    require_version("qrcode", PYTHON_QRCODE_VERSION)

    import qrcode.base
    import qrcode.constants
    import qrcode.util
    from qrcodegen import QrCode, QrSegment

    levels = [
        ("L", QrCode.Ecc.LOW, qrcode.constants.ERROR_CORRECT_L),
        ("M", QrCode.Ecc.MEDIUM, qrcode.constants.ERROR_CORRECT_M),
        ("Q", QrCode.Ecc.QUARTILE, qrcode.constants.ERROR_CORRECT_Q),
        ("H", QrCode.Ecc.HIGH, qrcode.constants.ERROR_CORRECT_H),
    ]
    modes = [
        (QrSegment.Mode.NUMERIC, qrcode.util.MODE_NUMBER),
        (QrSegment.Mode.ALPHANUMERIC, qrcode.util.MODE_ALPHA_NUM),
        (QrSegment.Mode.BYTE, qrcode.util.MODE_8BIT_BYTE),
        (QrSegment.Mode.KANJI, qrcode.util.MODE_KANJI),
    ]
    output = io.StringIO()
    output.write(
        "# Non-normative QR Model 2 table fixture.\n"
        f"# Oracles: qrcodegen=={QRCODEGEN_VERSION}, qrcode=={PYTHON_QRCODE_VERSION}.\n"
        "# Regenerate only with: uv run --project tests/oracles --locked python "
        "tests/support/verify_qr_tables.py --write\n"
    )
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(
        [
            "version",
            "ecc",
            "total_codewords",
            "data_codewords",
            "ecc_codewords_per_block",
            "group1_blocks",
            "group1_data_codewords",
            "group2_blocks",
            "group2_data_codewords",
            "remainder_bits",
            "alignment_centers",
            "numeric_count_bits",
            "alphanumeric_count_bits",
            "byte_count_bits",
            "kanji_count_bits",
        ]
    )

    for version in range(1, 41):
        raw_modules = QrCode._get_num_raw_data_modules(version)
        total_codewords = raw_modules // 8
        remainder_bits = raw_modules % 8
        generated = QrCode.encode_segments(
            [], QrCode.Ecc.LOW, version, version, 0, False
        )
        nayuki_centers = generated._get_alignment_pattern_positions()
        python_centers = qrcode.util.pattern_position(version)
        if nayuki_centers != python_centers:
            raise ValueError(f"version {version} alignment oracle disagreement")

        count_widths = []
        for nayuki_mode, python_mode in modes:
            nayuki_width = nayuki_mode.num_char_count_bits(version)
            python_width = qrcode.util.length_in_bits(python_mode, version)
            if nayuki_width != python_width:
                raise ValueError(f"version {version} character-width disagreement")
            count_widths.append(nayuki_width)

        for letter, nayuki_ecc, python_ecc in levels:
            python_blocks = qrcode.base.rs_blocks(version, python_ecc)
            python_pairs = [
                (block.total_count, block.data_count) for block in python_blocks
            ]
            groups = normalized_groups(python_pairs)
            if len(groups) not in (1, 2):
                raise ValueError(f"version {version}-{letter} has {len(groups)} groups")

            block_count = QrCode._NUM_ERROR_CORRECTION_BLOCKS[
                nayuki_ecc.ordinal
            ][version]
            ecc_per_block = QrCode._ECC_CODEWORDS_PER_BLOCK[
                nayuki_ecc.ordinal
            ][version]
            data_codewords = total_codewords - block_count * ecc_per_block
            nayuki_short_total = total_codewords // block_count
            nayuki_long_count = total_codewords % block_count
            nayuki_groups = [
                (
                    block_count - nayuki_long_count,
                    nayuki_short_total,
                    nayuki_short_total - ecc_per_block,
                )
            ]
            if nayuki_long_count:
                nayuki_groups.append(
                    (
                        nayuki_long_count,
                        nayuki_short_total + 1,
                        nayuki_short_total + 1 - ecc_per_block,
                    )
                )
            if groups != nayuki_groups:
                raise ValueError(f"version {version}-{letter} block oracle disagreement")
            if sum(total for total, _data in python_pairs) != total_codewords:
                raise ValueError(f"version {version}-{letter} total disagreement")
            if sum(data for _total, data in python_pairs) != data_codewords:
                raise ValueError(f"version {version}-{letter} data disagreement")

            second = groups[1] if len(groups) == 2 else (0, 0, 0)
            writer.writerow(
                [
                    version,
                    letter,
                    total_codewords,
                    data_codewords,
                    ecc_per_block,
                    groups[0][0],
                    groups[0][2],
                    second[0],
                    second[2],
                    remainder_bits,
                    ";".join(str(center) for center in nayuki_centers),
                    *count_widths,
                ]
            )
    return output.getvalue()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--write", action="store_true")
    args = parser.parse_args()
    fixture_path = pathlib.Path("tests/fixtures/qr_tables.csv")
    generated = render_fixture()
    if args.check:
        if fixture_path.read_text(encoding="ascii") != generated:
            raise ValueError(f"{fixture_path} does not match the pinned oracles")
    else:
        fixture_path.write_text(generated, encoding="ascii")


if __name__ == "__main__":
    main()
