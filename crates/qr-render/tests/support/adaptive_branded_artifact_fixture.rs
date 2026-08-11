use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    LogoStyle, ProfileId, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg,
};
use sha2::{Digest, Sha256};

pub const CASES: [(&str, u8); 6] = [
    ("adaptive version 6", 6),
    ("adaptive version 7", 7),
    ("adaptive version 8", 8),
    ("adaptive version 9", 9),
    (
        "https://example.test/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        10,
    ),
    ("adaptive version 11", 11),
];

pub const SHA256: [[&str; 2]; 6] = [
    [
        "531174a5982a13502495a37c7c1fee7a44f4a7d580127dd178c5ebfc8ccf53f3",
        "967b8a2b00b1b15bb9c9fd8175a7cdd618eef01fc77ddc97269f56daa5519ea7",
    ],
    [
        "8db9b458c2b3ecee3df27da826a240458a3f6aae59af183f2a93bf25e43573f3",
        "4ec6b5f0b8fe1cc78814a085538ed4d7c5378a76faa6cafcfd674fb0284c6a59",
    ],
    [
        "46837c254bf026427b494d082555c9edcb65b3640dc90ab2e9da766b5fcc03be",
        "f77088cfd9c165462ba2ce03c8559a3c83c04982f39cec6ab2901ac6f317cbf8",
    ],
    [
        "a8651c61d436f2b8c19576ac709fa647a697384f996fed3dd958610fd759bf19",
        "ae502106f3bab6b5c0f9034b0fade1991570989863df711c5014ee3a158a527a",
    ],
    [
        "3314077eb6651bb02c61f931a360aea4f05e97fb2e27709531b1a22e63cc1917",
        "6e075b8cf6c6aee10e303e9b4cb247694fa915e5c40edecf2c2e7388c5e34a55",
    ],
    [
        "b4fbf5ef17efe3f10b711ce0677aa7307fefcc2719367c37c03891bc43f390eb",
        "7db6947605a4a68fd26bc6b23d86834b3a9f77ebb1db6b1bcf6876d86efab2ab",
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
        .find(|profile| profile.id() == ProfileId::Adaptive)
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
pub fn provenance_hashes() -> [[String; 2]; 6] {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/adaptive-branded-artifacts.json")).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["synthetic"], true);
    assert_eq!(
        manifest["payload_provenance"],
        "Generated non-sensitive version-labelled and 110-byte URL regression payloads"
    );
    assert_eq!(manifest["profile"], "Adaptive");
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

pub fn hashes() -> [[String; 2]; 6] {
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
