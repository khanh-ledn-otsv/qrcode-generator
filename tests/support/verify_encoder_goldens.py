#!/usr/bin/env python3
"""Generate composed encoder golden fingerprints from both pinned encoders."""

from __future__ import annotations

import argparse
import importlib.util
import pathlib


FIXTURE_PATH = pathlib.Path(__file__).parents[1] / "fixtures" / "encoder_goldens.csv"
GENERATORS_SCRIPT = pathlib.Path(__file__).with_name("generate_fixtures.py")
MASK_SCRIPT = pathlib.Path(__file__).with_name("verify_mask_selection.py")
COMMAND = (
    "uv run --project tests/oracles --locked python "
    "tests/support/verify_encoder_goldens.py --check"
)


def load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def payload(mode: str, length: int, seed: int) -> bytes:
    if mode == "numeric":
        return bytes(ord("0") + (index * 7 + seed) % 10 for index in range(length))
    if mode == "alphanumeric":
        alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 $%*+-./:"
        return bytes(alphabet[(index * 11 + seed) % len(alphabet)] for index in range(length))
    if mode == "byte":
        alphabet = b"abcdefghijklmnopqrstuvwxyz_@"
        return bytes(alphabet[(index * 17 + seed) % len(alphabet)] for index in range(length))
    if mode == "utf8":
        alphabet = ("é", "à", "QR", "ç")
        return "".join(alphabet[(index + seed) % len(alphabet)] for index in range(length)).encode()
    raise ValueError(f"unsupported mode {mode}")


def nayuki_segments(data: bytes, mode: str):
    from qrcodegen import QrSegment

    if mode == "numeric":
        return [QrSegment.make_numeric(data.decode("ascii"))]
    if mode == "alphanumeric":
        return [QrSegment.make_alphanumeric(data.decode("ascii"))]
    if mode == "byte":
        return [QrSegment.make_bytes(data)]
    if mode == "utf8":
        return [QrSegment.make_eci(26), QrSegment.make_bytes(data)]
    raise ValueError(f"unsupported mode {mode}")


def auto_version(data: bytes, mode: str, ecc_name: str) -> int:
    from qrcodegen import DataTooLongError, QrCode

    ecc = {
        "L": QrCode.Ecc.LOW,
        "M": QrCode.Ecc.MEDIUM,
        "Q": QrCode.Ecc.QUARTILE,
        "H": QrCode.Ecc.HIGH,
    }[ecc_name]
    try:
        code = QrCode.encode_segments(
            nayuki_segments(data, mode),
            ecc,
            minversion=1,
            maxversion=40,
            mask=0,
            boostecl=False,
        )
    except DataTooLongError:
        return 41
    return (code.get_size() - 17) // 4


def first_length_for_version(target: int, mode: str, ecc: str) -> int:
    low, high = 1, 4096
    while low < high:
        middle = low + (high - low) // 2
        if auto_version(payload(mode, middle, 0), mode, ecc) < target:
            low = middle + 1
        else:
            high = middle
    if auto_version(payload(mode, low, 0), mode, ecc) != target:
        raise ValueError(f"no {mode} {ecc} payload selects Version {target}")
    return low


def largest_length_for_version(target: int, mode: str, ecc: str) -> int:
    if target == 40:
        raise ValueError("Version 40 has no next-version boundary")
    return first_length_for_version(target + 1, mode, ecc) - 1


def matrix_rows(text: str) -> list[list[bool]]:
    return [[value == "1" for value in row] for row in text.splitlines()]


