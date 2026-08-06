#!/usr/bin/env python3
"""Verify BCH information, completed mask candidates, and penalty selection."""

from __future__ import annotations

import argparse
import importlib.util
import pathlib


FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "mask_selection.csv"
PLACEMENT_SCRIPT = pathlib.Path(__file__).with_name("verify_placement_matrices.py")
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_mask_selection.py --check"
)
SELECTION_CASES = ((2, "Q"), (7, "H"), (40, "L"))


def placement_fixture_module():
    spec = importlib.util.spec_from_file_location("placement_fixture", PLACEMENT_SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def bit_value(matrix: list[list[bool]], x: int, y: int) -> int:
    return int(matrix[y][x])


def extract_format_bits(matrix: list[list[bool]]) -> int:
    coordinates = [
        *((8, bit) for bit in range(6)),
        (8, 7),
        (8, 8),
        (7, 8),
        *((14 - bit, 8) for bit in range(9, 15)),
    ]
    return sum(bit_value(matrix, x, y) << bit for bit, (x, y) in enumerate(coordinates))


def extract_version_bits(matrix: list[list[bool]]) -> int:
    start = len(matrix) - 11
    return sum(
        bit_value(matrix, start + bit % 3, bit // 3) << bit for bit in range(18)
    )


def line_penalty(line: list[bool]) -> int:
    if not line:
        return 0
    score = 0
    run_value = line[0]
    run_length = 0
    for value in line:
        if value == run_value:
            run_length += 1
        else:
            if run_length >= 5:
                score += run_length - 2
            run_value = value
            run_length = 1
    if run_length >= 5:
        score += run_length - 2
    patterns = (
        [False, False, False, False, True, False, True, True, True, False, True],
        [True, False, True, True, True, False, True, False, False, False, False],
    )
    score += sum(line[index : index + 11] in patterns for index in range(len(line) - 10)) * 40
    return score


def reference_penalty(matrix: list[list[bool]]) -> int:
    size = len(matrix)
    score = sum(line_penalty(row) for row in matrix)
    score += sum(line_penalty([matrix[y][x] for y in range(size)]) for x in range(size))
    score += sum(
        3
        for y in range(size - 1)
        for x in range(size - 1)
        if matrix[y][x]
        == matrix[y][x + 1]
        == matrix[y + 1][x]
        == matrix[y + 1][x + 1]
    )
    dark = sum(value for row in matrix for value in row)
    total = size * size
    score += abs(dark * 20 - total * 10) // total * 10
    return score


def nayuki_penalty(matrix: list[list[bool]]) -> int:
    from qrcodegen import QrCode

    code = object.__new__(QrCode)
    code._modules = matrix
    code._size = len(matrix)
    return code._get_penalty_score()


def matrix_fingerprint(matrix: list[list[bool]]) -> str:
    text = "".join("".join("1" if value else "0" for value in row) + "\n" for row in matrix)
    value = 0xCBF29CE484222325
    for byte in text.encode("ascii"):
        value ^= byte
        value = value * 0x100000001B3 & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def oracle_matrix(placement, version: int, ecc_name: str, mask: int):
    nayuki_ecc, python_ecc = placement.ecc_values(ecc_name)
    data, interleaved = placement.synthetic_data(version, python_ecc)
    nayuki, _ = placement.nayuki_result(version, nayuki_ecc, data, mask)
    python, _ = placement.python_qrcode_result(version, python_ecc, interleaved, mask)
    if nayuki != python:
        raise ValueError(f"Version {version}-{ecc_name} mask {mask} matrix disagreement")
    return nayuki


def synthetic_penalty_matrices() -> list[tuple[str, list[list[bool]]]]:
    checkerboard = [[(x + y) % 2 == 0 for x in range(21)] for y in range(21)]
    contextual = [[(x + y) % 2 == 0 for x in range(21)] for y in range(21)]
    pattern = [False] * 4 + [True, False, True, True, True, False, True, False, True]
    contextual[10][3 : 3 + len(pattern)] = pattern
    return [("checkerboard", checkerboard), ("contextual-finder", contextual)]


def render_fixture() -> str:
    import qrcode.util

    placement = placement_fixture_module()
    placement.require_pin("qrcodegen", "1.8.0")
    placement.require_pin("qrcode", "8.2")
    lines = [
        "# public-corroborated, non-normative; ISO/IEC 18004:2024 clause mapping pending audit",
        "# BCH/final matrices: Nayuki 1.8.0 + python-qrcode 8.2; penalties: both on isolated agreement matrices, Nayuki + independent slow reference on QR candidates",
        f"# Command: {COMMAND}",
        "kind,version,ecc,mask,value,fnv1a64",
    ]
    for ecc_name in ("L", "M", "Q", "H"):
        for mask in range(8):
            matrix = oracle_matrix(placement, 2, ecc_name, mask)
            lines.append(f"format,2,{ecc_name},{mask},{extract_format_bits(matrix):04x},-")
    for version in range(7, 41):
        matrix = oracle_matrix(placement, version, "M", 0)
        lines.append(f"version,{version},M,0,{extract_version_bits(matrix):05x},-")
    for version, ecc_name in SELECTION_CASES:
        candidates = []
        nayuki_scores = []
        python_scores = []
        for mask in range(8):
            matrix = oracle_matrix(placement, version, ecc_name, mask)
            score = reference_penalty(matrix)
            python_score = qrcode.util.lost_point(matrix)
            if score != python_score:
                raise ValueError(f"Version {version}-{ecc_name} mask {mask} score disagreement")
            candidates.append(score)
            nayuki_scores.append(nayuki_penalty(matrix))
            python_scores.append(python_score)
            lines.append(
                f"candidate,{version},{ecc_name},{mask},{score},{matrix_fingerprint(matrix)}"
            )
        selected = min(range(8), key=lambda mask: candidates[mask])
        nayuki_selected = min(range(8), key=lambda mask: nayuki_scores[mask])
        python_selected = min(range(8), key=lambda mask: python_scores[mask])
        if selected != python_selected or selected != nayuki_selected:
            raise ValueError(f"Version {version}-{ecc_name} automatic-mask disagreement")
        lines.append(f"selected,{version},{ecc_name},{selected},{candidates[selected]},-")
    for name, matrix in synthetic_penalty_matrices():
        nayuki = nayuki_penalty(matrix)
        python = qrcode.util.lost_point(matrix)
        if nayuki != python:
            raise ValueError(f"synthetic penalty matrix {name} disagreement")
        lines.append(f"synthetic,0,{name},0,{nayuki},{matrix_fingerprint(matrix)}")
    return "\n".join(lines) + "\n"


def check_fixture(path: pathlib.Path = FIXTURE_PATH) -> None:
    if render_fixture() != path.read_text(encoding="ascii"):
        raise ValueError(f"mask-selection fixture drift: run {COMMAND.replace('--check', '--write')}")


def main() -> None:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--write", action="store_true")
    args = parser.parse_args()
    generated = render_fixture()
    if args.write:
        FIXTURE_PATH.write_text(generated, encoding="ascii")
    else:
        check_fixture()


if __name__ == "__main__":
    main()
