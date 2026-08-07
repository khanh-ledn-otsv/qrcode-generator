import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from collect_release_readiness import (
    ACCESSIBILITY_TESTS,
    CRITICAL_WORKFLOW_TESTS,
    DOWNLOAD_TESTS,
    GUIDANCE_TEST,
    PRIVACY_TEST,
    REQUIRED_PROJECTS,
    BuildMismatchError,
    ResultEvidenceError,
    collect_build_evidence,
    collect_result_evidence,
)
from validate_release_readiness import (
    EvidenceError,
    build_readiness_report,
    validate_automated_evidence,
    validate_manual_evidence,
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
        "compressed_wasm": {"observed_bytes": 154450, "maximum_bytes": 160000},
        "browsers": {
            name: {"passed": True, "retries": 0}
            for name in ["chromium", "mobile-chromium", "firefox", "webkit"]
        },
        "network_inspection": {"passed": True, "external_requests": 0},
        "accessibility": {"passed": True, "violations": 0},
        "downloads": {"passed": True},
        "guidance": {"passed": True},
        "artifact_evidence": {"passed": True, "matrix_rows": 192},
    }


def manual_evidence() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "release_candidate": "abc123",
        "owner": "Release Owner",
        "environments": {
            "desktop_browsers": [{"name": "Safari 20 / macOS", "evidence_project": "webkit"}],
            "mobile_browsers": [
                {"name": "Chrome / Pixel 7", "evidence_project": "mobile-chromium"}
            ],
            "camera_devices": ["iPhone 17 camera"],
            "scanner_apps": ["Scanner 1.0"],
            "screens": ["MacBook display"],
            "printers": ["Office printer"],
            "materials": ["matte white stock"],
            "placement_environments": ["office wall"],
            "accessibility_pairs": [
                "VoiceOver / Safari on macOS",
                "NVDA / Firefox on Windows",
            ],
        },
        "physical_results": [
            {
                "id": "print-25",
                "size_mm": 25,
                "result": "passed",
                "evidence": "evidence/print-25.pdf",
                "tested_by": "Release Owner",
                "tested_on": "2026-08-07",
                "environments": {
                    "camera_device": "iPhone 17 camera",
                    "scanner_app": "Scanner 1.0",
                    "screen": "MacBook display",
                    "printer": "Office printer",
                    "material": "matte white stock",
                    "placement_environment": "office wall",
                },
            },
            {
                "id": "print-30",
                "size_mm": 30,
                "result": "passed",
                "evidence": "evidence/print-30.pdf",
                "tested_by": "Release Owner",
                "tested_on": "2026-08-07",
                "environments": {
                    "camera_device": "iPhone 17 camera",
                    "scanner_app": "Scanner 1.0",
                    "screen": "MacBook display",
                    "printer": "Office printer",
                    "material": "matte white stock",
                    "placement_environment": "office wall",
                },
            },
        ],
        "manual_accessibility_results": [
            {"pair": "VoiceOver / Safari on macOS", "result": "passed"},
            {"pair": "NVDA / Firefox on Windows", "result": "passed"},
        ],
        "exceptions": [],
        "signoff": {
            "decision": "approved",
            "signed_by": "Release Owner",
            "signed_on": "2026-08-07",
        },
    }


class ReleaseReadinessEvidenceTests(unittest.TestCase):
    def test_result_evidence_is_derived_from_all_required_projects_and_files(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            titles = {
                PRIVACY_TEST,
                GUIDANCE_TEST,
                *DOWNLOAD_TESTS,
                *ACCESSIBILITY_TESTS,
                *CRITICAL_WORKFLOW_TESTS,
            }
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
            (evidence / "approved-output-matrix.json").write_text(json.dumps({"rows": [{}] * 192}))
            (evidence / "adverse-decode.json").write_text(json.dumps({"outcomes": [{}]}))

            result = collect_result_evidence(report_path, evidence)

            self.assertEqual(set(result["browsers"]), set(REQUIRED_PROJECTS))
            self.assertEqual(result["artifact_evidence"]["matrix_rows"], 192)

    def test_result_evidence_rejects_a_missing_required_project(self) -> None:
        with TemporaryDirectory() as temporary:
            report = Path(temporary) / "playwright.json"
            report.write_text('{"suites": []}')

            with self.assertRaisesRegex(ResultEvidenceError, "missing required tests"):
                collect_result_evidence(report, Path(temporary))

    def test_identical_builds_record_hashes_and_compressed_wasm_size(self) -> None:
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = root / "first"
            second = root / "second"
            first.mkdir()
            second.mkdir()
            for build in (first, second):
                (build / "app.js").write_bytes(b"javascript")
                (build / "app_bg.wasm").write_bytes(b"wasm" * 100)

            evidence = collect_build_evidence(first, second, maximum_wasm_bytes=160_000)

            self.assertTrue(evidence["reproducible_builds"]["match"])
            self.assertEqual(
                set(evidence["reproducible_builds"]["hashes"]), {"app.js", "app_bg.wasm"}
            )
            self.assertGreater(evidence["compressed_wasm"]["observed_bytes"], 0)

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
                collect_build_evidence(first, second, maximum_wasm_bytes=160_000)

    def test_complete_automated_and_manual_evidence_builds_a_signed_report(self) -> None:
        automated = automated_evidence()
        manual = manual_evidence()

        validate_automated_evidence(automated)
        validate_manual_evidence(manual)
        report = build_readiness_report(automated, manual)

        self.assertEqual(report["release_candidate"], "abc123")
        self.assertEqual(report["decision"], "approved")
        self.assertEqual(len(report["criteria"]), 6)
        self.assertTrue(all(item["status"] == "passed" for item in report["criteria"]))

    def test_pending_or_placeholder_physical_evidence_is_rejected(self) -> None:
        manual = manual_evidence()
        manual["owner"] = "TODO"
        manual["physical_results"] = []

        with self.assertRaisesRegex(EvidenceError, "owner|physical"):
            validate_manual_evidence(manual)

    def test_browsers_must_pass_without_retries(self) -> None:
        automated = automated_evidence()
        automated["browsers"]["webkit"] = {"passed": True, "retries": 1}

        with self.assertRaisesRegex(EvidenceError, "webkit"):
            validate_automated_evidence(automated)

    def test_embedded_placeholders_are_rejected(self) -> None:
        manual = manual_evidence()
        manual["environments"]["accessibility_pairs"][0] = "VoiceOver / TODO browser"

        with self.assertRaisesRegex(EvidenceError, "placeholder"):
            validate_manual_evidence(manual)

    def test_duplicate_or_missing_required_accessibility_pairs_are_rejected(self) -> None:
        manual = manual_evidence()
        manual["manual_accessibility_results"] = [
            {"pair": "VoiceOver / Safari on macOS", "result": "passed"},
            {"pair": "VoiceOver / Safari on macOS", "result": "passed"},
        ]

        with self.assertRaisesRegex(EvidenceError, "accessibility"):
            validate_manual_evidence(manual)

    def test_signed_exception_can_replace_physical_results(self) -> None:
        manual = manual_evidence()
        manual["physical_results"] = []
        manual["exceptions"] = [
            {
                "criterion": "physical-validation",
                "reason": "Owner accepted a documented release limitation.",
                "signed_by": "Release Owner",
                "signed_on": "2026-08-07",
            }
        ]

        validate_manual_evidence(manual)
        report = build_readiness_report(automated_evidence(), manual)

        physical = next(item for item in report["criteria"] if item["id"] == "physical-validation")
        self.assertEqual(physical["status"], "signed-exception")


if __name__ == "__main__":
    unittest.main()
