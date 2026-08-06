import importlib.util
import pathlib
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("verify_interleaved_codewords.py")


class InterleavedCodewordOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_interleaved_codewords", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_cases_cover_one_group_and_two_group_layouts(self):
        self.assertEqual(
            self.module.CASES,
            ((1, "M", "one-group"), (5, "Q", "two-group-short-long")),
        )

    def test_check_rejects_committed_fixture_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "interleaved_codewords.csv"
            path.write_text("altered\n", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "fixture drift"):
                self.module.check_fixture(path)


if __name__ == "__main__":
    unittest.main()
