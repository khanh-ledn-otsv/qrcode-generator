#!/usr/bin/env python3
"""Verify interleaved QR codeword fixtures with both pinned encoders."""

from __future__ import annotations

import argparse
import importlib.metadata
import pathlib


FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "interleaved_codewords.csv"
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_interleaved_codewords.py --check"
)
CASES = (
    (1, "M", "one-group"),
    (5, "Q", "two-group-short-long"),
)


def require_pin(distribution: str, version: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != version:
        raise RuntimeError(f"expected {distribution} {version}, got {actual}")


def oracle_streams(version: int, ecc_name: str, data: bytes) -> tuple[bytes, bytes]:
    from qrcodegen import QrCode
    import qrcode.base
    import qrcode.constants
    import qrcode.util

    nayuki_ecc = {
        "L": QrCode.Ecc.LOW,
        "M": QrCode.Ecc.MEDIUM,
        "Q": QrCode.Ecc.QUARTILE,
        "H": QrCode.Ecc.HIGH,
    }[ecc_name]
    python_ecc = {
        "L": qrcode.constants.ERROR_CORRECT_L,
        "M": qrcode.constants.ERROR_CORRECT_M,
        "Q": qrcode.constants.ERROR_CORRECT_Q,
        "H": qrcode.constants.ERROR_CORRECT_H,
    }[ecc_name]

    nayuki = QrCode(version, nayuki_ecc, bytearray(len(data)), 0)
    nayuki_stream = bytes(nayuki._add_ecc_and_interleave(bytearray(data)))
    buffer = qrcode.util.BitBuffer()
    buffer.buffer = list(data)
    buffer.length = len(data) * 8
    python_stream = bytes(
        qrcode.util.create_bytes(buffer, qrcode.base.rs_blocks(version, python_ecc))
    )
    return nayuki_stream, python_stream


def render_fixture() -> str:
    require_pin("qrcodegen", "1.8.0")
    require_pin("qrcode", "8.2")
    from qrcodegen import QrCode
    import qrcode.base
    import qrcode.constants

    lines = [
        "# public-corroborated, non-normative; ISO/IEC 18004:2024 clause mapping pending audit",
        "# Nayuki QR Code Generator 1.8.0 _add_ecc_and_interleave; python-qrcode 8.2 create_bytes",
        f"# Command: {COMMAND}",
        "version,ecc,data_hex,interleaved_hex,remainder_bits,case",
    ]
    python_levels = {
        "L": qrcode.constants.ERROR_CORRECT_L,
        "M": qrcode.constants.ERROR_CORRECT_M,
        "Q": qrcode.constants.ERROR_CORRECT_Q,
        "H": qrcode.constants.ERROR_CORRECT_H,
    }
    for version, ecc_name, label in CASES:
        blocks = qrcode.base.rs_blocks(version, python_levels[ecc_name])
        data = bytes(range(sum(block.data_count for block in blocks)))
        nayuki_stream, python_stream = oracle_streams(version, ecc_name, data)
        if nayuki_stream != python_stream:
            raise ValueError(f"Version {version}-{ecc_name} interleaving disagreement")
        remainder_bits = QrCode._get_num_raw_data_modules(version) % 8
        lines.append(
            f"{version},{ecc_name},{data.hex().upper()},"
            f"{nayuki_stream.hex().upper()},{remainder_bits},{label}"
        )
    return "\n".join(lines) + "\n"


def check_fixture(path: pathlib.Path = FIXTURE_PATH) -> None:
    generated = render_fixture()
    committed = path.read_text(encoding="ascii")
    if generated != committed:
        raise ValueError(f"interleaved-codeword fixture drift: run {COMMAND}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", required=True)
    parser.parse_args()
    check_fixture()


if __name__ == "__main__":
    main()
