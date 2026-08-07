from __future__ import annotations

import argparse
import json
import re
from datetime import date
from pathlib import Path
from typing import Any


class EvidenceError(RuntimeError):
    pass


REQUIRED_ENVIRONMENTS = (
    "desktop_browsers",
    "mobile_browsers",
    "camera_devices",
    "scanner_apps",
    "screens",
    "printers",
    "materials",
    "placement_environments",
    "accessibility_pairs",
)
REQUIRED_BROWSERS = ("chromium", "mobile-chromium", "firefox", "webkit")
PLACEHOLDER = re.compile(r"(?:\b(?:todo|tbd|pending|replace me)\b|<[^>]*>)", re.IGNORECASE)
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def _mapping(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f"{label} must be an object")
    return value


def _list(value: object, label: str) -> list[Any]:
    if not isinstance(value, list):
        raise EvidenceError(f"{label} must be a list")
    return value


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value.strip() or PLACEHOLDER.search(value.strip()):
        raise EvidenceError(f"{label} must be named and must not be a placeholder")
    return value.strip()


def _date(value: object, label: str) -> str:
    text = _text(value, label)
    try:
        date.fromisoformat(text)
    except ValueError as error:
        raise EvidenceError(f"{label} must use YYYY-MM-DD") from error
    return text


def _passed(evidence: dict[str, Any], key: str) -> None:
    result = _mapping(evidence.get(key), key)
    if result.get("passed") is not True:
        raise EvidenceError(f"{key} did not pass")


def validate_automated_evidence(evidence: dict[str, Any]) -> None:
    if evidence.get("schema_version") != 1:
        raise EvidenceError("automated evidence schema_version must be 1")
    _text(evidence.get("release_candidate"), "release_candidate")
    git = _mapping(evidence.get("git"), "git")
    if git.get("clean") is not True:
        raise EvidenceError("release evidence requires a clean git worktree")

    tools = _mapping(evidence.get("tools"), "tools")
    for tool in ("node", "pnpm", "rustc", "trunk", "playwright", "zxing"):
        _text(tools.get(tool), f"tools.{tool}")

    builds = _mapping(evidence.get("reproducible_builds"), "reproducible_builds")
    if builds.get("match") is not True:
        raise EvidenceError("independent production-build hashes do not match")
    hashes = _mapping(builds.get("hashes"), "reproducible_builds.hashes")
    if not hashes or not any(name.endswith(".wasm") for name in hashes):
        raise EvidenceError("production build hashes must include the WASM artifact")
    for name, digest in hashes.items():
        _text(name, "build artifact name")
        if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
            raise EvidenceError(f"invalid SHA-256 for build artifact {name}")

    bundle = _mapping(evidence.get("compressed_wasm"), "compressed_wasm")
    observed = bundle.get("observed_bytes")
    maximum = bundle.get("maximum_bytes")
    if not isinstance(observed, int) or not isinstance(maximum, int) or observed > maximum:
        raise EvidenceError("compressed WASM exceeds its recorded ceiling")

    browsers = _mapping(evidence.get("browsers"), "browsers")
    for browser in REQUIRED_BROWSERS:
        result = _mapping(browsers.get(browser), f"browsers.{browser}")
        if result.get("passed") is not True or result.get("retries") != 0:
            raise EvidenceError(f"{browser} must pass without retries")

    for gate in (
        "network_inspection",
        "accessibility",
        "downloads",
        "guidance",
        "artifact_evidence",
    ):
        _passed(evidence, gate)
    network = _mapping(evidence["network_inspection"], "network_inspection")
    if network.get("external_requests") != 0:
        raise EvidenceError("network inspection recorded external runtime requests")
    accessibility = _mapping(evidence["accessibility"], "accessibility")
    if accessibility.get("violations") != 0:
        raise EvidenceError("automated accessibility inspection recorded violations")


def _physical_exception(manual: dict[str, Any]) -> dict[str, Any] | None:
    exceptions = _list(manual.get("exceptions"), "exceptions")
    for exception_value in exceptions:
        exception = _mapping(exception_value, "exception")
        if exception.get("criterion") != "physical-validation":
            raise EvidenceError("exceptions may only name a known release criterion")
        _text(exception.get("reason"), "exception.reason")
        _text(exception.get("signed_by"), "exception.signed_by")
        _date(exception.get("signed_on"), "exception.signed_on")
        return exception
    return None


