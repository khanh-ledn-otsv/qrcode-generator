"""Verify BCH information, completed mask candidates, and penalty selection."""

from __future__ import annotations

import argparse
import importlib.util
import pathlib

FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "mask_selection.csv"
PLACEMENT_SCRIPT = pathlib.Path(__file__).with_name("verify_placement_matrices.py")
COMMAND = (
    "uv run --project tests/oracles --locked python tests/support/verify_mask_selection.py --check"
)
SELECTION_CASES = ((2, "Q"), (7, "H"), (40, "L"))


def placement_fixture_module():
    spec = importlib.util.spec_from_file_location("placement_fixture", PLACEMENT_SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load support module {PLACEMENT_SCRIPT}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def extract_format_bits(matrix: list[list[bool]]) -> int:
    coordinates = [
        *((8, bit) for bit in range(6)),
        (8, 7),
        (8, 8),
        (7, 8),
        *((14 - bit, 8) for bit in range(9, 15)),
    ]
    return sum(int(matrix[y][x]) << bit for bit, (x, y) in enumerate(coordinates))


def extract_version_bits(matrix: list[list[bool]]) -> int:
    start = len(matrix) - 11
    return sum(int(matrix[bit // 3][start + bit % 3]) << bit for bit in range(18))


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
            score += max(0, run_length - 2) if run_length >= 5 else 0
            run_value = value
            run_length = 1
    score += max(0, run_length - 2) if run_length >= 5 else 0
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
        if matrix[y][x] == matrix[y][x + 1] == matrix[y + 1][x] == matrix[y + 1][x + 1]
    )
    dark = sum(value for row in matrix for value in row)
    return score + abs(dark * 20 - size * size * 10) // (size * size) * 10


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
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
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
        "# Rule 3 owner decision: literal complete 11-module windows; no virtual quiet-zone padding",
        "# Nayuki run-history totals are retained in nayuki_value when they differ",
        f"# Command: {COMMAND}",
        "kind,version,ecc,mask,value,nayuki_value,fnv1a64",
    ]
    for ecc_name in ("L", "M", "Q", "H"):
        for mask in range(8):
            matrix = oracle_matrix(placement, 2, ecc_name, mask)
            value = extract_format_bits(matrix)
            lines.append(f"format,2,{ecc_name},{mask},{value:04x},{value:04x},-")
    for version in range(7, 41):
        matrix = oracle_matrix(placement, version, "M", 0)
        value = extract_version_bits(matrix)
        lines.append(f"version,{version},M,0,{value:05x},{value:05x},-")
    for version, ecc_name in SELECTION_CASES:
        accepted_scores = []
        nayuki_scores = []
        for mask in range(8):
            matrix = oracle_matrix(placement, version, ecc_name, mask)
            accepted = reference_penalty(matrix)
            python = qrcode.util.lost_point(matrix)
            nayuki = nayuki_penalty(matrix)
            if accepted != python:
                raise ValueError(
                    f"Version {version}-{ecc_name} mask {mask} literal score disagreement"
                )
            accepted_scores.append(accepted)
            nayuki_scores.append(nayuki)
            lines.append(
                f"candidate,{version},{ecc_name},{mask},{accepted},{nayuki},{matrix_fingerprint(matrix)}"
            )
        selected = min(range(8), key=lambda mask: accepted_scores[mask])
        nayuki_selected = min(range(8), key=lambda mask: nayuki_scores[mask])
        if selected != nayuki_selected:
            raise ValueError(f"Version {version}-{ecc_name} automatic-mask disagreement")
        lines.append(
            f"selected,{version},{ecc_name},{selected},{accepted_scores[selected]},"
            f"{nayuki_scores[selected]},-"
        )
    for name, matrix in synthetic_penalty_matrices():
        accepted = reference_penalty(matrix)
        python = qrcode.util.lost_point(matrix)
        nayuki = nayuki_penalty(matrix)
        if accepted != python or accepted != nayuki:
            raise ValueError(f"synthetic penalty matrix {name} disagreement")
        lines.append(f"synthetic,0,{name},0,{accepted},{nayuki},{matrix_fingerprint(matrix)}")
    return "\n".join(lines) + "\n"


def check_fixture(path: pathlib.Path = FIXTURE_PATH) -> None:
    if render_fixture() != path.read_text(encoding="ascii"):
        raise ValueError(
            f"mask-selection fixture drift: run {COMMAND.replace('--check', '--write')}"
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--write", action="store_true")
    args = parser.parse_args()
    generated = render_fixture()
    if args.write:
        FIXTURE_PATH.write_text(generated, encoding="ascii", newline="\n")
    else:
        check_fixture()


if __name__ == "__main__":
    main()
