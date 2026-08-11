"""Verify the reviewed mixed-segment fixtures with two pinned QR encoders."""

from __future__ import annotations

import hashlib
import importlib.metadata
import importlib.util
import json
import pathlib

FIXTURE = pathlib.Path(__file__).parents[2] / "docs/generated/mixed-mode-oracle-fixtures.json"
MASK_REFERENCE = pathlib.Path(__file__).with_name("verify_mask_selection.py")
COMMAND = (
    "uv run --project tests/oracles --locked python tests/support/verify_mixed_mode_oracles.py"
)


def load_mask_reference():
    spec = importlib.util.spec_from_file_location("mixed_mask_reference", MASK_REFERENCE)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load the independent mask reference")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def require_pin(distribution: str, version: str) -> None:
    actual = importlib.metadata.version(distribution)
    if actual != version:
        raise RuntimeError(f"expected {distribution} {version}, got {actual}")


def fingerprint(matrix: str) -> str:
    value = 0xCBF29CE484222325
    for byte in matrix.encode("ascii"):
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def nayuki_matrix(case: dict, mask: int) -> tuple[str, int]:
    from qrcodegen import QrCode, QrSegment

    segments = []
    if case["eci_assignment"] is not None:
        segments.append(QrSegment.make_eci(case["eci_assignment"]))
    payload = case["payload"]
    byte_count = case["segments"][-2 if case["segments"][-1]["mode"] == "numeric" else -1][
        "byte_count"
    ]
    prefix = payload.encode("utf-8")[:byte_count]
    digits = payload[len(prefix.decode("utf-8")) :]
    segments.extend([QrSegment.make_bytes(prefix), QrSegment.make_numeric(digits)])
    data_bits_used = QrSegment.get_total_bits(segments, case["selected_version"])
    if data_bits_used is None:
        raise ValueError(f"{case['id']} segment lengths do not fit the selected version")
    qr = QrCode.encode_segments(
        segments,
        QrCode.Ecc.LOW,
        minversion=case["selected_version"],
        maxversion=case["selected_version"],
        mask=mask,
        boostecl=False,
    )
    matrix = "".join(
        "".join("1" if qr.get_module(x, y) else "0" for x in range(qr.get_size())) + "\n"
        for y in range(qr.get_size())
    )
    return matrix, data_bits_used


def python_matrix(case: dict, mask: int) -> tuple[str, int, str]:
    import qrcode
    import qrcode.base
    import qrcode.util
    from qrcode.util import MODE_8BIT_BYTE, MODE_NUMBER, QRData

    version = case["selected_version"]
    buffer = qrcode.util.BitBuffer()
    if case["eci_assignment"] is not None:
        buffer.put(0b0111, 4)
        buffer.put(case["eci_assignment"], 8)
    payload = case["payload"]
    byte_count = case["segments"][-2]["byte_count"]
    prefix = payload.encode("utf-8")[:byte_count]
    digits = payload[len(prefix.decode("utf-8")) :].encode("ascii")
    for data, mode in [(prefix, MODE_8BIT_BYTE), (digits, MODE_NUMBER)]:
        segment = QRData(data, mode=mode, check_data=False)
        buffer.put(mode, 4)
        buffer.put(len(segment), qrcode.util.length_in_bits(mode, version))
        segment.write(buffer)
    data_bits_used = len(buffer)
    blocks = qrcode.base.rs_blocks(version, qrcode.constants.ERROR_CORRECT_L)
    bit_limit = sum(block.data_count for block in blocks) * 8
    for _ in range(min(bit_limit - len(buffer), 4)):
        buffer.put_bit(False)
    while len(buffer) % 8:
        buffer.put_bit(False)
    pad = 0
    while len(buffer) < bit_limit:
        buffer.put(0xEC if pad % 2 == 0 else 0x11, 8)
        pad += 1
    data_codewords_hex = "".join(f"{value:02X}" for value in buffer.buffer)
    code = qrcode.QRCode(
        version=version,
        error_correction=qrcode.constants.ERROR_CORRECT_L,
        border=0,
        mask_pattern=mask,
    )
    code.data_cache = qrcode.util.create_bytes(buffer, blocks)
    code.makeImpl(False, mask)
    matrix = "".join(
        "".join("1" if module else "0" for module in row) + "\n" for row in code.get_matrix()
    )
    return matrix, data_bits_used, data_codewords_hex


def verify_fixture() -> None:
    require_pin("qrcodegen", "1.8.0")
    require_pin("qrcode", "8.2")
    document = json.loads(FIXTURE.read_text(encoding="utf-8"))
    if document["generation_command"] != COMMAND:
        raise ValueError("mixed-mode fixture generation command drift")
    reference = load_mask_reference()
    for case in document["cases"]:
        if hashlib.sha256(case["payload"].encode()).hexdigest() != case["payload_sha256"]:
            raise ValueError(f"{case['id']} payload hash drift")
        candidates = []
        for mask in range(8):
            nayuki, nayuki_bits = nayuki_matrix(case, mask)
            python, python_bits, data_codewords_hex = python_matrix(case, mask)
            if nayuki != python:
                raise ValueError(f"{case['id']} mask {mask} oracle matrix disagreement")
            if nayuki_bits != case["data_bits_used"] or python_bits != case["data_bits_used"]:
                raise ValueError(f"{case['id']} data bit count drift")
            if data_codewords_hex != case["data_codewords_hex"]:
                raise ValueError(f"{case['id']} data codeword drift")
            rows = [[module == "1" for module in row] for row in nayuki.splitlines()]
            candidates.append((reference.reference_penalty(rows), nayuki))
        selected = min(range(8), key=lambda mask: candidates[mask][0])
        if selected != case["selected_mask"]:
            raise ValueError(f"{case['id']} selected mask drift")
        if fingerprint(candidates[selected][1]) != case["matrix_fnv1a64"]:
            raise ValueError(f"{case['id']} completed matrix drift")


def main() -> None:
    verify_fixture()


if __name__ == "__main__":
    main()
