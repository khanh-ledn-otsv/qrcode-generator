import argparse
import json
from pathlib import Path
from typing import Any


class EvidenceMismatchError(RuntimeError):
    pass


def _rows(path: Path, expected_format: str) -> dict[str, tuple[dict[str, Any], dict[str, Any]]]:
    document = json.loads(path.read_text())
    if document.get("schema_version") != 1 or not isinstance(document.get("rows"), list):
        raise EvidenceMismatchError(f"{path} is not format evidence schema 1")
    indexed: dict[str, tuple[dict[str, Any], dict[str, Any]]] = {}
    for value in document["rows"]:
        if not isinstance(value, dict) or not isinstance(value.get("id"), str):
            raise EvidenceMismatchError(f"{path} contains an invalid scenario row")
        common = dict(value)
        artifact = common.pop("artifact", None)
        if not isinstance(artifact, dict) or artifact.get("format") != expected_format:
            raise EvidenceMismatchError(f"{value['id']} has the wrong artifact format")
        artifact = dict(artifact)
        artifact.pop("format")
        if value["id"] in indexed:
            raise EvidenceMismatchError(f"{path} repeats scenario {value['id']}")
        indexed[value["id"]] = (common, artifact)
    return indexed


def combine_evidence(png_path: Path, svg_path: Path, output: Path) -> dict[str, Any]:
    png = _rows(png_path, "png")
    svg = _rows(svg_path, "svg")
    if png.keys() != svg.keys():
        raise EvidenceMismatchError("PNG and SVG evidence cover different scenarios")

    rows = []
    for row_id in sorted(png):
        png_common, png_artifact = png[row_id]
        svg_common, svg_artifact = svg[row_id]
        if png_common != svg_common:
            raise EvidenceMismatchError(f"PNG and SVG metadata differ for {row_id}")
        if png_artifact.get("outcome") != svg_artifact.get("outcome"):
            raise EvidenceMismatchError(f"PNG and SVG outcomes differ for {row_id}")
        rows.append(
            {
                **png_common,
                "artifacts": {"png": png_artifact, "svg": svg_artifact},
            }
        )

    matrix = {"schema_version": 2, "rows": rows}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(matrix, indent=2) + "\n")
    return matrix


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--png", type=Path, required=True)
    parser.add_argument("--svg", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    combine_evidence(arguments.png, arguments.svg, arguments.output)


if __name__ == "__main__":
    main()
