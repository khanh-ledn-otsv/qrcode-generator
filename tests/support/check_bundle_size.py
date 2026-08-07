import argparse
import gzip
import json
from pathlib import Path


class BundleSizeError(RuntimeError):
    pass


def compressed_size(wasm: Path) -> int:
    return len(gzip.compress(wasm.read_bytes(), compresslevel=9, mtime=0))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dist", type=Path)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("--evidence", type=Path, required=True)
    arguments = parser.parse_args()

    wasm_files = sorted(arguments.dist.glob("*.wasm"))
    if len(wasm_files) != 1:
        raise BundleSizeError(
            f"expected exactly one release WASM artifact; found {len(wasm_files)}"
        )
    maximum = json.loads(arguments.baseline.read_text())["wasm_release"]["maximum_gzip_bytes"]
    observed = compressed_size(wasm_files[0])
    arguments.evidence.parent.mkdir(parents=True, exist_ok=True)
    arguments.evidence.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "artifact": wasm_files[0].name,
                "compression": "gzip-level-9-mtime-0",
                "observed_bytes": observed,
                "maximum_bytes": maximum,
            },
            indent=2,
        )
        + "\n"
    )
    if observed > maximum:
        raise BundleSizeError(f"compressed WASM is {observed} bytes; baseline maximum is {maximum}")
    print(f"compressed WASM: {observed} bytes (maximum {maximum})")


if __name__ == "__main__":
    main()
