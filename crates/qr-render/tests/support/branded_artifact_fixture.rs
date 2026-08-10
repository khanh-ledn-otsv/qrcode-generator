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
        "82c1c5a8e3f1fb196434761d7281fad6c0e3df5ff0d77c5b78b597dd94c8b362",
        "bae494fd8bee9c962ed913e1856d662cd245289c006f7174eb544a91fd8b7dcc",
    ],
    [
        "6285420a690d11b5569ec168158a593af2883c391db63775ffcdc26733160ee5",
        "68faf07edc13d19ae9bf50743616cdee086985922f4c1d89bd1da03bf6cd30e4",
    ],
    [
        "ee49d27cafa71f3222c76587c2935f5340b070076719672cb7fc8dbce4992623",
        "6bb6cd7dbbd73d26f1fdcf862ade1c19873edaae42b63d8ca201662ad63f36c5",
    ],
    [
        "200fd48511ac86579c8890b2be5a499aa1ad1f43a854ea35651451b65996a16c",
        "34e8d37ef14f5efe73882efa1a3d6e321713dd2e64540cc603661bf4114564aa",
    ],
    [
        "828bcf09e2ac2ab01eeaf46114baff79eeeeb568d1c53e00634f34a98a79a0c1",
        "9d5d748530b6dad43e7f9669e83fd09a6352b2f11b4ee185f974eed6d8684b42",
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
