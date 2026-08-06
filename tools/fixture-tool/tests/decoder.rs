#![cfg(unix)]

use fixture_tool::{DecodeExpectation, ErrorCorrection, QrVersion, ZxingDecoder};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn decoder_compares_exact_bytes_and_exposed_qr_metadata() {
    let (directory, decoder_path, source_commit) = install_fake_decoder(
        r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  echo "ZXingReader version 3.0.2"
elif printf '%s\n' "$@" | grep -q -- '-bytes'; then
  printf 'SYNTHETIC-FIXTURE-01'
elif ! printf '%s\n' "$@" | grep -qx -- 'eci'; then
  echo 'expected lowercase eci mode' >&2
  exit 2
else
  cat <<'EOF'
Text:       "SYNTHETIC-FIXTURE-01"
Bytes:      53 59 4E
Format:     QR Code
HasECI:     0
ECLevel:    M
Version:    1
EOF
fi
"#,
    );
    let artifact = directory.path().join("artifact.png");
    fs::write(&artifact, b"not read by the boundary fake").unwrap();

    let decoder = ZxingDecoder::new(decoder_path, "3.0.2", directory.path(), source_commit);
    let result = decoder
        .inspect_and_compare(
            &artifact,
            &DecodeExpectation {
                payload: b"SYNTHETIC-FIXTURE-01".to_vec(),
                version: QrVersion::new(1).unwrap(),
                ecc: ErrorCorrection::M,
                eci_assignment: None,
            },
        )
        .unwrap();

    assert_eq!(result.version, QrVersion::new(1).unwrap());
    assert_eq!(result.ecc, ErrorCorrection::M);
    assert!(!result.has_eci);
}

#[test]
fn decoder_rejects_an_unpinned_binary_version() {
    let (directory, decoder_path, source_commit) =
        install_fake_decoder("#!/bin/sh\necho 'ZXingReader version 9.9.9'\n");

    let error = ZxingDecoder::new(decoder_path, "3.0.2", directory.path(), source_commit)
        .inspect_and_compare(
            directory.path().join("artifact.png"),
            &DecodeExpectation {
                payload: Vec::new(),
                version: QrVersion::new(1).unwrap(),
                ecc: ErrorCorrection::L,
                eci_assignment: None,
            },
        )
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("expected ZXingReader version 3.0.2")
    );
}

#[test]
fn decoder_rejects_a_modified_source_checkout() {
    let (directory, decoder_path, source_commit) =
        install_fake_decoder("#!/bin/sh\necho 'ZXingReader version 3.0.2'\n");
    fs::write(&decoder_path, "#!/bin/sh\necho modified\n").unwrap();

    let error = ZxingDecoder::new(decoder_path, "3.0.2", directory.path(), source_commit)
        .inspect_and_compare(
            directory.path().join("artifact.png"),
            &DecodeExpectation {
                payload: Vec::new(),
                version: QrVersion::new(1).unwrap(),
                ecc: ErrorCorrection::L,
                eci_assignment: None,
            },
        )
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("source checkout has tracked modifications")
    );
}

fn install_fake_decoder(script: &str) -> (tempfile::TempDir, std::path::PathBuf, String) {
    let directory = tempfile::tempdir().unwrap();
    let decoder_path = directory.path().join("ZXingReader");
    fs::write(&decoder_path, script).unwrap();
    let mut permissions = fs::metadata(&decoder_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&decoder_path, permissions).unwrap();
    for arguments in [
        vec!["init", "--quiet"],
        vec!["add", "ZXingReader"],
        vec![
            "-c",
            "user.name=Fixture Test",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fake decoder",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(arguments)
                .current_dir(directory.path())
                .status()
                .unwrap()
                .success()
        );
    }
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    let source_commit = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    (directory, decoder_path, source_commit)
}
