#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

#[path = "support/branded_artifact_fixture.rs"]
mod branded_artifact_fixture;
#[path = "support/inline_version_six_artifact_fixture.rs"]
mod inline_version_six_artifact_fixture;
#[path = "support/png_fixture.rs"]
mod png_fixture;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{LogoStyle, RenderError, RenderModel, RenderOptions, SUPPORTED_PROFILES};

#[wasm_bindgen_test]
fn png_bytes_match_the_native_artifact_fixture() {
    let first = png_fixture::artifact();
    let second = png_fixture::artifact();

    assert_eq!(first, second);
    assert_eq!(png_fixture::sha256_hex(&first), png_fixture::SHA256);
}

#[wasm_bindgen_test]
fn branded_svg_and_png_bytes_match_every_enabled_native_fixture() {
    assert_eq!(
        branded_artifact_fixture::hashes(),
        branded_artifact_fixture::SHA256
    );
}

#[wasm_bindgen_test]
fn unbranded_inline_version_six_svg_and_png_match_the_native_fixture() {
    assert_eq!(
        inline_version_six_artifact_fixture::hashes(),
        inline_version_six_artifact_fixture::SHA256
    );
}

#[wasm_bindgen_test]
fn branded_version_seven_is_rejected_on_wasm() {
    let version_seven = Version::new(7).unwrap();
    let encoded = encode(EncodeRequest::with_version_range(
        &"a".repeat(59),
        ErrorCorrection::High,
        version_seven,
        version_seven,
    ))
    .unwrap();
    let options = RenderOptions::safe(SUPPORTED_PROFILES[3])
        .unwrap()
        .with_logo(LogoStyle::Bundled)
        .unwrap();

    assert_eq!(
        RenderModel::new(&encoded, options).unwrap_err(),
        RenderError::UnsafeLogoGeometry
    );
}
