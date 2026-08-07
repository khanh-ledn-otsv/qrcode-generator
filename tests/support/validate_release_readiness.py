import argparse
import json
import re
from pathlib import Path
from typing import Any


class EvidenceError(RuntimeError):
    pass


REQUIRED_BROWSERS = ("chromium", "mobile-chromium", "firefox", "webkit")
PLACEHOLDER = re.compile(r"(?:\b(?:todo|tbd|pending|replace me)\b|<[^>]*>)", re.IGNORECASE)
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def _mapping(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f"{label} must be an object")
    return value


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value.strip() or PLACEHOLDER.search(value.strip()):
        raise EvidenceError(f"{label} must be named and must not be a placeholder")
    return value.strip()


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

    browsers = _mapping(evidence.get("browsers"), "browsers")
    for browser in REQUIRED_BROWSERS:
        result = _mapping(browsers.get(browser), f"browsers.{browser}")
        if result.get("passed") is not True or result.get("retries") != 0:
            raise EvidenceError(f"{browser} must pass without retries")

    for gate in ("network_inspection", "downloads", "guidance", "artifact_evidence"):
        _passed(evidence, gate)
    network = _mapping(evidence["network_inspection"], "network_inspection")
    if network.get("external_requests") != 0:
        raise EvidenceError("network inspection recorded external runtime requests")


def build_readiness_report(automated: dict[str, Any]) -> dict[str, Any]:
    validate_automated_evidence(automated)
    return {
        "schema_version": 1,
        "release_candidate": automated["release_candidate"],
        "decision": "passed",
        "criteria": [
            {
                "id": "runtime-privacy",
                "status": "passed",
                "evidence": ["automated.network_inspection"],
            },
            {
                "id": "reproducible-build",
                "status": "passed",
                "evidence": ["automated.reproducible_builds"],
            },
            {
                "id": "supported-browsers",
                "status": "passed",
                "evidence": ["automated.browsers"],
            },
            {
                "id": "artifact-validation",
                "status": "passed",
                "evidence": ["automated.artifact_evidence"],
            },
            {
                "id": "user-guidance",
                "status": "passed",
                "evidence": ["automated.guidance"],
            },
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--automated", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()

    automated = json.loads(arguments.automated.read_text())
    report = build_readiness_report(automated)
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"release readiness passed for {report['release_candidate']}")


if __name__ == "__main__":
    main()
