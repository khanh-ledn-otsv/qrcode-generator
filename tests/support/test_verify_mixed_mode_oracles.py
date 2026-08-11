import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).with_name("verify_mixed_mode_oracles.py")


class MixedModeOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("verify_mixed_mode_oracles", SCRIPT)
        if spec is None or spec.loader is None:
            raise RuntimeError("could not load mixed-mode oracle verifier")
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_pinned_generators_agree_on_every_explicit_mask_and_selected_matrix(self):
        self.module.verify_fixture()


if __name__ == "__main__":
    unittest.main()
