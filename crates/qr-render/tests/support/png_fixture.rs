use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = r#"safe/<script>alert("payload")</script>"#;
pub const SHA256: &str = "aa6b541af70eecde35a7d05f73fa8dcca5d7faa66819ea8ace380b4ddd3d6bc1";

pub fn artifact() -> Vec<u8> {
    let encoded = encode(EncodeRequest::first_fit(
        PAYLOAD,
        ErrorCorrection::Medium,
        Version::try_from(8).unwrap(),
    ))
    .unwrap();
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap(),
    )
    .unwrap();
    render_png(&model).unwrap()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
