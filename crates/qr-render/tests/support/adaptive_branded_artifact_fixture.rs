use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    LogoStyle, ProfileId, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg,
};
use sha2::{Digest, Sha256};

pub const CASES: [(&str, u8); 5] = [
    ("adaptive branded version 6", 6),
    ("adaptive branded version 7", 7),
    ("adaptive branded version 8", 8),
    ("adaptive branded version 9", 9),
    (
        "https://example.test/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        10,
    ),
];

pub const SHA256: [[&str; 2]; 5] = [
    [
        "dd592d85a3480eb7b65ed1407bd226979ee7fa7d58558a3b8a3d47fc3bb2c767",
        "dae1c7165d015239cb1c519f1f97cf2b8196f8a020602a2877e47027c838b042",
    ],
    [
        "c732031d99f6430be50a01470b53c18114874b34c28cff70b59c00e435ed69a4",
        "7695db15fbf0ea4bfb5594ea94fa79f4f70f280b9331f173ed328b1ffd1c2307",
    ],
    [
        "16529c4d8ba105fa7065c649a3333687f7e8ca184abc150e86c3d0e27ebcc732",
        "8f5e49387d17862ac237afb9ea65ba1655e5562772e64923f34984c5a83ca38f",
    ],
    [
        "a3bfc07b9b42f3f9f9060765645ccdd7f63a6e2db30cec432b95b6b0142647ff",
        "06dabd7a9479babf5ff132d298b270fad1094821a125d44819cd9bb589a7963a",
    ],
    [
        "267dd3d841e69a5189026fb8f566826f259c2e8fcc59ec366a7847f98ab67580",
        "7b60a292ad89cccbbb90669e6e75f91d13abb1676fdc1888d961e45921510ea1",
    ],
];

pub fn artifacts(payload: &str, version_number: u8) -> (Vec<u8>, Vec<u8>) {
    let version = Version::new(version_number).unwrap();
    let encoded = encode(EncodeRequest::with_version_range(
        payload,
        ErrorCorrection::High,
        version,
        version,
    ))
    .unwrap();
    let profile = SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == ProfileId::AdaptiveBranded)
        .unwrap();
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(profile)
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

#[cfg(not(target_arch = "wasm32"))]
pub fn provenance_hashes() -> [[String; 2]; 5] {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/adaptive-branded-artifacts.json")).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["synthetic"], true);
    assert_eq!(
        manifest["payload_provenance"],
        "Generated non-sensitive version-labelled and 110-byte URL regression payloads"
    );
    assert_eq!(manifest["profile"], "Adaptive Branded");
    assert_eq!(manifest["ecc"], "H");
    assert_eq!(manifest["logo"], "bundled ONE");
    assert_eq!(manifest["verification"]["state"], "accepted");

    std::array::from_fn(|index| {
        let case = &manifest["cases"][index];
        assert_eq!(case["payload"], CASES[index].0);
        assert_eq!(case["version"], CASES[index].1);
        assert_eq!(
            case["payload_sha256"],
            sha256_hex(CASES[index].0.as_bytes())
        );
        [
            case["svg_sha256"].as_str().unwrap().to_owned(),
            case["png_sha256"].as_str().unwrap().to_owned(),
        ]
    })
}

pub fn hashes() -> [[String; 2]; 5] {
    CASES.map(|(payload, version)| {
        let (svg, png) = artifacts(payload, version);
        [sha256_hex(&svg), sha256_hex(&png)]
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
