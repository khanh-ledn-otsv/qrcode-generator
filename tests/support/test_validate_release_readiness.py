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


class ReleaseReadinessEvidenceTests(unittest.TestCase):
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
            (evidence / "approved-output-matrix.json").write_text(json.dumps({"rows": [{}] * 96}))
            (evidence / "adverse-decode.json").write_text(json.dumps({"outcomes": [{}]}))

            result = collect_result_evidence(report_path, evidence)

            self.assertEqual(set(result["browsers"]), set(REQUIRED_PROJECTS))
            self.assertEqual(result["artifact_evidence"]["matrix_rows"], 96)

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
