import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


class CoverageError(RuntimeError):
    pass


@dataclass(frozen=True)
class Coverage:
    line_percent: float
    region_percent: float


def _percent(covered: int, count: int, metric: str) -> float:
    if count == 0:
        raise CoverageError(f"coverage scope contains no {metric}")
    return 100.0 * covered / count


def coverage_for_scope(report: dict[str, Any], suffixes: tuple[str, ...]) -> Coverage:
    try:
        files = report["data"][0]["files"]
    except (KeyError, IndexError, TypeError) as error:
        raise CoverageError("invalid llvm-cov JSON report") from error
    matching = [
        file
        for file in files
        if not suffixes or any(str(file["filename"]).endswith(suffix) for suffix in suffixes)
    ]
    if not matching:
        raise CoverageError("coverage report contains none of the requested files")

    line_count = sum(file["summary"]["lines"]["count"] for file in matching)
    line_covered = sum(file["summary"]["lines"]["covered"] for file in matching)
    region_count = sum(file["summary"]["regions"]["count"] for file in matching)
    region_covered = sum(file["summary"]["regions"]["covered"] for file in matching)
    return Coverage(
        line_percent=_percent(line_covered, line_count, "lines"),
        region_percent=_percent(region_covered, region_count, "regions"),
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("--minimum-lines", type=float, required=True)
    parser.add_argument("--minimum-regions", type=float, required=True)
    parser.add_argument("--include", action="append", default=[])
    parser.add_argument("--evidence", type=Path)
    arguments = parser.parse_args()

    report = json.loads(arguments.report.read_text())
    coverage = coverage_for_scope(report, tuple(arguments.include))
    print(
        f"line coverage {coverage.line_percent:.2f}%; "
        f"region coverage {coverage.region_percent:.2f}%"
    )
    if arguments.evidence is not None:
        arguments.evidence.parent.mkdir(parents=True, exist_ok=True)
        arguments.evidence.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "minimum_lines": arguments.minimum_lines,
                    "minimum_regions": arguments.minimum_regions,
                    "includes": arguments.include,
                    **asdict(coverage),
                },
                indent=2,
            )
            + "\n"
        )
    if coverage.line_percent < arguments.minimum_lines:
        raise CoverageError(
            f"line coverage {coverage.line_percent:.2f}% is below {arguments.minimum_lines:.2f}%"
        )
    if coverage.region_percent < arguments.minimum_regions:
        raise CoverageError(
            f"region coverage {coverage.region_percent:.2f}% is below "
            f"{arguments.minimum_regions:.2f}%"
        )


if __name__ == "__main__":
    main()
