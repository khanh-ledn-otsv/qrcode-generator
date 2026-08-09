use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = r#"safe/<script>alert("payload")</script>"#;
pub const SHA256: &str = "84fbe86bf19f9d1b4aff3416e784d9ac5969e3163145779ccc9a16805b43111c";

pub fn artifact() -> Vec<u8> {
    let encoded = encode(EncodeRequest {
        text: PAYLOAD,
        ecc: ErrorCorrection::Medium,
        max_version: Version::try_from(8).unwrap(),
    })
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
