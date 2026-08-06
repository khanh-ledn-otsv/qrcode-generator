import importlib.util
import pathlib
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("verify_placement_matrices.py")


class PlacementMatrixOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_placement_matrices", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_cases_cover_structural_versions_and_every_mask(self):
        self.assertEqual(tuple(case[0] for case in self.module.CASES), (1, 2, 7, 40))
        self.assertEqual(self.module.MASKS, tuple(range(8)))

    def test_check_rejects_committed_fixture_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "placement_matrices.txt"
            path.write_text("altered\n", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "fixture drift"):
                self.module.check_fixture(path)


if __name__ == "__main__":
    unittest.main()
