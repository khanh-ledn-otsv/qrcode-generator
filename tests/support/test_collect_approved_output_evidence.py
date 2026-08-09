import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from collect_approved_output_evidence import EvidenceMismatchError, combine_evidence


def row(format_name: str, row_id: str, outcome: str) -> dict[str, object]:
    return {
        "id": row_id,
        "case_kind": "required-payload",
        "profile_index": 1,
        "payload_class": "short-url",
        "version": 6,
        "safety": "caution",
        "logo_geometry": {"obscured_data_modules": 105},
        "artifact": {
            "format": format_name,
            "outcome": outcome,
            "sha256": "a" * 64 if outcome == "decoded" else None,
            "decoder_input_sha256": "b" * 64 if outcome == "decoded" else None,
        },
    }


class ApprovedOutputEvidenceTests(unittest.TestCase):
    def test_matching_png_and_svg_rows_become_one_complete_matrix(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            png = root / "png.json"
            svg = root / "svg.json"
            output = root / "matrix.json"
            png.write_text(
                json.dumps({"schema_version": 1, "rows": [row("png", "one", "decoded")]})
            )
            svg.write_text(
                json.dumps({"schema_version": 1, "rows": [row("svg", "one", "decoded")]})
            )

            matrix = combine_evidence(png, svg, output)

            self.assertEqual(matrix["schema_version"], 2)
            self.assertEqual(set(matrix["rows"][0]["artifacts"]), {"png", "svg"})
            self.assertEqual(json.loads(output.read_text()), matrix)

    def test_mismatched_scenario_metadata_is_rejected(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            png = root / "png.json"
            svg = root / "svg.json"
            svg_row = row("svg", "one", "decoded")
            svg_row["version"] = 7
            png.write_text(
                json.dumps({"schema_version": 1, "rows": [row("png", "one", "decoded")]})
            )
            svg.write_text(json.dumps({"schema_version": 1, "rows": [svg_row]}))

            with self.assertRaises(EvidenceMismatchError):
                combine_evidence(png, svg, root / "matrix.json")


if __name__ == "__main__":
    unittest.main()
