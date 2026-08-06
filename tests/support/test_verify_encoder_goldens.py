import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).with_name("verify_encoder_goldens.py")


class EncoderGoldenOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_encoder_goldens", SCRIPT)
        if spec is None or spec.loader is None:
            raise RuntimeError("could not load support module")
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_fixture_covers_versions_ecc_modes_eci_and_masks(self):
        rendered = self.module.render_fixture()
        rows = [line.split(",") for line in rendered.splitlines() if not line.startswith("#")][1:]
        self.assertTrue({1, 2, 6, 7, 9, 10, 26, 27, 40}.issubset({int(row[3]) for row in rows}))
        self.assertEqual({"L", "M", "Q", "H"}, {row[2] for row in rows})
        self.assertTrue(
            {"numeric", "alphanumeric", "byte", "utf8"}.issubset({row[1] for row in rows})
        )
        self.assertIn("26", {row[5] for row in rows})
        self.assertEqual(set(range(8)), {int(row[4]) for row in rows})
        self.module.check_fixture()


if __name__ == "__main__":
    unittest.main()
