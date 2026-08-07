import tempfile
import unittest
from pathlib import Path

from check_bundle_size import compressed_size


class BundleSizeTests(unittest.TestCase):
    def test_compression_is_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / "app.wasm"
            artifact.write_bytes(b"synthetic wasm bytes" * 100)

            self.assertEqual(compressed_size(artifact), 57)
            self.assertEqual(compressed_size(artifact), 57)


if __name__ == "__main__":
    unittest.main()
