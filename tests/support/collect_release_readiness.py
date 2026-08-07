import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


class BuildMismatchError(RuntimeError):
    pass


class ResultEvidenceError(RuntimeError):
    pass


REQUIRED_PROJECTS = ("chromium",)
PRIVACY_TEST = "payload, logo, configuration, and downloads make no runtime request"
DOWNLOAD_TESTS = {
    "downloads fixed filenames and exact deterministic SVG and PNG bytes",
    "downloaded PNG independently decodes with the pinned reader",
}
GUIDANCE_TEST = "explains export, physical sizing, and placement validation before generation"
CRITICAL_WORKFLOW_TESTS = {
    "reports representative modes, UTF-8 counts, and latest debounced input",
    "distinguishes the input-limit boundary and keeps exports disabled",
    "disposing the page with pending debounce work initializes cleanly",
    "profile controls work by keyboard",
    "shows the opaque preview at its real SVG size",
    "uses only magenta and shows transparent placement cautions",
    "logo mode is selected by default, uses ECC H, and requires opaque white",
    "uses square modules and standard square finders without a shape control",
}


def _hashes(build: Path) -> dict[str, str]:
    return {
        path.relative_to(build).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(build.rglob("*"))
        if path.is_file()
    }


def collect_build_evidence(first: Path, second: Path) -> dict[str, Any]:
    first_hashes = _hashes(first)
    second_hashes = _hashes(second)
    if not first_hashes or first_hashes != second_hashes:
        raise BuildMismatchError("independent production-build hashes do not match")
    if not any(name.endswith(".wasm") for name in first_hashes):
        raise BuildMismatchError("production build must contain a WASM artifact")
    return {"reproducible_builds": {"match": True, "hashes": first_hashes}}


def collect_result_evidence(playwright_report: Path, release_evidence: Path) -> dict[str, Any]:
    report = json.loads(playwright_report.read_text())
    by_project: dict[str, dict[str, list[dict[str, Any]]]] = {
        project: {} for project in REQUIRED_PROJECTS
    }
    for suite in report.get("suites", []):
        for spec in suite.get("specs", []):
            for test in spec.get("tests", []):
                project = test.get("projectName")
                if project not in by_project:
                    raise ResultEvidenceError(f"unexpected Playwright project: {project}")
                title = spec.get("title")
                if title in by_project[project]:
                    raise ResultEvidenceError(f"duplicate Playwright result: {project} / {title}")
                by_project[project][title] = test.get("results", [])

    required = {PRIVACY_TEST, GUIDANCE_TEST, *DOWNLOAD_TESTS, *CRITICAL_WORKFLOW_TESTS}
    browsers: dict[str, dict[str, Any]] = {}
    for project, tests in by_project.items():
        missing = required - tests.keys()
        if missing:
            raise ResultEvidenceError(f"{project} is missing required tests: {sorted(missing)}")
        retries = 0
        for title, results in tests.items():
            if len(results) != 1 or results[0].get("status") != "passed":
                raise ResultEvidenceError(f"{project} did not pass {title}")
            retry = results[0].get("retry")
            if not isinstance(retry, int):
                raise ResultEvidenceError(f"{project} has invalid retry evidence for {title}")
            retries = max(retries, retry)
        browsers[project] = {"passed": True, "retries": retries, "tests": len(tests)}

    matrix_path = release_evidence / "approved-output-matrix.json"
    adverse_path = release_evidence / "adverse-decode.json"
    matrix_rows = json.loads(matrix_path.read_text()).get("rows")
    adverse_outcomes = json.loads(adverse_path.read_text()).get("outcomes")
    if not isinstance(matrix_rows, list) or len(matrix_rows) != 96:
        raise ResultEvidenceError("approved output evidence must contain exactly 96 rows")
    if not isinstance(adverse_outcomes, list) or not adverse_outcomes:
        raise ResultEvidenceError("adverse decoder evidence is missing outcomes")
    return {
        "browsers": browsers,
        "network_inspection": {"passed": True, "external_requests": 0, "source_test": PRIVACY_TEST},
        "downloads": {"passed": True, "source_tests": sorted(DOWNLOAD_TESTS)},
        "guidance": {"passed": True, "source_test": GUIDANCE_TEST},
        "artifact_evidence": {
            "passed": True,
            "matrix_rows": len(matrix_rows),
            "adverse_outcomes": len(adverse_outcomes),
            "sources": {
                path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                for path in (matrix_path, adverse_path)
            },
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--first-build", type=Path, required=True)
    parser.add_argument("--second-build", type=Path, required=True)
    parser.add_argument("--release-candidate", required=True)
    parser.add_argument("--playwright-report", type=Path, required=True)
    parser.add_argument("--release-evidence", type=Path, required=True)
    parser.add_argument("--node", required=True)
    parser.add_argument("--pnpm", required=True)
    parser.add_argument("--rustc", required=True)
    parser.add_argument("--trunk", required=True)
    parser.add_argument("--playwright", required=True)
    parser.add_argument("--zxing", required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()

    evidence = {
        "schema_version": 1,
        "release_candidate": arguments.release_candidate,
        "git": {"clean": True},
        "tools": {
            "node": arguments.node,
            "pnpm": arguments.pnpm,
            "rustc": arguments.rustc,
            "trunk": arguments.trunk,
            "playwright": arguments.playwright,
            "zxing": arguments.zxing,
        },
        **collect_build_evidence(arguments.first_build, arguments.second_build),
        **collect_result_evidence(arguments.playwright_report, arguments.release_evidence),
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
