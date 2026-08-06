#!/usr/bin/env python3
"""Verify QR Reed–Solomon vectors with both pinned development oracles."""

from __future__ import annotations

import argparse
import importlib.metadata
import pathlib


DEGREES = (7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30)
FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "reed_solomon.csv"
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_reed_solomon.py --check"
)


def require_pin(distribution: str, version: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != version:
        raise RuntimeError(f"expected {distribution} {version}, got {actual}")


def python_generator(degree: int) -> list[int]:
    import qrcode.base

    polynomial = qrcode.base.Polynomial([1], 0)
    for exponent in range(degree):
        polynomial *= qrcode.base.Polynomial([1, qrcode.base.gexp(exponent)], 0)
    return polynomial.num


def oracle_vectors(degree: int, data: bytes) -> tuple[list[int], list[int]]:
    from qrcodegen import QrCode
    import qrcode.base

    nayuki_divisor = list(QrCode._reed_solomon_compute_divisor(degree))
    nayuki_generator = [1, *nayuki_divisor]
    nayuki_remainder = list(
        QrCode._reed_solomon_compute_remainder(data, nayuki_divisor)
    )

    qrcode_generator = python_generator(degree)
    raw_remainder = qrcode.base.Polynomial(list(data), degree) % qrcode.base.Polynomial(
        qrcode_generator, 0
    )
    qrcode_remainder = [0] * (degree - len(raw_remainder.num)) + raw_remainder.num

    if nayuki_generator != qrcode_generator:
        raise ValueError(f"degree {degree} generator disagreement")
    if nayuki_remainder != qrcode_remainder:
        raise ValueError(f"degree {degree} remainder disagreement")
    return nayuki_generator, nayuki_remainder


def cases() -> list[tuple[int, bytes, str]]:
    rows = []
    for degree in DEGREES:
        data = bytes([0, *(((index * 73) + degree) & 0xFF for index in range(1, degree + 7)), 0])
        rows.append((degree, data, "leading-trailing-zero"))
    rows.append((30, bytes(range(123)), "maximum-qr-data-block"))
    return rows


def render_fixture() -> str:
    require_pin("qrcodegen", "1.8.0")
    require_pin("qrcode", "8.2")
    lines = [
        "# public-corroborated, non-normative; ISO/IEC 18004:2024 clause mapping pending audit",
        "# Nayuki QR Code Generator 1.8.0 reed_solomon_*; python-qrcode 8.2 Polynomial/gexp/glog",
        f"# Command: {COMMAND}",
        "degree,generator_hex,data_hex,remainder_hex,case",
    ]
    for degree, data, label in cases():
        generator, remainder = oracle_vectors(degree, data)
        lines.append(
            f"{degree},{bytes(generator).hex().upper()},{data.hex().upper()},"
            f"{bytes(remainder).hex().upper()},{label}"
        )
    return "\n".join(lines) + "\n"


def check_fixture(path: pathlib.Path = FIXTURE_PATH) -> None:
    generated = render_fixture()
    committed = path.read_text(encoding="ascii")
    if generated != committed:
        raise ValueError(f"Reed–Solomon fixture drift: run {COMMAND}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", required=True)
    parser.parse_args()
    check_fixture()


if __name__ == "__main__":
    main()
