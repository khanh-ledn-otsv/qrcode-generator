import importlib.util
import pathlib
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("verify_reed_solomon.py")


class ReedSolomonOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_reed_solomon", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_cases_cover_every_degree_and_the_maximum_qr_block(self):
        cases = self.module.cases()
        self.assertEqual(sorted({degree for degree, _, _ in cases}), list(self.module.DEGREES))
        self.assertIn((30, bytes(range(123)), "maximum-qr-data-block"), cases)

    def test_check_rejects_committed_fixture_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "reed_solomon.csv"
            path.write_text("altered\n", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "fixture drift"):
                self.module.check_fixture(path)


if __name__ == "__main__":
    unittest.main()
