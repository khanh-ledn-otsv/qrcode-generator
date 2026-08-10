use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    ProfileId, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg,
};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = "cross-target unbranded Inline Version 6 artifact";
pub const SHA256: [&str; 2] = [
    "a5f2060b7f03f98a8e9e7278abc30b69831d1162568f57cf19d40d3f3791b425",
    "4ee3f0e06fb0f77ac70b59678df99a83a0643ebec9f836dcbc8f5b1713788750",
];

pub fn artifacts() -> (Vec<u8>, Vec<u8>) {
    let version_six = Version::new(6).unwrap();
    let encoded = encode(EncodeRequest::with_version_range(
        PAYLOAD,
        ErrorCorrection::Medium,
        version_six,
        version_six,
    ))
    .unwrap();
    let inline = SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == ProfileId::Inline)
        .unwrap();
    let model = RenderModel::new(&encoded, RenderOptions::safe(inline).unwrap()).unwrap();

    (
        render_svg(&model).unwrap().into_bytes(),
        render_png(&model).unwrap(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn provenance_hashes() -> [String; 2] {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/inline-version-six-artifact.json")).unwrap();

    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["synthetic"], true);
    assert_eq!(manifest["payload"], PAYLOAD);
    assert_eq!(manifest["payload_sha256"], sha256_hex(PAYLOAD.as_bytes()));
    assert_eq!(manifest["profile"], "Inline");
    assert_eq!(manifest["version"], 6);
    assert_eq!(manifest["ecc"], "M");
    assert_eq!(manifest["logo"], "none");
    assert_eq!(manifest["verification"]["state"], "accepted");

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
