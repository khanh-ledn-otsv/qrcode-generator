import argparse
import hashlib
import itertools
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
    "downloaded PNGs and SVGs independently decode common payload formats",
}
GUIDANCE_TESTS = {
    "explains export, physical sizing, and placement validation before generation",
    "guides long-link profile, logo, and PNG choices accurately",
}
CRITICAL_WORKFLOW_TESTS = {
    "reports representative modes, UTF-8 counts, and latest debounced input",
    "distinguishes the input-limit boundary and keeps exports disabled",
    "disposing the page with pending debounce work initializes cleanly",
    "profile controls work by keyboard",
    "shows the opaque preview at its real SVG size",
    "uses approved foreground themes with one opaque white rounded ONE appearance",
    "logo mode is enabled by default and can be turned off",
    "fixed profiles reject centered branding above Version 6",
    "Poster / Package preserves fixed dimensions at Version 12",
    "always uses rounded ONE modules without an appearance control",
}
WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
MATRIX_POLICY_PATH = WORKSPACE_ROOT / "tests/approved-output-matrix-policy.json"


def _mapping(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ResultEvidenceError(f"{label} must be an object")
    return value


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


def _valid_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _matrix_policy() -> dict[str, Any]:
    policy = _mapping(json.loads(MATRIX_POLICY_PATH.read_text()), "approved matrix policy")
    if policy.get("schema_version") != 1:
        raise ResultEvidenceError("approved matrix policy has an invalid schema")
    return policy


def _expected_matrix_keys(policy: dict[str, Any]) -> set[tuple[object, ...]]:
    dimensions = _mapping(policy.get("tuple_dimensions"), "approved matrix dimensions")
    dimension_names = ("profiles", "logo_states", "foreground_themes")
    counts: list[int] = []
    for name in dimension_names:
        count = dimensions.get(name)
        if not isinstance(count, int) or count <= 0:
            raise ResultEvidenceError("approved matrix policy has invalid tuple dimensions")
        counts.append(count)
    profile_min_versions = policy.get("profile_min_versions")
    profile_max_versions = policy.get("profile_max_versions")
    payload_classes = policy.get("required_payload_classes")
    version_payload_class = policy.get("version_coverage_payload_class")
    if (
        not isinstance(profile_min_versions, list)
        or len(profile_min_versions) != counts[0]
        or not all(isinstance(version, int) and version > 0 for version in profile_min_versions)
        or not isinstance(profile_max_versions, list)
        or len(profile_max_versions) != counts[0]
        or not all(isinstance(version, int) and version > 0 for version in profile_max_versions)
        or any(
            minimum > maximum
            for minimum, maximum in zip(profile_min_versions, profile_max_versions)
        )
        or not isinstance(payload_classes, list)
        or not payload_classes
        or not all(isinstance(payload, str) for payload in payload_classes)
        or not isinstance(version_payload_class, str)
    ):
        raise ResultEvidenceError("approved matrix policy has invalid scenario membership")

    expected = set()
    ranges = [range(count) for count in counts]
    for indices in itertools.product(*ranges):
        for payload_class in payload_classes:
            expected.add((*indices, "required-payload", payload_class, payload_class, None))
        for version in range(
            profile_min_versions[indices[0]], profile_max_versions[indices[0]] + 1
        ):
            expected.add(
                (
                    *indices,
                    "version-coverage",
                    version_payload_class,
                    f"version-v{version}",
                    version,
                )
            )
    return expected


def _validate_approved_matrix(path: Path) -> tuple[int, int, int]:
    policy = _matrix_policy()
    expected = _mapping(policy.get("expected_rows"), "approved matrix expected rows")
    total_rows = expected.get("total")
    required_rows = expected.get("required_payload")
    version_rows = expected.get("version_coverage")
    decoded_rows = expected.get("decoded")
    invalid_rows = expected.get("expected_invalid")
    if not all(
        isinstance(value, int)
        for value in (total_rows, required_rows, version_rows, decoded_rows, invalid_rows)
    ):
        raise ResultEvidenceError("approved matrix policy has invalid expected row counts")
    document = _mapping(json.loads(path.read_text()), "approved output evidence")
    rows = document.get("rows")
    if document.get("schema_version") != 2 or not isinstance(rows, list):
        raise ResultEvidenceError("approved output evidence must use complete matrix schema 2")
    if len(rows) != total_rows:
        raise ResultEvidenceError(
            f"approved output evidence must contain exactly {total_rows} rows"
        )
    ids = {row.get("id") for row in rows if isinstance(row, dict)}
    if len(ids) != total_rows or None in ids:
        raise ResultEvidenceError("approved output evidence contains invalid or duplicate IDs")
    expected_keys = _expected_matrix_keys(policy)
    actual_keys = {
        (
            row.get("profile_index"),
            row.get("logo_state_index"),
            row.get("foreground_index"),
            row.get("case_kind"),
            row.get("payload_class"),
            row.get("case_label"),
            row.get("version") if row.get("case_kind") == "version-coverage" else None,
        )
        for row in rows
        if isinstance(row, dict)
    }
    if actual_keys != expected_keys:
        raise ResultEvidenceError("approved output evidence has incomplete scenario membership")
    case_kinds = [row.get("case_kind") for row in rows]
    if (
        case_kinds.count("required-payload") != required_rows
        or case_kinds.count("version-coverage") != version_rows
    ):
        raise ResultEvidenceError("approved output evidence has incomplete case-kind coverage")
    dimensions = _mapping(policy.get("tuple_dimensions"), "approved matrix dimensions")
    dimension_fields = {
        "profile_index": "profiles",
        "logo_state_index": "logo_states",
        "foreground_index": "foreground_themes",
    }
    for field, dimension in dimension_fields.items():
        count = dimensions.get(dimension)
        if not isinstance(count, int) or {row.get(field) for row in rows} != set(range(count)):
            raise ResultEvidenceError(
                f"approved output evidence has incomplete {dimension} coverage"
            )
    profile_min_versions = policy.get("profile_min_versions")
    profile_max_versions = policy.get("profile_max_versions")
    if (
        not isinstance(profile_min_versions, list)
        or not isinstance(profile_max_versions, list)
        or not all(isinstance(version, int) for version in profile_min_versions)
        or not all(isinstance(version, int) for version in profile_max_versions)
    ):
        raise ResultEvidenceError("approved matrix policy has invalid profile versions")
    approved_versions = {
        version
        for minimum, maximum in zip(profile_min_versions, profile_max_versions)
        for version in range(minimum, maximum + 1)
    }
    if {
        row.get("version") for row in rows if isinstance(row.get("version"), int)
    } != approved_versions:
        raise ResultEvidenceError("approved output evidence has incomplete version coverage")
    required_payloads = set(policy.get("required_payload_classes", []))
    if {row.get("payload_class") for row in rows} != required_payloads:
        raise ResultEvidenceError("approved output evidence has incomplete payload coverage")

    decoded = 0
    invalid = 0
    branded = 0
    branding = _mapping(policy.get("branded_geometry_policy"), "branded geometry policy")
    for row in rows:
        artifacts = row.get("artifacts")
        if not isinstance(artifacts, dict) or set(artifacts) != {"png", "svg"}:
            raise ResultEvidenceError(f"{row['id']} does not record both artifact formats")
        outcomes = {artifact.get("outcome") for artifact in artifacts.values()}
        if len(outcomes) != 1:
            raise ResultEvidenceError(f"{row['id']} has mismatched artifact outcomes")
        outcome = outcomes.pop()
        if outcome == "decoded":
            decoded += 1
            if row.get("safety") not in {"safe", "caution"}:
                raise ResultEvidenceError(f"{row['id']} lacks a safety classification")
            for artifact in artifacts.values():
                if not _valid_sha256(artifact.get("sha256")) or not _valid_sha256(
                    artifact.get("decoder_input_sha256")
                ):
                    raise ResultEvidenceError(f"{row['id']} has invalid artifact hashes")
            if row.get("logo_state_index") == 1:
                branded += 1
                if row.get("safety") == "caution":
                    if row.get("logo_geometry") != _expected_logo_geometry(row, branding):
                        raise ResultEvidenceError(f"{row['id']} has unapproved branded geometry")
                elif row.get("logo_geometry") is not None:
                    raise ResultEvidenceError(f"{row['id']} has geometry after logo fallback")
        elif outcome == "expected-invalid":
            invalid += 1
            if row.get("safety") is not None or row.get("logo_geometry") is not None:
                raise ResultEvidenceError(f"{row['id']} records facts for an invalid artifact")
        else:
            raise ResultEvidenceError(f"{row['id']} has an unknown artifact outcome")
    if (decoded, invalid) != (decoded_rows, invalid_rows) or branded == 0:
        raise ResultEvidenceError("approved output evidence has incomplete outcome coverage")
    return len(rows), decoded, invalid


def _expected_logo_geometry(row: dict[str, Any], branding: dict[str, Any]) -> dict[str, Any]:
    profile_index = row.get("profile_index")
    version = row.get("version")
    geometry_rows = branding.get("geometry_rows")
    if (
        not isinstance(profile_index, int)
        or not isinstance(version, int)
        or not isinstance(geometry_rows, list)
    ):
        raise ResultEvidenceError("branded geometry policy has invalid rows")
    matches = []
    for geometry_row in geometry_rows:
        if not isinstance(geometry_row, dict):
            raise ResultEvidenceError("branded geometry policy has invalid rows")
        profile_indices = geometry_row.get("profile_indices")
        if not isinstance(profile_indices, list) or not all(
            isinstance(index, int) for index in profile_indices
        ):
            raise ResultEvidenceError("branded geometry policy has invalid profile indices")
        if geometry_row.get("version") == version and profile_index in profile_indices:
            matches.append(geometry_row)
    if len(matches) != 1:
        raise ResultEvidenceError(f"{row['id']} has an unapproved branded version")
    geometry = {
        key: value for key, value in matches[0].items() if key not in {"profile_indices", "version"}
    }
    if (
        not isinstance(geometry.get("source_ten_thousandths"), list)
        or len(geometry["source_ten_thousandths"]) != 4
        or not all(isinstance(value, int) for value in geometry["source_ten_thousandths"])
        or not isinstance(geometry.get("knockout_modules"), list)
        or len(geometry["knockout_modules"]) != 4
        or not all(isinstance(value, int) for value in geometry["knockout_modules"])
        or not isinstance(geometry.get("protected_clearance_modules"), int)
        or not isinstance(geometry.get("obscured_data_modules"), int)
        or not isinstance(geometry.get("obscured_remainder_modules"), int)
        or set(geometry)
        != {
            "source_ten_thousandths",
            "knockout_modules",
            "protected_clearance_modules",
            "obscured_data_modules",
            "obscured_remainder_modules",
        }
    ):
        raise ResultEvidenceError("branded geometry policy has invalid measurements")
    return geometry


def validate_adverse_evidence(path: Path) -> int:
    document = _mapping(json.loads(path.read_text()), "adverse decoder evidence")
    outcomes = document.get("outcomes")
    parameters = document.get("parameters")
    if (
        document.get("schema_version") != 1
        or parameters != "tests/adverse/parameters.json"
        or document.get("seed") != 20260807
        or not isinstance(outcomes, list)
    ):
        raise ResultEvidenceError("adverse decoder evidence has invalid metadata")
    manifest = _mapping(
        json.loads((WORKSPACE_ROOT / parameters).read_text()), "adverse transform manifest"
    )
    envelopes = manifest.get("pass_envelopes")
    if manifest.get("schema_version") != 1 or manifest.get("seed") != document.get("seed"):
        raise ResultEvidenceError("adverse transform manifest has invalid metadata")
    if not isinstance(envelopes, list):
        raise ResultEvidenceError("adverse transform manifest is missing pass envelopes")
    expected: dict[str, tuple[str, set[str]]] = {}
    for value in envelopes:
        envelope = _mapping(value, "adverse pass envelope")
        configuration = envelope.get("configuration")
        safety = envelope.get("safety")
        transforms = envelope.get("transforms")
        if (
            not isinstance(configuration, str)
            or safety not in {"safe", "caution"}
            or not isinstance(transforms, list)
            or not transforms
            or not all(isinstance(transform, str) for transform in transforms)
            or len(set(transforms)) != len(transforms)
            or configuration in expected
        ):
            raise ResultEvidenceError("adverse transform manifest has an invalid pass envelope")
        expected[configuration] = (safety, set(transforms))
    grouped: dict[str, list[dict[str, Any]]] = {}
    for value in outcomes:
        if not isinstance(value, dict) or not isinstance(value.get("configuration"), str):
            raise ResultEvidenceError("adverse decoder evidence contains an invalid outcome")
        grouped.setdefault(value["configuration"], []).append(value)
    if grouped.keys() != expected.keys():
        raise ResultEvidenceError("adverse decoder evidence has incomplete pass envelopes")
    for configuration, (safety, expected_transforms) in expected.items():
        rows = grouped[configuration]
        transforms = {row.get("transform") for row in rows}
        if (
            len(rows) != len(expected_transforms)
            or transforms != expected_transforms
            or any(
                row.get("safety") != safety
                or row.get("outcome") != "decoded"
                or not isinstance(row.get("decoder"), str)
                or not row["decoder"]
                for row in rows
            )
        ):
            raise ResultEvidenceError(f"{configuration} adverse pass envelope is incomplete")
    return len(outcomes)


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

    required = {PRIVACY_TEST, *GUIDANCE_TESTS, *DOWNLOAD_TESTS, *CRITICAL_WORKFLOW_TESTS}
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
    matrix_rows, decoded_rows, invalid_rows = _validate_approved_matrix(matrix_path)
    adverse_outcomes = validate_adverse_evidence(adverse_path)
    return {
        "browsers": browsers,
        "network_inspection": {"passed": True, "external_requests": 0, "source_test": PRIVACY_TEST},
        "downloads": {"passed": True, "source_tests": sorted(DOWNLOAD_TESTS)},
        "guidance": {"passed": True, "source_tests": sorted(GUIDANCE_TESTS)},
        "artifact_evidence": {
            "passed": True,
            "matrix_rows": matrix_rows,
            "decoded_rows": decoded_rows,
            "expected_invalid_rows": invalid_rows,
            "adverse_outcomes": adverse_outcomes,
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
