import tempfile
import unittest
from pathlib import Path

from check_mutation_score import MutationScoreError, calculate_score


class MutationScoreTests(unittest.TestCase):
    def test_score_excludes_unviable_and_enforces_timeout_triage(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "caught.txt").write_text("src/matrix.rs:1\nsrc/matrix.rs:2\n")
            (output / "missed.txt").write_text("src/matrix.rs:3\n")
            (output / "unviable.txt").write_text("src/matrix.rs:4\n")
            (output / "timeout.txt").write_text("")

            result = calculate_score(output, ("src/matrix.rs",))

            self.assertEqual(result.caught, 2)
            self.assertEqual(result.missed, 1)
            self.assertAlmostEqual(result.percent, 66.6666666667)

            (output / "timeout.txt").write_text("src/matrix.rs:5\n")
            with self.assertRaises(MutationScoreError):
                calculate_score(output, ("src/matrix.rs",))

    def test_scope_filter_uses_only_named_critical_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "caught.txt").write_text("src/geometry.rs:1\nsrc/svg.rs:2\n")
            (output / "missed.txt").write_text("src/svg.rs:3\n")
            (output / "timeout.txt").write_text("")

            result = calculate_score(output, ("src/geometry.rs",))

            self.assertEqual((result.caught, result.missed, result.percent), (1, 0, 100.0))


if __name__ == "__main__":
    unittest.main()
