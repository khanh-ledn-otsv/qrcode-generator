use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = "cross-target unbranded Inline Version 6 artifact";
pub const SHA256: [&str; 2] = [
    "4dfcdbac53bad49e5757dd67c0142725f8172d056afd7552439320296e833ab7",
    "6e06b6694c8269e7651f16fa89b2a47b7fe4a85971e3c9da02c7c561ca402461",
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
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[0]).unwrap(),
    )
    .unwrap();

    (
        render_svg(&model).unwrap().into_bytes(),
        render_png(&model).unwrap(),
    )
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
