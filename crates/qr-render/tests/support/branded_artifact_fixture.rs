use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    BRANDED_LOGO_VERSION, LogoStyle, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png,
    render_svg,
};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = "cross-target branded artifact";
pub const ENABLED_PROFILE_INDICES: [usize; 5] = [0, 1, 2, 3, 4];
pub const SHA256: [[&str; 2]; 5] = [
    [
        "bee8532a83dd1a11b1953616f50bb32c8666bdb7b159033ace3ff2dd9231cbab",
        "5d8b876247505411e62d2e11781063f5db525b147de1cb7050af5347dcc2b174",
    ],
    [
        "7b3c0247549b399b4afd697eb09c2070901cab9ce24bb64066acb22cfdaedd4f",
        "4ec62f4fbcfd579c8f76099c72f564c485a6b90b9ab2ae3ce9bc7196a72f8585",
    ],
    [
        "0f987440039d4b3988777f3e87460e05d83eadc94c63176dd90fc6d7d9dc498f",
        "b4bef6d193cbf9a4fe2b1b54355124f98f78e9380b4b3479d85685cf47f8e98e",
    ],
    [
        "48c591b6e80d44ca2a6e53bbb95d0b9e84fbab674b5515f989f816521981c295",
        "05f5dd50e689bc703ee11f865533c19e405b54c73a5cd79b6fc32e3368081020",
    ],
    [
        "fa5845337fc3ca37edb7d26ff6355989bd2c38369e1f2ea001b36abdacbaabaa",
        "b301cce425117f6aa9b894304f9b3f563d12d24f5b9d8906da19a0e75c2fbbe7",
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

pub fn hashes() -> [[String; 2]; 5] {
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
