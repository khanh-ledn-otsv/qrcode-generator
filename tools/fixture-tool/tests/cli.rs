use std::process::Command;

#[test]
fn verify_command_checks_the_committed_manifest_without_regenerating_it() {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_fixture-tool"))
        .arg("verify")
        .arg(repository_root.join("tests/fixtures/manifest.json"))
        .status()
        .unwrap();

    assert!(status.success());
}
