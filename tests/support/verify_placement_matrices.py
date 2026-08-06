#!/usr/bin/env python3
"""Verify explicit-mask data placement with both pinned QR encoders."""

from __future__ import annotations

import argparse
import importlib.metadata
import importlib.util
import pathlib
from unittest.mock import patch


FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "placement_matrices.txt"
FUNCTION_SCRIPT = pathlib.Path(__file__).with_name("verify_function_matrices.py")
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_placement_matrices.py --check"
)
CASES = ((1, "M", 0), (2, "Q", 3), (7, "H", 7), (40, "L", 5))
MASKS = tuple(range(8))


def require_pin(distribution: str, version: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != version:
        raise RuntimeError(f"expected {distribution} {version}, got {actual}")


def function_fixture_module():
    spec = importlib.util.spec_from_file_location("function_fixture", FUNCTION_SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def ecc_values(ecc_name: str):
    from qrcodegen import QrCode
    import qrcode.constants

    return (
        {
            "L": QrCode.Ecc.LOW,
            "M": QrCode.Ecc.MEDIUM,
            "Q": QrCode.Ecc.QUARTILE,
            "H": QrCode.Ecc.HIGH,
        }[ecc_name],
        {
            "L": qrcode.constants.ERROR_CORRECT_L,
            "M": qrcode.constants.ERROR_CORRECT_M,
            "Q": qrcode.constants.ERROR_CORRECT_Q,
            "H": qrcode.constants.ERROR_CORRECT_H,
        }[ecc_name],
    )


def synthetic_data(version: int, python_ecc: int) -> tuple[bytes, bytes]:
    import qrcode.base
    import qrcode.util

    blocks = qrcode.base.rs_blocks(version, python_ecc)
    data = bytes((index * 149 + version) & 0xFF for index in range(sum(b.data_count for b in blocks)))
    buffer = qrcode.util.BitBuffer()
    buffer.buffer = list(data)
    buffer.length = len(data) * 8
    interleaved = bytes(qrcode.util.create_bytes(buffer, blocks))
    return data, interleaved


def nayuki_result(version: int, ecc, data: bytes, mask: int):
    from qrcodegen import QrCode

    placements: list[tuple[int, int]] = []
    recording = [False]
    original = QrCode._draw_codewords

    class TrackingRow(list):
        def __init__(self, values, y: int):
            super().__init__(values)
            self.y = y

        def __setitem__(self, x, value):
            if recording[0]:
                placements.append((x, self.y))
            super().__setitem__(x, value)

    def draw_codewords(self, codewords) -> None:
        self._modules = [TrackingRow(row, y) for y, row in enumerate(self._modules)]
        recording[0] = True
        try:
            original(self, codewords)
        finally:
            recording[0] = False

    with patch.object(QrCode, "_draw_codewords", draw_codewords):
        code = QrCode(version, ecc, data, mask)
    matrix = [
        [code.get_module(x, y) for x in range(code.get_size())]
        for y in range(code.get_size())
    ]
    return matrix, placements


def python_qrcode_result(version: int, ecc: int, interleaved: bytes, mask: int):
    import qrcode

    placements: list[tuple[int, int]] = []
    recording = [False]
    original = qrcode.QRCode.map_data

    class TrackingRow(list):
        def __init__(self, values, y: int):
            super().__init__(values)
            self.y = y

        def __setitem__(self, x, value):
            if recording[0]:
                placements.append((x, self.y))
            super().__setitem__(x, value)

    def map_data(self, data, mask_pattern) -> None:
        self.modules = [TrackingRow(row, y) for y, row in enumerate(self.modules)]
        recording[0] = True
        try:
            original(self, data, mask_pattern)
        finally:
            recording[0] = False

    code = qrcode.QRCode(
        version=version,
        error_correction=ecc,
        border=0,
        mask_pattern=mask,
    )
    code.data_cache = list(interleaved)
    with patch.object(qrcode.QRCode, "map_data", map_data):
        code.makeImpl(False, mask)
    return [[bool(module) for module in row] for row in code.modules], placements


def classified_matrix(version: int, ecc_name: str, mask: int, function_state):
    nayuki_ecc, python_ecc = ecc_values(ecc_name)
    data, interleaved = synthetic_data(version, python_ecc)
    nayuki_matrix, nayuki_placements = nayuki_result(version, nayuki_ecc, data, mask)
    python_matrix, python_placements = python_qrcode_result(
        version, python_ecc, interleaved, mask
    )
    if nayuki_matrix != python_matrix:
        raise ValueError(f"Version {version}-{ecc_name} mask {mask} matrix disagreement")
    data_bit_count = len(interleaved) * 8
    if nayuki_placements != python_placements[:data_bit_count]:
        raise ValueError(f"Version {version}-{ecc_name} mask {mask} traversal disagreement")
    if len(set(python_placements)) != len(python_placements):
        raise ValueError(f"Version {version}-{ecc_name} mask {mask} duplicate placement")

    data_coordinates = set(nayuki_placements)
    function_coordinates = set(function_state)
    remainder_coordinates = set(python_placements[data_bit_count:])
    if data_coordinates & function_coordinates or remainder_coordinates & function_coordinates:
        raise ValueError(f"Version {version}-{ecc_name} mask {mask} function overwrite")
    size = 17 + 4 * version
    matrix = []
    for y in range(size):
        row = []
        for x in range(size):
            coordinate = (x, y)
            if coordinate in function_state:
                row.append(function_state[coordinate])
            elif coordinate in data_coordinates:
                row.append(("data", nayuki_matrix[y][x]))
            elif coordinate in remainder_coordinates:
                row.append(("remainder", nayuki_matrix[y][x]))
            else:
                raise ValueError(
                    f"Version {version}-{ecc_name} mask {mask} unowned coordinate {coordinate}"
                )
        matrix.append(row)
    return matrix, len(data), len(interleaved), len(remainder_coordinates)


def glyph(cell: tuple[str, bool]) -> str:
    kind, dark = cell
    symbols = {
        "finder": "F" if dark else "f",
        "separator": "s",
        "timing": "T" if dark else "t",
        "alignment": "A" if dark else "a",
        "format": "r",
        "version": "v",
        "dark": "D",
        "data": "B" if dark else "b",
        "remainder": "E" if dark else "e",
    }
    return symbols[kind]


def matrix_text(matrix: list[list[tuple[str, bool]]]) -> str:
    return "".join("".join(glyph(cell) for cell in row) + "\n" for row in matrix)


def fnv1a64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode("ascii"):
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def render_fixture() -> str:
    require_pin("qrcodegen", "1.8.0")
    require_pin("qrcode", "8.2")
    function_fixtures = function_fixture_module()
    lines = [
        "# public-corroborated, non-normative; ISO/IEC 18004:2024 clause mapping pending audit",
        "# Data codeword byte i is (i * 149 + version) modulo 256",
        "# Placement/masks: Nayuki 1.8.0 + python-qrcode 8.2",
        f"# Command: {COMMAND}",
        "version,ecc,mask,data_codewords,interleaved_codewords,remainder_bits,fnv1a64",
    ]
    readable = []
    for version, ecc_name, readable_mask in CASES:
        function_state = function_fixtures.nayuki_classified_state(version)
        for mask in MASKS:
            matrix, data_count, interleaved_count, remainder_count = classified_matrix(
                version, ecc_name, mask, function_state
            )
            lines.append(
                f"{version},{ecc_name},{mask},{data_count},{interleaved_count},"
                f"{remainder_count},{fnv1a64(matrix_text(matrix))}"
            )
            if mask == readable_mask:
                readable.append((version, ecc_name, mask, matrix))
    lines.append("endhashes")
    lines.extend(
        [
            "# Legend: F/f finder, s separator, T/t timing, A/a alignment,",
            "# r format reservation, v version reservation, D fixed dark,",
            "# B/b data dark/light, E/e remainder dark/light",
        ]
    )
    for version, ecc_name, mask, matrix in readable:
        lines.append(f"version={version} ecc={ecc_name} mask={mask} size={17 + 4 * version}")
        lines.extend(matrix_text(matrix).splitlines())
        lines.append("end")
    return "\n".join(lines) + "\n"


def check_fixture(path: pathlib.Path = FIXTURE_PATH) -> None:
    generated = render_fixture()
    committed = path.read_text(encoding="ascii")
    if generated != committed:
        raise ValueError(f"placement-matrix fixture drift: run {COMMAND}")


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
            raise ValueError(f"placement-matrix fixture drift: run {COMMAND}")
    else:
        FIXTURE_PATH.write_text(generated, encoding="ascii", newline="\n")


if __name__ == "__main__":
    main()
