use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    ProfileId, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg,
};
use sha2::{Digest, Sha256};

pub const PAYLOAD_LENGTH: usize = 2_331;
pub const SHA256: [&str; 2] = [
    "e6546e2da8c610f896ab5f639634923173ac8a775cbb3e9e9fc0b95acbd70fa5",
    "69e6587286fcd49c5f45c5556b19c33ecbbf7e1ad2bfeb743d02bee8a849c8f8",
];

pub fn artifacts() -> (Vec<u8>, Vec<u8>) {
    let payload = "a".repeat(PAYLOAD_LENGTH);
    let version_forty = Version::new(40).unwrap();
    let encoded = encode(EncodeRequest::with_version_range(
        &payload,
        ErrorCorrection::Medium,
        version_forty,
        version_forty,
    ))
    .unwrap();
    let adaptive = SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == ProfileId::Adaptive)
        .unwrap();
    let model = RenderModel::new(&encoded, RenderOptions::safe(adaptive).unwrap()).unwrap();

    (
        render_svg(&model).unwrap().into_bytes(),
        render_png(&model).unwrap(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn provenance_hashes() -> [String; 2] {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/adaptive-v40-artifact.json")).unwrap();
    let payload = "a".repeat(PAYLOAD_LENGTH);

    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(
        manifest["id"],
        "synthetic-adaptive-v40-m-unbranded-artifact"
    );
    assert_eq!(manifest["synthetic"], true);
    assert_eq!(manifest["payload_repeat"]["text"], "a");
    assert_eq!(manifest["payload_repeat"]["count"], PAYLOAD_LENGTH);
    assert_eq!(manifest["payload_sha256"], sha256_hex(payload.as_bytes()));
    assert_eq!(manifest["encoding"], "utf-8");
    assert_eq!(manifest["profile"], "Adaptive");
    assert_eq!(manifest["version"], 40);
    assert_eq!(manifest["ecc"], "M");
    assert_eq!(manifest["logo"], "none");
    assert_eq!(manifest["verification"]["state"], "accepted");
    assert_eq!(
        manifest["generation"]["source"],
        "crates/qr-render/tests/support/adaptive_v40_artifact_fixture.rs"
    );
    assert_eq!(manifest["local_verification"].as_array().unwrap().len(), 3);
    assert_eq!(
        manifest["verification"]["reviewer"],
        "ticket-31-visible-round-dot-review"
    );
    assert_eq!(manifest["verification"]["verified_at"], "2026-08-10");

    [
        manifest["svg_sha256"].as_str().unwrap().to_owned(),
        manifest["png_sha256"].as_str().unwrap().to_owned(),
    ]
}

pub fn hashes() -> [String; 2] {
    let (svg, png) = artifacts();
    [sha256_hex(&svg), sha256_hex(&png)]
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