def validate_manual_evidence(manual: dict[str, Any]) -> None:
    if manual.get("schema_version") != 1:
        raise EvidenceError("manual evidence schema_version must be 1")
    _text(manual.get("release_candidate"), "release_candidate")
    _text(manual.get("owner"), "owner")

    environments = _mapping(manual.get("environments"), "environments")
    for category in ("desktop_browsers", "mobile_browsers"):
        values = _list(environments.get(category), f"environments.{category}")
        if not values:
            raise EvidenceError(f"environments.{category} must name at least one browser")
        for index, value in enumerate(values):
            browser = _mapping(value, f"environments.{category}[{index}]")
            _text(browser.get("name"), f"environments.{category}[{index}].name")
            project = _text(
                browser.get("evidence_project"),
                f"environments.{category}[{index}].evidence_project",
            )
            if project not in REQUIRED_BROWSERS:
                raise EvidenceError(f"unsupported automated browser project: {project}")

    for category in REQUIRED_ENVIRONMENTS[2:]:
        values = _list(environments.get(category), f"environments.{category}")
        if not values:
            raise EvidenceError(f"environments.{category} must name at least one environment")
        for index, value in enumerate(values):
            _text(value, f"environments.{category}[{index}]")

    physical_results = _list(manual.get("physical_results"), "physical_results")
    exception = _physical_exception(manual)
    if physical_results:
        sizes: set[int] = set()
        physical_fields = {
            "camera_devices": "camera_device",
            "scanner_apps": "scanner_app",
            "screens": "screen",
            "printers": "printer",
            "materials": "material",
            "placement_environments": "placement_environment",
        }
        used_environments = {category: set() for category in physical_fields}
        for index, result_value in enumerate(physical_results):
            result = _mapping(result_value, f"physical_results[{index}]")
            _text(result.get("id"), f"physical_results[{index}].id")
            size = result.get("size_mm")
            if size not in (25, 30):
                raise EvidenceError("physical print results must use 25 mm or 30 mm samples")
            sizes.add(size)
            if result.get("result") != "passed":
                raise EvidenceError(f"physical result {result.get('id')} did not pass")
            _text(result.get("evidence"), f"physical_results[{index}].evidence")
            _text(result.get("tested_by"), f"physical_results[{index}].tested_by")
            _date(result.get("tested_on"), f"physical_results[{index}].tested_on")
            result_environments = _mapping(
                result.get("environments"), f"physical_results[{index}].environments"
            )
            for category, field in physical_fields.items():
                name = _text(
                    result_environments.get(field),
                    f"physical_results[{index}].environments.{field}",
                )
                if name not in environments[category]:
                    raise EvidenceError(
                        f"physical result {result.get('id')} references an unnamed {field}: {name}"
                    )
                used_environments[category].add(name)
        if sizes != {25, 30}:
            raise EvidenceError("physical evidence must include both 25 mm and 30 mm samples")
        for category, used in used_environments.items():
            missing = set(environments[category]) - used
            if missing:
                raise EvidenceError(
                    f"physical results do not cover named environments.{category}: {sorted(missing)}"
                )
    elif exception is None:
        raise EvidenceError("physical evidence is missing and has no signed exception")

    accessibility_results = _list(
        manual.get("manual_accessibility_results"), "manual_accessibility_results"
    )
    pairs = set(_list(environments["accessibility_pairs"], "accessibility_pairs"))
    if not any("voiceover" in pair.lower() for pair in pairs) or not any(
        "nvda" in pair.lower() and "windows" in pair.lower() for pair in pairs
    ):
        raise EvidenceError("manual accessibility requires VoiceOver and NVDA on Windows pairs")
    completed_pairs: set[str] = set()
    for index, result_value in enumerate(accessibility_results):
        result = _mapping(result_value, f"manual_accessibility_results[{index}]")
        pair = _text(result.get("pair"), f"manual_accessibility_results[{index}].pair")
        if pair not in pairs or result.get("result") != "passed":
            raise EvidenceError(f"manual accessibility pair {pair} did not pass")
        if pair in completed_pairs:
            raise EvidenceError(f"duplicate manual accessibility result for {pair}")
        completed_pairs.add(pair)
    if completed_pairs != pairs:
        raise EvidenceError(
            f"manual accessibility results do not cover every named pair: {sorted(pairs - completed_pairs)}"
        )

    signoff = _mapping(manual.get("signoff"), "signoff")
    if signoff.get("decision") != "approved":
        raise EvidenceError("release owner has not approved the manual evidence")
    _text(signoff.get("signed_by"), "signoff.signed_by")
    _date(signoff.get("signed_on"), "signoff.signed_on")


def build_readiness_report(automated: dict[str, Any], manual: dict[str, Any]) -> dict[str, Any]:
    validate_automated_evidence(automated)
    validate_manual_evidence(manual)
    if automated["release_candidate"] != manual["release_candidate"]:
        raise EvidenceError("automated and manual evidence name different release candidates")
    physical_status = "signed-exception" if _physical_exception(manual) else "passed"
    return {
        "schema_version": 1,
        "release_candidate": automated["release_candidate"],
        "decision": "approved",
        "signed_by": manual["signoff"]["signed_by"],
        "signed_on": manual["signoff"]["signed_on"],
        "criteria": [
            {
                "id": "runtime-privacy",
                "status": "passed",
                "evidence": ["automated.network_inspection"],
            },
            {
                "id": "reproducible-build",
                "status": "passed",
                "evidence": ["automated.reproducible_builds", "automated.compressed_wasm"],
            },
            {
                "id": "supported-browsers",
                "status": "passed",
                "evidence": ["automated.browsers", "automated.accessibility"],
            },
            {
                "id": "physical-validation",
                "status": physical_status,
                "evidence": ["manual.physical_results", "manual.exceptions"],
            },
            {
                "id": "user-guidance",
                "status": "passed",
                "evidence": ["automated.guidance"],
            },
            {
                "id": "criterion-mapping",
                "status": "passed",
                "evidence": ["this report"],
            },
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--automated", type=Path, required=True)
    parser.add_argument("--manual", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()

    if not arguments.manual.exists():
        raise EvidenceError(
            f"manual evidence is missing: copy tests/release/manual-evidence.template.json to {arguments.manual} and replace every placeholder"
        )
    automated = json.loads(arguments.automated.read_text())
    manual = json.loads(arguments.manual.read_text())
    report = build_readiness_report(automated, manual)
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"release readiness approved for {report['release_candidate']}")


if __name__ == "__main__":
    main()
