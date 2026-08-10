use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = r#"safe/<script>alert("payload")</script>"#;
pub const SHA256: &str = "b09c673c0ed4b3d2a976817763da036343b1cf0e1b12e12b0f81eeadb6ddd097";

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
