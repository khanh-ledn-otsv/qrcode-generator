import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from collect_release_readiness import (
    CRITICAL_WORKFLOW_TESTS,
    DOWNLOAD_TESTS,
    GUIDANCE_TEST,
    PRIVACY_TEST,
    REQUIRED_PROJECTS,
    BuildMismatchError,
    ResultEvidenceError,
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
            "rustc": "1.97.1",
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
    expected = policy["expected_rows"]
    dimensions = policy["tuple_dimensions"]
    payload_classes = policy["required_payload_classes"]
    rows = []
    for index in range(expected["total"]):
        decoded = index < expected["decoded"]
        logo = index == 0
        outcome = "decoded" if decoded else "expected-invalid"
        artifact = {
            "outcome": outcome,
            "sha256": "a" * 64 if decoded else None,
            "decoder_input_sha256": "b" * 64 if decoded else None,
        }
        rows.append(
            {
                "id": f"row-{index}",
                "case_kind": "required-payload"
                if index < expected["required_payload"]
                else "version-coverage",
                "profile_index": index % dimensions["profiles"],
                "foreground_index": 0,
                "background_index": index % dimensions["backgrounds"],
                "module_style_index": 0,
                "finder_style_index": 0,
                "logo_state_index": 1 if logo else 0,
                "payload_class": payload_classes[index % len(payload_classes)],
                "version": 6 if logo else index % 13 + 1,
                "safety": "caution" if decoded else None,
                "logo_geometry": {
                    "source_ten_thousandths": [140000, 180625, 130000, 48750],
                    "knockout_modules": [13, 17, 15, 7],
                    "protected_clearance_modules": 6,
                    "obscured_data_modules": 105,
                    "obscured_remainder_modules": 0,
                }
                if logo
                else None,
                "artifacts": {"png": dict(artifact), "svg": dict(artifact)},
            }
        )
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
            "uses compact dots and standard square finders without a shape control",
            CRITICAL_WORKFLOW_TESTS,
        )
        self.assertIn(
            "rejects a centered logo when the payload naturally selects Version 7",
            CRITICAL_WORKFLOW_TESTS,
        )
        self.assertNotIn(
            "uses square modules and standard square finders without a shape control",
            CRITICAL_WORKFLOW_TESTS,
        )

    def test_result_evidence_is_derived_from_chromium_and_required_files(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            titles = {PRIVACY_TEST, GUIDANCE_TEST, *DOWNLOAD_TESTS, *CRITICAL_WORKFLOW_TESTS}
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
            self.assertEqual(result["artifact_evidence"]["adverse_outcomes"], 29)

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
