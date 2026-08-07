import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path


class MutationScoreError(RuntimeError):
    pass


@dataclass(frozen=True)
class MutationScore:
    caught: int
    missed: int
    percent: float


def _matching_lines(path: Path, includes: tuple[str, ...]) -> list[str]:
    if not path.is_file():
        raise MutationScoreError(f"missing cargo-mutants outcome file: {path}")
    lines = [line for line in path.read_text().splitlines() if line]
    if not includes:
        return lines
    return [line for line in lines if any(include in line for include in includes)]


def calculate_score(output: Path, includes: tuple[str, ...]) -> MutationScore:
    caught = _matching_lines(output / "caught.txt", includes)
    missed = _matching_lines(output / "missed.txt", includes)
    timeouts = _matching_lines(output / "timeout.txt", includes)
    if timeouts:
        raise MutationScoreError(f"{len(timeouts)} mutation timeout(s) require explicit triage")
    tested = len(caught) + len(missed)
    if tested == 0:
        raise MutationScoreError("mutation scope contains no viable tested mutants")
    return MutationScore(
        caught=len(caught),
        missed=len(missed),
        percent=100.0 * len(caught) / tested,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--minimum", type=float, required=True)
    parser.add_argument("--include", action="append", default=[])
    parser.add_argument("--evidence", type=Path)
    arguments = parser.parse_args()

    result = calculate_score(arguments.output, tuple(arguments.include))
    print(f"mutation score {result.percent:.2f}% ({result.caught} caught, {result.missed} missed)")
    if arguments.evidence is not None:
        arguments.evidence.parent.mkdir(parents=True, exist_ok=True)
        arguments.evidence.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "minimum_percent": arguments.minimum,
                    "includes": arguments.include,
                    **asdict(result),
                },
                indent=2,
            )
            + "\n"
        )
    if result.percent < arguments.minimum:
        raise MutationScoreError(
            f"mutation score {result.percent:.2f}% is below {arguments.minimum:.2f}%"
        )


if __name__ == "__main__":
    main()
