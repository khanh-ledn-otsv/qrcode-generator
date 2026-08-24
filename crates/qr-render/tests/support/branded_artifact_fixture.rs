use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    BRANDED_LOGO_VERSION, ForegroundTheme, LogoStyle, RenderModel, RenderOptions,
    SUPPORTED_PROFILES, render_png, render_svg,
};
use sha2::{Digest, Sha256};

pub const PAYLOAD: &str = "cross-target branded artifact";
pub const ENABLED_PROFILE_INDICES: [usize; 5] = [0, 1, 2, 3, 4];
pub const SHA256: [[&str; 2]; 5] = [
    [
        "2b99c437c5acf5c9dd1a760c1a95e26631f8e3b6d8d1d52e2133a45cbc51a001",
        "9f6ac2b6a5ec87295241df30c64b854860b2e816b28d8eaeea429ff8b3d71ed8",
    ],
    [
        "c355dc46f90879873a1e1ce7e951181f2d4f99b1c8f37e252652b761e36c0179",
        "cea27bef2c98b9a45c3d6231633cf24679973e9ec2f9e4042d691d8e19b13df7",
    ],
    [
        "6a04ec5ef969a9cd4d183cfe875f4800c39749cc386030c5ffc1f00a64b6e261",
        "8edb35e689fb15837546053fc4de4a93e66f1b6f7a19e374d41d419e33750be6",
    ],
    [
        "684519e4a83da0e96cd431b3a0a9bab6d890cbf69b2c88ab15cc74f735bd98ad",
        "3b9760350a61fbcd198357d26d4a6d21151d2f9b9844b4bde8b571bb12f74dec",
    ],
    [
        "7fe0ca72be4a8467b8fd2f76945dd709dbeb66ecf1f36b0227ac47f0b63d8a88",
        "7dd883fec5e2cb1a54151bdd22160fd969537b091b367578bac7dd5be5c8b3f2",
    ],
];

#[allow(dead_code)]
pub const BLACK_SHA256: [&str; 2] = [
    "872e6462c0e6d347d58093d5ce3655e939690cee8fb6ff1d2c3420709c36bcda",
    "5c3eaedd8a148468c1f3c1905e3c2943709025cb27ab891257154e554d3c68e0",
];

pub fn artifacts(profile_index: usize) -> (Vec<u8>, Vec<u8>) {
    artifacts_with_foreground(profile_index, ForegroundTheme::Magenta)
}

pub fn artifacts_with_foreground(
    profile_index: usize,
    foreground: ForegroundTheme,
) -> (Vec<u8>, Vec<u8>) {
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
            .unwrap()
            .with_foreground_theme(foreground)
            .unwrap(),
    )
    .unwrap();
    (
        render_svg(&model).unwrap().into_bytes(),
        render_png(&model).unwrap(),
    )
}

#[allow(dead_code)]
pub fn black_content_hashes() -> [String; 2] {
    let (svg, png) = artifacts_with_foreground(1, ForegroundTheme::Black);
    [sha256_hex(&svg), sha256_hex(&png)]
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
