import importlib.util
import pathlib
import unittest
from unittest import mock


SCRIPT = pathlib.Path(__file__).with_name("generate_fixtures.py")


class DualOracleComparisonTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("generate_fixtures", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_accepts_identical_human_readable_matrices(self):
        matrix = "01\n10\n"

        self.assertEqual(
            self.module.compare_oracle_matrices(
                "fixture-01", "nayuki", matrix, "python-qrcode", matrix
            ),
            matrix,
        )

    def test_rejects_oracle_disagreement_with_a_unified_diff(self):
        with self.assertRaisesRegex(
            ValueError,
            r"(?s)fixture-01.*--- fixture-01-nayuki.*\+\+\+ fixture-01-python-qrcode",
        ):
            self.module.compare_oracle_matrices(
                "fixture-01",
                "nayuki",
                "01\n10\n",
                "python-qrcode",
                "01\n11\n",
            )

    def test_rejects_declared_provenance_that_does_not_match_the_oracle_pin(self):
        fixture = {"id": "fixture-01"}
        source = {
            "oracle": "python-qrcode",
            "tool": "different tool",
            "implementation": "python-qrcode",
            "version": "8.2",
            "command": self.module.canonical_command("fixture-01", "python-qrcode"),
        }

        with mock.patch.object(
            self.module, "require_pinned_package", return_value="8.2"
        ):
            with self.assertRaisesRegex(ValueError, "provenance mismatch in tool"):
                self.module.refresh_or_validate_source(fixture, source, False)

    def test_regeneration_refreshes_provenance_from_the_executed_oracle(self):
        fixture = {"id": "fixture-01"}
        source = {"oracle": "python-qrcode"}

        with mock.patch.object(
            self.module, "require_pinned_package", return_value="8.2"
        ):
            self.module.refresh_or_validate_source(fixture, source, True)

        self.assertEqual(source["tool"], "python-qrcode")
        self.assertEqual(source["version"], "8.2")
        self.assertEqual(
            source["command"],
            "uv run --project tests/oracles --locked python "
            "tests/support/generate_fixtures.py "
            "--fixture fixture-01 --oracle python-qrcode",
        )


if __name__ == "__main__":
    unittest.main()
