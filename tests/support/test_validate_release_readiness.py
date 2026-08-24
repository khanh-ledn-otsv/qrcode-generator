import itertools
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from collect_release_readiness import (
    CRITICAL_WORKFLOW_TESTS,
    DOWNLOAD_TESTS,
    GUIDANCE_TESTS,
    PRIVACY_TEST,
    REQUIRED_PROJECTS,
    BuildMismatchError,
    ResultEvidenceError,
    _expected_logo_geometry,
    _validate_approved_matrix,
    collect_build_evidence,
    collect_result_evidence,
    validate_adverse_evidence,
)
from validate_release_readiness import (
    EvidenceError,
    build_readiness_report,
    validate_automated_evidence,
)


def automated_evidence() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "release_candidate": "abc123",
        "git": {"clean": True},
        "tools": {
            "node": "v24.18.0",
            "pnpm": "11.20.0",
            "rustc": "1.98.0",
            "trunk": "0.21.14",
            "playwright": "1.62.1",
            "zxing": "3.0.2",
        },
        "reproducible_builds": {
            "match": True,
            "hashes": {"app.js": "a" * 64, "app.wasm": "b" * 64},
        },
        "browsers": {name: {"passed": True, "retries": 0} for name in ["chromium"]},
        "network_inspection": {"passed": True, "external_requests": 0},
        "downloads": {"passed": True},
        "guidance": {"passed": True},
        "artifact_evidence": {"passed": True, "matrix_rows": 96},
    }


def approved_matrix_rows() -> list[dict[str, Any]]:
    policy = json.loads(Path("tests/approved-output-matrix-policy.json").read_text())
    dimensions = policy["tuple_dimensions"]
    payload_classes = policy["required_payload_classes"]
    branding = policy["branded_geometry_policy"]
    rows = []
    dimension_order = ("profiles", "logo_states", "foreground_themes")
    tuples = itertools.product(*(range(dimensions[name]) for name in dimension_order))
    scenarios = []
    for indices in tuples:
        scenarios.extend(
            (*indices, "required-payload", payload, payload, None) for payload in payload_classes
        )
        scenarios.extend(
            (
                *indices,
                "version-coverage",
                policy["version_coverage_payload_class"],
                f"version-v{version}",
                version,
            )
            for version in range(
                policy["profile_min_versions"][indices[0]],
                policy["profile_max_versions"][indices[0]] + 1,
            )
        )
    for index, scenario in enumerate(scenarios):
        (
            profile_index,
            logo_state_index,
            foreground_index,
            case_kind,
            payload_class,
            case_label,
            covered_version,
        ) = scenario
        logo = logo_state_index == 1
        supports_logo = profile_index in branding["fixed_profile_indices"] and (
            policy["profile_min_versions"][profile_index]
            <= branding["minimum_version"]
            <= policy["profile_max_versions"][profile_index]
        )
        decoded = True
        outcome = "decoded" if decoded else "expected-invalid"
        artifact = {
            "outcome": outcome,
            "sha256": "a" * 64 if decoded else None,
            "decoder_input_sha256": "b" * 64 if decoded else None,
        }
        row_version = covered_version
        if case_kind == "required-payload" and decoded and logo:
            row_version = branding["minimum_version"]
        has_reviewed_geometry = isinstance(row_version, int) and 6 <= row_version <= 11
        row = {
            "id": f"row-{index}",
            "case_kind": case_kind,
            "case_label": case_label,
            "profile_index": profile_index,
            "logo_state_index": logo_state_index,
            "foreground_index": foreground_index,
            "payload_class": payload_class,
            "version": row_version,
            "safety": "caution" if logo and supports_logo and has_reviewed_geometry else "safe",
            "logo_geometry": None,
            "artifacts": {"png": dict(artifact), "svg": dict(artifact)},
        }
        if logo and decoded and supports_logo and has_reviewed_geometry:
            row["logo_geometry"] = _expected_logo_geometry(row, branding)
        rows.append(row)
    return rows


def adverse_outcomes() -> list[dict[str, str]]:
    outcomes = []
    manifest = json.loads(Path("tests/adverse/parameters.json").read_text())
    for envelope in manifest["pass_envelopes"]:
        outcomes.extend(
            {
                "configuration": envelope["configuration"],
                "safety": envelope["safety"],
                "transform": transform,
                "decoder": "ZXingReader version 3.0.2",
                "outcome": "decoded",
            }
            for transform in envelope["transforms"]
        )
    return outcomes


