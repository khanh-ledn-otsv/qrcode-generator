#!/usr/bin/env python3
"""Verify classified QR function-matrix fixtures with both pinned encoders."""

from __future__ import annotations

import argparse
import importlib.metadata
import pathlib
from unittest.mock import patch


FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "function_matrices.txt"
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_function_matrices.py --check"
)
VERSIONS = (1, 2, 7, 40)
STABLE_VALUE_KINDS = {"finder", "separator", "timing", "alignment", "dark"}


def require_pin(distribution: str, version: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != version:
        raise RuntimeError(f"expected {distribution} {version}, got {actual}")


def alignment_centers(version: int) -> tuple[int, ...]:
    from qrcodegen import QrCode
    import qrcode.util

    code = object.__new__(QrCode)
    code._version = version
    code._size = 17 + 4 * version
    nayuki = tuple(code._get_alignment_pattern_positions())
    python_qrcode = tuple(qrcode.util.PATTERN_POSITION_TABLE[version - 1])
    if nayuki != python_qrcode:
        raise ValueError(f"Version {version} alignment-center disagreement")
    return nayuki


def nayuki_function_state(version: int) -> tuple[set[tuple[int, int]], dict[tuple[int, int], bool]]:
    from qrcodegen import QrCode

    coordinates: set[tuple[int, int]] = set()
    original = QrCode._set_function_module

    def record(self, x: int, y: int, dark: bool) -> None:
        coordinates.add((x, y))
        original(self, x, y, dark)

    with patch.object(QrCode, "_set_function_module", record):
        code = QrCode.encode_segments(
            [],
            QrCode.Ecc.LOW,
            minversion=version,
            maxversion=version,
            mask=0,
            boostecl=False,
        )
    values = {(x, y): code.get_module(x, y) for x, y in coordinates}
    return coordinates, values


def python_qrcode_function_state(
    version: int,
) -> tuple[set[tuple[int, int]], dict[tuple[int, int], bool]]:
    import qrcode

    code = qrcode.QRCode(
        version=version,
        error_correction=qrcode.constants.ERROR_CORRECT_L,
        border=0,
        mask_pattern=0,
    )
    size = 17 + 4 * version
    code.modules_count = size
    code.modules = [[None] * size for _ in range(size)]
    code.setup_position_probe_pattern(0, 0)
    code.setup_position_probe_pattern(size - 7, 0)
    code.setup_position_probe_pattern(0, size - 7)
    code.setup_position_adjust_pattern()
    code.setup_timing_pattern()
    code.setup_type_info(False, 0)
    if version >= 7:
        code.setup_type_number(False)
    coordinates = {
        (x, y)
        for y, row in enumerate(code.modules)
        for x, module in enumerate(row)
        if module is not None
    }
    values = {(x, y): bool(code.modules[y][x]) for x, y in coordinates}
    return coordinates, values


def classified_matrix(version: int) -> list[list[tuple[str, bool, str]]]:
    size = 17 + 4 * version
    cells: list[list[tuple[str, bool, str] | None]] = [
        [None for _ in range(size)] for _ in range(size)
    ]

    def write(x: int, y: int, dark: bool, kind: str, glyph: str) -> None:
        if not (0 <= x < size and 0 <= y < size):
            raise ValueError(f"Version {version} out-of-bounds fixture coordinate ({x}, {y})")
        if cells[y][x] is not None:
            raise ValueError(f"Version {version} duplicate fixture coordinate ({x}, {y})")
        cells[y][x] = (glyph.upper() if dark else glyph.lower(), dark, kind)

    def finder(origin_x: int, origin_y: int) -> None:
        for offset_y in range(7):
            for offset_x in range(7):
                dark = (
                    offset_x in (0, 6)
                    or offset_y in (0, 6)
                    or (2 <= offset_x <= 4 and 2 <= offset_y <= 4)
                )
                write(origin_x + offset_x, origin_y + offset_y, dark, "finder", "f")

    finder(0, 0)
    finder(size - 7, 0)
    finder(0, size - 7)
    for offset in range(8):
        write(7, offset, False, "separator", "s")
        write(size - 8, offset, False, "separator", "s")
        write(7, size - 1 - offset, False, "separator", "s")
    for offset in range(7):
        write(offset, 7, False, "separator", "s")
        write(size - 1 - offset, 7, False, "separator", "s")
        write(offset, size - 8, False, "separator", "s")

    centers = alignment_centers(version)
    final_center = centers[-1] if centers else None
    for center_y in centers:
        for center_x in centers:
            if (center_x == 6 and center_y in (6, final_center)) or (
                center_x == final_center and center_y == 6
            ):
                continue
            for offset_y in range(-2, 3):
                for offset_x in range(-2, 3):
                    dark = (
                        abs(offset_x) == 2
                        or abs(offset_y) == 2
                        or (offset_x == 0 and offset_y == 0)
                    )
                    write(center_x + offset_x, center_y + offset_y, dark, "alignment", "a")

    for coordinate in range(8, size - 8):
        dark = coordinate % 2 == 0
        if cells[6][coordinate] is None:
            write(coordinate, 6, dark, "timing", "t")
        if cells[coordinate][6] is None:
            write(6, coordinate, dark, "timing", "t")

    for coordinate in range(6):
        write(8, coordinate, False, "format", "r")
        write(coordinate, 8, False, "format", "r")
    write(8, 7, False, "format", "r")
    write(8, 8, False, "format", "r")
    write(7, 8, False, "format", "r")
    for offset in range(8):
        write(size - 1 - offset, 8, False, "format", "r")
    for offset in range(7):
        write(8, size - 1 - offset, False, "format", "r")

    if version >= 7:
        start = size - 11
        for offset in range(6):
            for band in range(3):
                write(start + band, offset, False, "version", "v")
                write(offset, start + band, False, "version", "v")

    write(8, size - 8, True, "dark", "d")
    for y in range(size):
        for x in range(size):
            if cells[y][x] is None:
                cells[y][x] = (".", False, "data")
    return [[cell for cell in row if cell is not None] for row in cells]


def verify_oracles(version: int, matrix: list[list[tuple[str, bool, str]]]) -> None:
    expected_coordinates = {
        (x, y)
        for y, row in enumerate(matrix)
        for x, (_, _, kind) in enumerate(row)
        if kind != "data"
    }
    nayuki_coordinates, nayuki_values = nayuki_function_state(version)
    python_coordinates, python_values = python_qrcode_function_state(version)
    if nayuki_coordinates != python_coordinates:
        raise ValueError(f"Version {version} function-coordinate oracle disagreement")
    if expected_coordinates != nayuki_coordinates:
        raise ValueError(f"Version {version} classified function-coordinate disagreement")
    for y, row in enumerate(matrix):
        for x, (_, expected_dark, kind) in enumerate(row):
            if kind in STABLE_VALUE_KINDS:
                if nayuki_values[(x, y)] != expected_dark:
                    raise ValueError(f"Version {version} Nayuki value disagreement at ({x}, {y})")
                if python_values[(x, y)] != expected_dark:
                    raise ValueError(
                        f"Version {version} python-qrcode value disagreement at ({x}, {y})"
                    )


def render_fixture() -> str:
    require_pin("qrcodegen", "1.8.0")
    require_pin("qrcode", "8.2")
    lines = [
        "# public-corroborated, non-normative; ISO/IEC 18004:2024 clause mapping pending audit",
        "# Coordinates and fixed function values: Nayuki 1.8.0 + python-qrcode 8.2",
        f"# Command: {COMMAND}",
        "# Legend: F/f finder dark/light, s separator, T/t timing dark/light,",
        "# A/a alignment dark/light, r format reservation, v version reservation,",
        "# D fixed dark, . data placeholder",
    ]
    for version in VERSIONS:
        matrix = classified_matrix(version)
        verify_oracles(version, matrix)
        lines.append(f"version={version} size={17 + 4 * version}")
        lines.extend("".join(cell[0] for cell in row) for row in matrix)
        lines.append("end")
    return "\n".join(lines) + "\n"


def check_fixture(path: pathlib.Path = FIXTURE_PATH) -> None:
    generated = render_fixture()
    committed = path.read_text(encoding="ascii")
    if generated != committed:
        raise ValueError(f"function-matrix fixture drift: run {COMMAND}")


def main() -> None:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--write", action="store_true")
    args = parser.parse_args()
    generated = render_fixture()
    if args.check:
        committed = FIXTURE_PATH.read_text(encoding="ascii")
        if generated != committed:
            raise ValueError(f"function-matrix fixture drift: run {COMMAND}")
    else:
        FIXTURE_PATH.write_text(generated, encoding="ascii", newline="\n")


if __name__ == "__main__":
    main()