def fingerprint(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode("ascii"):
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def evaluate_case(generators, mask_reference, name: str, data: bytes, mode: str, ecc: str):
    version = auto_version(data, mode, ecc)
    fixture_mode = "byte" if mode == "utf8" else mode
    eci = 26 if mode == "utf8" else None
    candidates = []
    for mask in range(8):
        fixture = {
            "mode": fixture_mode,
            "eci_assignment": eci,
            "ecc": ecc,
            "version": version,
            "mask": mask,
        }
        nayuki = generators.generate_nayuki(data, fixture)
        python = generators.generate_python_qrcode(data, fixture)
        if nayuki != python:
            raise ValueError(f"{name} Version {version}-{ecc} mask {mask} matrix disagreement")
        candidates.append((mask_reference.reference_penalty(matrix_rows(nayuki)), nayuki))
    selected = min(range(8), key=lambda mask: candidates[mask][0])
    return version, selected, fingerprint(candidates[selected][1])


def render_fixture() -> str:
    generators = load_module("fixture_generators", GENERATORS_SCRIPT)
    mask_reference = load_module("mask_reference", MASK_SCRIPT)
    generators.require_pinned_package("qrcodegen", "1.8.0")
    generators.require_pinned_package("qrcode", "8.2")

    v1_byte = largest_length_for_version(1, "byte", "Q")
    v9_numeric = largest_length_for_version(9, "numeric", "L")
    v26_alpha = largest_length_for_version(26, "alphanumeric", "M")
    specifications = [
        ("v01-q-byte-exact", "byte", "Q", v1_byte, 0),
        ("v02-q-byte-over", "byte", "Q", v1_byte + 1, 0),
        ("v06-h-byte-first", "byte", "H", first_length_for_version(6, "byte", "H"), 0),
        ("v07-m-numeric-first", "numeric", "M", first_length_for_version(7, "numeric", "M"), 0),
        ("v09-l-numeric-exact", "numeric", "L", v9_numeric, 0),
        ("v10-l-numeric-over", "numeric", "L", v9_numeric + 1, 0),
        ("v26-m-alpha-exact", "alphanumeric", "M", v26_alpha, 0),
        ("v27-m-alpha-over", "alphanumeric", "M", v26_alpha + 1, 0),
        ("v40-l-utf8-first", "utf8", "L", first_length_for_version(40, "utf8", "L"), 0),
    ]
    rows = []
    covered_masks = set()
    for name, mode, ecc, length, seed in specifications:
        data = payload(mode, length, seed)
        version, mask, matrix_hash = evaluate_case(
            generators, mask_reference, name, data, mode, ecc
        )
        rows.append((name, mode, ecc, version, mask, 26 if mode == "utf8" else 0, data, matrix_hash))
        covered_masks.add(mask)

    search_limits = {
        ecc: largest_length_for_version(1, "byte", ecc) for ecc in ("L", "M", "Q", "H")
    }
    for seed in range(1, 2048):
        if len(covered_masks) == 8:
            break
        ecc = ("L", "M", "Q", "H")[seed % 4]
        length = 1 + (seed * 13) % search_limits[ecc]
        data = payload("byte", length, seed)
        name = f"v01-{ecc.lower()}-byte-mask-search-{seed:04}"
        version, mask, matrix_hash = evaluate_case(
            generators, mask_reference, name, data, "byte", ecc
        )
        if mask not in covered_masks:
            rows.append((name, "byte", ecc, version, mask, 0, data, matrix_hash))
            covered_masks.add(mask)
    if covered_masks != set(range(8)):
        raise ValueError(f"composed golden cases cover masks {sorted(covered_masks)}")

    lines = [
        "# public-corroborated, non-normative; completed matrices agree between qrcodegen 1.8.0 and python-qrcode 8.2",
        "# UTF-8 ECI 26 is manually framed through python-qrcode's independent bit-buffer/RS/matrix path",
        f"# Command: {COMMAND}",
        "name,mode,ecc,version,mask,eci,payload_hex,fnv1a64",
    ]
    lines.extend(
        f"{name},{mode},{ecc},{version},{mask},{eci},{data.hex()},{matrix_hash}"
        for name, mode, ecc, version, mask, eci, data, matrix_hash in rows
    )
    return "\n".join(lines) + "\n"


def check_fixture() -> None:
    if render_fixture() != FIXTURE_PATH.read_text(encoding="ascii"):
        raise ValueError(f"encoder golden fixture drift: run {COMMAND.replace('--check', '--write')}")


def main() -> None:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--write", action="store_true")
    args = parser.parse_args()
    rendered = render_fixture()
    if args.write:
        FIXTURE_PATH.write_text(rendered, encoding="ascii", newline="\n")
    else:
        check_fixture()


if __name__ == "__main__":
    main()
