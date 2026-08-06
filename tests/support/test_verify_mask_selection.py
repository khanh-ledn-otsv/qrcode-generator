import importlib.util
import pathlib
import unittest


SCRIPT = pathlib.Path(__file__).with_name("verify_mask_selection.py")


class MaskSelectionOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_mask_selection", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_fixture_is_reproducible_and_preserves_disagreeing_totals(self):
        rendered = self.module.render_fixture()
        self.assertEqual(rendered.count("\nformat,"), 32)
        self.assertEqual(rendered.count("\nversion,"), 34)
        self.assertEqual(rendered.count("\ncandidate,"), 24)
        self.assertEqual(rendered.count("\nselected,"), 3)
        self.assertEqual(rendered.count("\nsynthetic,"), 2)
        candidate = next(line for line in rendered.splitlines() if line.startswith("candidate,2,Q,0,"))
        self.assertEqual(candidate.split(",")[4:6], ["387", "1107"])
        self.module.check_fixture()


if __name__ == "__main__":
    unittest.main()
