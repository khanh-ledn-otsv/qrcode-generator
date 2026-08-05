#!/usr/bin/env python3
"""Explicitly generate and compare development-only QR matrix fixtures."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import importlib.metadata
import json
import pathlib
import sys
from typing import Callable


ORACLE_PROVENANCE = {
    "nayuki": {
        "distribution": "qrcodegen",
        "tool": "Nayuki QR Code Generator",
        "implementation": "nayuki-qrcodegen-python",
        "version": "1.8.0",
    },
    "python-qrcode": {
        "distribution": "qrcode",
        "tool": "python-qrcode",
        "implementation": "python-qrcode",
        "version": "8.2",
    },
}


def compare_oracle_matrices(
    fixture_id: str,
    first_name: str,
    first_matrix: str,
    second_name: str,
    second_matrix: str,
) -> str:
    if first_matrix != second_matrix:
        difference = "".join(
            difflib.unified_diff(
                first_matrix.splitlines(keepends=True),
                second_matrix.splitlines(keepends=True),
                fromfile=f"{fixture_id}-{first_name}",
                tofile=f"{fixture_id}-{second_name}",
            )
        )
        raise ValueError(f"{fixture_id} oracle matrix disagreement:\n{difference}")
    return first_matrix


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require_pinned_package(distribution: str, expected_version: str) -> str:
    actual_version = importlib.metadata.version(distribution)
    if actual_version != expected_version:
        raise RuntimeError(
            f"expected {distribution} {expected_version}, got {actual_version}"
        )
    return actual_version


def generate_nayuki(payload: bytes, fixture: dict) -> str:
    pin = ORACLE_PROVENANCE["nayuki"]
    require_pinned_package(pin["distribution"], pin["version"])
    from qrcodegen import QrCode, QrSegment

    mode = fixture["mode"]
    if mode == "numeric":
        data_segment = QrSegment.make_numeric(payload.decode("ascii"))
    elif mode == "alphanumeric":
        data_segment = QrSegment.make_alphanumeric(payload.decode("ascii"))
    elif mode == "byte":
        data_segment = QrSegment.make_bytes(payload)
    else:
        raise ValueError(f"unsupported mode {mode}")

    segments = [data_segment]
    if fixture["eci_assignment"] is not None:
        segments.insert(0, QrSegment.make_eci(fixture["eci_assignment"]))
    ecc = {
        "L": QrCode.Ecc.LOW,
        "M": QrCode.Ecc.MEDIUM,
        "Q": QrCode.Ecc.QUARTILE,
        "H": QrCode.Ecc.HIGH,
    }[fixture["ecc"]]
    code = QrCode.encode_segments(
        segments,
        ecc,
        minversion=fixture["version"],
        maxversion=fixture["version"],
        mask=fixture["mask"],
        boostecl=False,
    )
    return "".join(
        "".join("1" if code.get_module(x, y) else "0" for x in range(code.get_size()))
        + "\n"
        for y in range(code.get_size())
    )


def generate_python_qrcode(payload: bytes, fixture: dict) -> str:
    pin = ORACLE_PROVENANCE["python-qrcode"]
    require_pinned_package(pin["distribution"], pin["version"])
    import qrcode
    from qrcode.util import MODE_8BIT_BYTE, MODE_ALPHA_NUM, MODE_NUMBER, QRData

    if fixture["eci_assignment"] is not None:
        raise ValueError("python-qrcode does not expose ECI segments")
    mode = {
        "numeric": MODE_NUMBER,
        "alphanumeric": MODE_ALPHA_NUM,
        "byte": MODE_8BIT_BYTE,
    }[fixture["mode"]]
    ecc = {
        "L": qrcode.constants.ERROR_CORRECT_L,
        "M": qrcode.constants.ERROR_CORRECT_M,
        "Q": qrcode.constants.ERROR_CORRECT_Q,
        "H": qrcode.constants.ERROR_CORRECT_H,
    }[fixture["ecc"]]
    code = qrcode.QRCode(
        version=fixture["version"],
        error_correction=ecc,
        border=0,
        mask_pattern=fixture["mask"],
    )
    code.add_data(QRData(payload, mode=mode, check_data=False), optimize=0)
    code.make(fit=False)
    return "".join(
        "".join("1" if module else "0" for module in row) + "\n"
        for row in code.get_matrix()
    )


GENERATORS: dict[str, Callable[[bytes, dict], str]] = {
    "nayuki": generate_nayuki,
    "python-qrcode": generate_python_qrcode,
}


def fixture_by_id(manifest: dict, fixture_id: str) -> dict:
    matches = [fixture for fixture in manifest["fixtures"] if fixture["id"] == fixture_id]
    if len(matches) != 1:
        raise ValueError(f"expected exactly one fixture named {fixture_id}")
    return matches[0]


def load_payload(manifest_path: pathlib.Path, fixture: dict) -> bytes:
    payload_path = manifest_path.parent / fixture["payload_file"]
    payload = payload_path.read_bytes()
    actual_hash = sha256(payload)
    if actual_hash != fixture["payload_sha256"]:
        raise ValueError(
            f"{fixture['id']} payload hash mismatch: "
            f"expected {fixture['payload_sha256']}, got {actual_hash}"
        )
    return payload


def canonical_command(fixture_id: str, oracle: str) -> str:
    return (
        "uv run --project tests/oracles --locked python "
        "tests/support/generate_fixtures.py "
        f"--fixture {fixture_id} --oracle {oracle}"
    )


def refresh_or_validate_source(fixture: dict, source: dict, refresh: bool) -> None:
    oracle = source.get("oracle")
    if oracle not in GENERATORS:
        raise ValueError(f"{fixture['id']} declares unsupported oracle {oracle!r}")
    pin = ORACLE_PROVENANCE[oracle]
    actual_version = require_pinned_package(pin["distribution"], pin["version"])
    expected = {
        "tool": pin["tool"],
        "implementation": pin["implementation"],
        "version": actual_version,
        "command": canonical_command(fixture["id"], oracle),
    }
    if refresh:
        source.update(expected)
    else:
        mismatches = [key for key, value in expected.items() if source.get(key) != value]
        if mismatches:
            raise ValueError(
                f"{fixture['id']} {oracle} provenance mismatch in {', '.join(mismatches)}"
            )


def generate_declared_sources(
    payload: bytes, fixture: dict, refresh_provenance: bool
) -> tuple[str, list[tuple[dict, str]]]:
    sources = fixture.get("sources", [])
    if len(sources) != 2 or len({source.get("oracle") for source in sources}) != 2:
        raise ValueError(f"{fixture['id']} must declare exactly two distinct oracles")
    generated = []
    for source in sources:
        refresh_or_validate_source(fixture, source, refresh_provenance)
        matrix = GENERATORS[source["oracle"]](payload, fixture)
        generated.append((source, matrix))
    accepted = compare_oracle_matrices(
        fixture["id"],
        generated[0][0]["oracle"],
        generated[0][1],
        generated[1][0]["oracle"],
        generated[1][1],
    )
    return accepted, generated


def check_fixture(manifest_path: pathlib.Path, fixture: dict) -> None:
    payload = load_payload(manifest_path, fixture)
    generated, _ = generate_declared_sources(payload, fixture, False)
    committed_path = manifest_path.parent / fixture["expected_matrix_file"]
    committed = committed_path.read_text(encoding="ascii")
    if generated != committed:
        difference = "".join(
            difflib.unified_diff(
                committed.splitlines(keepends=True),
                generated.splitlines(keepends=True),
                fromfile=f"{fixture['id']}-committed",
                tofile=f"{fixture['id']}-generated",
            )
        )
        raise ValueError(f"{fixture['id']} committed matrix drift:\n{difference}")


def prepare_fixture_write(
    manifest_path: pathlib.Path, fixture: dict
) -> tuple[pathlib.Path, str]:
    payload = load_payload(manifest_path, fixture)
    generated, source_outputs = generate_declared_sources(payload, fixture, True)
    matrix_path = manifest_path.parent / fixture["expected_matrix_file"]
    matrix_hash = sha256(generated.encode("ascii"))
    fixture["expected_matrix_sha256"] = matrix_hash
    for source, source_matrix in source_outputs:
        source["matrix_sha256"] = sha256(source_matrix.encode("ascii"))
    fixture["verification"] = {
        "state": "pending",
        "reviewer": "pending-human-review",
        "verified_at": "pending",
        "notes": "Regenerated by both pinned oracles; review matrix and metadata diff.",
    }
    return matrix_path, generated


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=pathlib.Path,
        default=pathlib.Path("tests/fixtures/manifest.json"),
    )
    parser.add_argument("--fixture", action="append", required=True)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--write", action="store_true")
    action.add_argument("--oracle", choices=sorted(GENERATORS))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    fixtures = [fixture_by_id(manifest, fixture_id) for fixture_id in args.fixture]
    if args.oracle and len(fixtures) != 1:
        raise ValueError("--oracle requires exactly one --fixture")

    if args.oracle:
        fixture = fixtures[0]
        payload = load_payload(args.manifest, fixture)
        source = next(
            (source for source in fixture["sources"] if source.get("oracle") == args.oracle),
            None,
        )
        if source is None:
            raise ValueError(f"{fixture['id']} does not declare oracle {args.oracle}")
        refresh_or_validate_source(fixture, source, False)
        sys.stdout.write(GENERATORS[args.oracle](payload, fixture))
    elif args.check:
        for fixture in fixtures:
            check_fixture(args.manifest, fixture)
    else:
        prepared = [prepare_fixture_write(args.manifest, fixture) for fixture in fixtures]
        for matrix_path, generated in prepared:
            matrix_path.parent.mkdir(parents=True, exist_ok=True)
            matrix_path.write_text(generated, encoding="ascii", newline="\n")
        args.manifest.write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        print("Fixtures regenerated with pending verification; review git diff before acceptance.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
