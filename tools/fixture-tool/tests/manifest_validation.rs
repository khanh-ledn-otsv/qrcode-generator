use fixture_tool::{FixtureManifest, VerificationError};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fixture_tree(source_count: usize) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join("payloads")).unwrap();
    fs::create_dir_all(root.join("matrices")).unwrap();

    let payload = b"SYNTHETIC-FIXTURE-01";
    let matrix = matrix_for_version(1);
    fs::write(root.join("payloads/example.bin"), payload).unwrap();
    fs::write(root.join("matrices/example.txt"), &matrix).unwrap();

    let all_sources = [
        serde_json::json!({
            "oracle": "nayuki",
            "tool": "Nayuki QR Code Generator",
            "implementation": "nayuki-qrcodegen-python",
            "version": "1.8.0",
            "command": "python3 tests/support/generate_fixtures.py --fixture synthetic-v01-m-mask0-byte-001 --oracle nayuki",
            "matrix_sha256": sha256(matrix.as_bytes())
        }),
        serde_json::json!({
            "oracle": "python-qrcode",
            "tool": "python-qrcode",
            "implementation": "python-qrcode",
            "version": "8.2",
            "command": "python3 tests/support/generate_fixtures.py --fixture synthetic-v01-m-mask0-byte-001 --oracle python-qrcode",
            "matrix_sha256": sha256(matrix.as_bytes())
        }),
    ];
    let manifest = serde_json::json!({
        "schema_version": 1,
        "decoder": {
            "tool": "ZXing-C++ ZXingReader",
            "version": "3.0.2",
            "source_url": "https://github.com/zxing-cpp/zxing-cpp.git",
            "source_commit": "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825",
            "checkout_command": "git clone https://github.com/zxing-cpp/zxing-cpp.git tests/oracles/zxing-cpp && git -C tests/oracles/zxing-cpp checkout --detach 8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825",
            "build_command": "cmake --build tests/oracles/zxing-cpp/build --config Release"
        },
        "fixtures": [{
            "id": "synthetic-v01-m-mask0-byte-001",
            "synthetic": true,
            "payload_file": "payloads/example.bin",
            "payload_sha256": sha256(payload),
            "encoding": "utf-8",
            "eci_assignment": null,
            "mode": "byte",
            "version": 1,
            "ecc": "M",
            "mask": 0,
            "expected_matrix_file": "matrices/example.txt",
            "expected_matrix_sha256": sha256(matrix.as_bytes()),
            "sources": all_sources[..source_count].to_vec(),
            "verification": {
                "state": "accepted",
                "reviewer": "repository-owner",
                "verified_at": "2026-08-05",
                "notes": "Generators agreed byte-for-byte."
            }
        }]
    });
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    directory
}

fn matrix_for_version(version: u8) -> String {
    let size = 17 + usize::from(version) * 4;
    let row = "0".repeat(size);
    (0..size).map(|_| format!("{row}\n")).collect()
}

fn load_and_verify(root: &Path) -> Result<FixtureManifest, VerificationError> {
    FixtureManifest::load_and_verify(root.join("manifest.json"))
}

#[test]
fn accepts_a_strict_dual_oracle_fixture_with_matching_hashes() {
    let directory = fixture_tree(2);

    let manifest = load_and_verify(directory.path()).unwrap();

    assert_eq!(manifest.fixtures().len(), 1);
}

#[test]
fn rejects_payload_hash_drift() {
    let directory = fixture_tree(2);
    fs::write(directory.path().join("payloads/example.bin"), b"changed").unwrap();

    let error = load_and_verify(directory.path()).unwrap_err();

    assert!(error.to_string().contains("payload SHA-256"));
}

#[test]
fn rejects_a_fixture_without_two_independent_generators() {
    let directory = fixture_tree(1);

    let error = load_and_verify(directory.path()).unwrap_err();

    assert!(error.to_string().contains("two independent generators"));
}

#[test]
fn rejects_oracle_matrix_disagreement() {
    let directory = fixture_tree(2);
    let manifest_path = directory.path().join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["fixtures"][0]["sources"][1]["matrix_sha256"] = "0".repeat(64).into();
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();

    let error = load_and_verify(directory.path()).unwrap_err();

    assert!(error.to_string().contains("oracle matrix disagreement"));
}

#[test]
fn rejects_provenance_that_does_not_match_the_declared_oracle() {
    let directory = fixture_tree(2);
    let manifest_path = directory.path().join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["fixtures"][0]["sources"][1]["tool"] = "Different tool".into();
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();

    let error = load_and_verify(directory.path()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("does not match pinned provenance")
    );
}

#[test]
fn rejects_a_matrix_with_the_wrong_dimensions() {
    let directory = fixture_tree(2);
    let invalid_matrix = "01\n10\n";
    fs::write(
        directory.path().join("matrices/example.txt"),
        invalid_matrix,
    )
    .unwrap();
    let manifest_path = directory.path().join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let hash = sha256(invalid_matrix.as_bytes());
    manifest["fixtures"][0]["expected_matrix_sha256"] = hash.clone().into();
    manifest["fixtures"][0]["sources"][0]["matrix_sha256"] = hash.clone().into();
    manifest["fixtures"][0]["sources"][1]["matrix_sha256"] = hash.into();
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();

    let error = load_and_verify(directory.path()).unwrap_err();

    assert!(error.to_string().contains("21 rows of 21 modules"));
}