class ReleaseReadinessEvidenceTests(unittest.TestCase):
    def test_approved_matrix_requires_exact_scenario_membership(self) -> None:
        with TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "matrix.json"
            rows = approved_matrix_rows()
            rows[-1]["version"] = 1
            rows[-1]["case_label"] = "version-v1"
            evidence.write_text(json.dumps({"schema_version": 2, "rows": rows}))

            with self.assertRaisesRegex(ResultEvidenceError, "scenario membership"):
                _validate_approved_matrix(evidence)

    def test_approved_matrix_rejects_geometry_drift_from_explicit_policy(self) -> None:
        with TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "matrix.json"
            rows = approved_matrix_rows()
            branded_row = next(row for row in rows if row["logo_geometry"] is not None)
            branded_row["logo_geometry"]["knockout_modules"][0] += 1
            evidence.write_text(json.dumps({"schema_version": 2, "rows": rows}))

            with self.assertRaisesRegex(ResultEvidenceError, "unapproved branded geometry"):
                _validate_approved_matrix(evidence)

    def test_adverse_evidence_requires_each_documented_pass_envelope(self) -> None:
        with TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "adverse.json"
            evidence.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "parameters": "tests/adverse/parameters.json",
                        "seed": 20260807,
                        "outcomes": adverse_outcomes()[:-6],
                    }
                )
            )

            with self.assertRaises(ResultEvidenceError):
                validate_adverse_evidence(evidence)

    def test_adverse_evidence_requires_exact_manifest_transform_membership(self) -> None:
        with TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "adverse.json"
            outcomes = adverse_outcomes()
            outcomes[0]["transform"] = "invented-transform"
            evidence.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "parameters": "tests/adverse/parameters.json",
                        "seed": 20260807,
                        "outcomes": outcomes,
                    }
                )
            )

            with self.assertRaisesRegex(ResultEvidenceError, "pass envelope"):
                validate_adverse_evidence(evidence)

    def test_critical_workflow_gate_names_the_final_branded_browser_behaviors(self) -> None:
        self.assertIn(
            "uses approved foreground themes with one opaque white rounded ONE appearance",
            CRITICAL_WORKFLOW_TESTS,
        )
        self.assertIn(
            "fixed profiles reject centered branding above Version 6", CRITICAL_WORKFLOW_TESTS
        )
        self.assertIn(
            "Poster / Package preserves fixed dimensions at Version 12", CRITICAL_WORKFLOW_TESTS
        )
        self.assertIn(
            "always uses rounded ONE modules without an appearance control",
            CRITICAL_WORKFLOW_TESTS,
        )

    def test_result_evidence_is_derived_from_chromium_and_required_files(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            titles = {PRIVACY_TEST, *GUIDANCE_TESTS, *DOWNLOAD_TESTS, *CRITICAL_WORKFLOW_TESTS}
            report = {
                "suites": [
                    {
                        "specs": [
                            {
                                "title": title,
                                "tests": [
                                    {
                                        "projectName": project,
                                        "results": [{"status": "passed", "retry": 0}],
                                    }
                                ],
                            }
                            for project in REQUIRED_PROJECTS
                            for title in titles
                        ]
                    }
                ]
            }
            report_path = root / "playwright.json"
            report_path.write_text(json.dumps(report))
            evidence = root / "evidence"
            evidence.mkdir()
            (evidence / "approved-output-matrix.json").write_text(
                json.dumps({"schema_version": 2, "rows": approved_matrix_rows()})
            )
            (evidence / "adverse-decode.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "parameters": "tests/adverse/parameters.json",
                        "seed": 20260807,
                        "outcomes": adverse_outcomes(),
                    }
                )
            )

            result = collect_result_evidence(report_path, evidence)
            expected = json.loads(Path("tests/approved-output-matrix-policy.json").read_text())[
                "expected_rows"
            ]

            self.assertEqual(set(result["browsers"]), set(REQUIRED_PROJECTS))
            self.assertEqual(result["artifact_evidence"]["matrix_rows"], expected["total"])
            self.assertEqual(result["artifact_evidence"]["decoded_rows"], expected["decoded"])
            self.assertEqual(
                result["artifact_evidence"]["expected_invalid_rows"],
                expected["expected_invalid"],
            )
            self.assertEqual(
                result["artifact_evidence"]["adverse_outcomes"], len(adverse_outcomes())
            )

    def test_result_evidence_rejects_a_missing_required_project(self) -> None:
        with TemporaryDirectory() as temporary:
            report = Path(temporary) / "playwright.json"
            report.write_text('{"suites": []}')

            with self.assertRaisesRegex(ResultEvidenceError, "missing required tests"):
                collect_result_evidence(report, Path(temporary))

    def test_identical_builds_record_hashes(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = root / "first"
            second = root / "second"
            first.mkdir()
            second.mkdir()
            for build in (first, second):
                (build / "app.js").write_bytes(b"javascript")
                (build / "app_bg.wasm").write_bytes(b"wasm")

            evidence = collect_build_evidence(first, second)

            self.assertTrue(evidence["reproducible_builds"]["match"])
            self.assertEqual(
                set(evidence["reproducible_builds"]["hashes"]), {"app.js", "app_bg.wasm"}
            )

    def test_different_builds_are_rejected(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = root / "first"
            second = root / "second"
            first.mkdir()
            second.mkdir()
            (first / "app_bg.wasm").write_bytes(b"first")
            (second / "app_bg.wasm").write_bytes(b"second")

            with self.assertRaises(BuildMismatchError):
                collect_build_evidence(first, second)

    def test_complete_automated_evidence_builds_a_report(self) -> None:
        automated = automated_evidence()

        validate_automated_evidence(automated)
        report = build_readiness_report(automated)

        self.assertEqual(report["release_candidate"], "abc123")
        self.assertEqual(report["decision"], "passed")
        self.assertEqual(len(report["criteria"]), 5)
        self.assertTrue(all(item["status"] == "passed" for item in report["criteria"]))

    def test_chromium_must_pass_without_retries(self) -> None:
        automated = automated_evidence()
        automated["browsers"]["chromium"] = {"passed": True, "retries": 1}

        with self.assertRaisesRegex(EvidenceError, "chromium"):
            validate_automated_evidence(automated)


if __name__ == "__main__":
    unittest.main()
