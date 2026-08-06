#![cfg(target_arch = "wasm32")]

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};
use sha2::{Digest, Sha256};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn png_bytes_match_the_native_artifact_fixture() {
    let payload = r#"safe/<script>alert("payload")</script>"#;
    let encoded = encode(EncodeRequest {
        text: payload,
        ecc: ErrorCorrection::Medium,
        max_version: Version::try_from(8).unwrap(),
    })
    .unwrap();
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap(),
    )
    .unwrap();

    let first = render_png(&model).unwrap();
    let second = render_png(&model).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        sha256_hex(&first),
        "139610a415ccf86ad47d932318abd86ec7d7dbbffe267df8a12f2001b2ef505d"
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
