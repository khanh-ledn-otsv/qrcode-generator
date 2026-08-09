use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    BRANDED_LOGO_VERSION, LogoStyle, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png,
    render_svg,
};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = "cross-target branded artifact";
pub const ENABLED_PROFILE_INDICES: [usize; 3] = [1, 2, 3];
pub const SHA256: [[&str; 2]; 3] = [
    [
        "5a4a3ebfeedf64c847c647c604b8c851cc5b71063d199f510634ec4a6ed9e0dd",
        "7fb69b3ce26483f28898046bb7251dc8835d988653b9a00fd6f1a6fcfe01bf8d",
    ],
    [
        "7d863f946c64f8caed9d09348c3b305c84e600b1d4cd14d26a71260765c70711",
        "c68813bffd50ac472014b04a903ef1e01ecad04dcb571d7931afc91a655759ca",
    ],
    [
        "88455b572c015e2644f85b41dbe631c74944939d66f076e9838c18546422a5dc",
        "49c6e5d60bd8c0b8ac14b6988cf91e510ef83723792059f1fddc38c8e2ddfc36",
    ],
];

pub fn artifacts(profile_index: usize) -> (Vec<u8>, Vec<u8>) {
    let encoded = encode(EncodeRequest::with_version_range(
        PAYLOAD,
        ErrorCorrection::High,
        BRANDED_LOGO_VERSION,
        BRANDED_LOGO_VERSION,
    ))
    .unwrap();
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[profile_index])
            .unwrap()
            .with_logo(LogoStyle::Bundled)
            .unwrap(),
    )
    .unwrap();
    (
        render_svg(&model).unwrap().into_bytes(),
        render_png(&model).unwrap(),
    )
}

pub fn hashes() -> [[String; 2]; 3] {
    ENABLED_PROFILE_INDICES.map(|profile_index| {
        let (svg, png) = artifacts(profile_index);
        [sha256_hex(&svg), sha256_hex(&png)]
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
