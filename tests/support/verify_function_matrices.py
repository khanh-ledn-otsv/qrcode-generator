"""Verify classified QR function-matrix fixtures with both pinned encoders."""

from __future__ import annotations

import argparse
import importlib.metadata
import pathlib
from collections.abc import Callable
from unittest.mock import patch

FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "function_matrices.txt"
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_function_matrices.py --check"
)
VERSIONS = (1, 2, 7, 40)
ALL_VERSIONS = range(1, 41)


def require_pin(distribution: str, version: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != version:
        raise RuntimeError(f"expected {distribution} {version}, got {actual}")


def normalize(kind: str, dark: bool) -> tuple[str, bool]:
    if kind in {"format", "version"}:
        return kind, False
    return kind, dark


def nayuki_classified_state(version: int) -> dict[tuple[int, int], tuple[str, bool]]:
    from qrcodegen import QrCode

    classified: dict[tuple[int, int], tuple[str, bool]] = {}
    context: list[tuple[str, int, int] | None] = [None]
    original_set = QrCode._set_function_module
    original_finder = QrCode._draw_finder_pattern
    original_alignment = QrCode._draw_alignment_pattern
    original_format = QrCode._draw_format_bits
    original_version = QrCode._draw_version

    def with_context(value: tuple[str, int, int], action: Callable[..., None], self, *args) -> None:
        previous = context[0]
        context[0] = value
        try:
            action(self, *args)
        finally:
            context[0] = previous

    def finder(self, x: int, y: int) -> None:
        with_context(("finder", x, y), original_finder, self, x, y)

    def alignment(self, x: int, y: int) -> None:
        with_context(("alignment", x, y), original_alignment, self, x, y)

    def format_bits(self, mask: int) -> None:
        with_context(("format", 0, 0), original_format, self, mask)

    def version_bits(self) -> None:
        with_context(("version", 0, 0), original_version, self)

    def record(self, x: int, y: int, dark: bool) -> None:
        active = context[0]
        if active is None:
            kind = "timing"
        elif active[0] == "finder":
            _, center_x, center_y = active
            kind = "finder" if abs(x - center_x) <= 3 and abs(y - center_y) <= 3 else "separator"
        elif active[0] == "format" and (x, y) == (8, self._size - 8):
            kind = "dark"
        else:
            kind = active[0]
        value = normalize(kind, dark)
        previous = classified.get((x, y))
        if previous is not None and previous[0] != kind and previous[0] != "timing":
            raise ValueError(
                f"Version {version} Nayuki changed kind at ({x}, {y}): {previous[0]} to {kind}"
            )
        classified[(x, y)] = value
        original_set(self, x, y, dark)

    with (
        patch.object(QrCode, "_set_function_module", record),
        patch.object(QrCode, "_draw_finder_pattern", finder),
        patch.object(QrCode, "_draw_alignment_pattern", alignment),
        patch.object(QrCode, "_draw_format_bits", format_bits),
        patch.object(QrCode, "_draw_version", version_bits),
    ):
        QrCode.encode_segments(
            [],
            QrCode.Ecc.LOW,
            minversion=version,
            maxversion=version,
            mask=0,
            boostecl=False,
        )
    return classified


def python_qrcode_classified_state(
    version: int,
) -> dict[tuple[int, int], tuple[str, bool]]:
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
    classified: dict[tuple[int, int], tuple[str, bool]] = {}

    def capture(kind: str, action: Callable[[], None]) -> set[tuple[int, int]]:
        before = set(classified)
        action()
        added = {
            (x, y)
            for y, row in enumerate(code.modules)
            for x, module in enumerate(row)
            if module is not None and (x, y) not in before
        }
        for x, y in added:
            actual_kind = "dark" if kind == "format" and (x, y) == (8, size - 8) else kind
            classified[(x, y)] = normalize(actual_kind, bool(code.modules[y][x]))
        return added

    for origin_x, origin_y in ((0, 0), (size - 7, 0), (0, size - 7)):
        added = capture(
            "finder",
            lambda origin_x=origin_x, origin_y=origin_y: code.setup_position_probe_pattern(
                origin_y, origin_x
            ),
        )
        for x, y in added:
            if not (origin_x <= x < origin_x + 7 and origin_y <= y < origin_y + 7):
                classified[(x, y)] = ("separator", False)
    capture("alignment", code.setup_position_adjust_pattern)
    capture("timing", code.setup_timing_pattern)
    capture("format", lambda: code.setup_type_info(False, 0))
    if version >= 7:
        capture("version", lambda: code.setup_type_number(False))
    return classified


def classified_matrix(version: int) -> list[list[tuple[str, bool]]]:
    nayuki = nayuki_classified_state(version)
    python_qrcode = python_qrcode_classified_state(version)
    if nayuki != python_qrcode:
        differing = sorted(
            coordinate
            for coordinate in set(nayuki) | set(python_qrcode)
            if nayuki.get(coordinate) != python_qrcode.get(coordinate)
        )
        raise ValueError(
            f"Version {version} classified function oracle disagreement at {differing[:10]}"
        )
    size = 17 + 4 * version
    return [[nayuki.get((x, y), ("data", False)) for x in range(size)] for y in range(size)]


def glyph(cell: tuple[str, bool]) -> str:
    kind, dark = cell
    symbols = {
        "data": ".",
        "finder": "F" if dark else "f",
        "separator": "s",
        "timing": "T" if dark else "t",
        "alignment": "A" if dark else "a",
        "format": "r",
        "version": "v",
        "dark": "D",
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
    lines = [
        "# public-corroborated, non-normative; ISO/IEC 18004:2024 clause mapping pending audit",
        "# Coordinates and fixed function values: Nayuki 1.8.0 + python-qrcode 8.2",
        f"# Command: {COMMAND}",
        "# Legend: F/f finder dark/light, s separator, T/t timing dark/light,",
        "# A/a alignment dark/light, r format reservation, v version reservation,",
        "# D fixed dark, . data placeholder",
    ]
    matrices = {version: classified_matrix(version) for version in ALL_VERSIONS}
    lines.append("version,size,fnv1a64")
    for version, matrix in matrices.items():
        lines.append(f"{version},{17 + 4 * version},{fnv1a64(matrix_text(matrix))}")
    lines.append("endhashes")
    for version in VERSIONS:
        matrix = matrices[version]
        lines.append(f"version={version} size={17 + 4 * version}")
        lines.extend(matrix_text(matrix).splitlines())
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
