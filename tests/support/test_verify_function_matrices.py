import importlib.util
import pathlib
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name("verify_function_matrices.py")


class FunctionMatrixOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_function_matrices", SCRIPT)
        if spec is None or spec.loader is None:
            raise RuntimeError("could not load support module")
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_cases_cover_human_review_versions(self):
        self.assertEqual(self.module.VERSIONS, (1, 2, 7, 40))

    def test_check_rejects_committed_fixture_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "function_matrices.txt"
            path.write_text("altered\n", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "fixture drift"):
                self.module.check_fixture(path)


if __name__ == "__main__":
    unittest.main()
