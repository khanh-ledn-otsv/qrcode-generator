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
        "2d258fc1c34e71c84f420b7a90ebc45a521d17cfba8f8f231202479567a068c5",
        "5f8e3e9314ce4cb8bc7e0317842f3fd376dac171414009c0da18630660b712d2",
    ],
    [
        "11017e2c69e30834ac923d9ef5f797682abc279466e872993311093e9bdc62f0",
        "9f85b079e778b425dd7cafdeec41dd053c4a1c0f0b12b46caa4b0eb6d9c095e2",
    ],
    [
        "6c64424340bf0ffc17396027aea2bb686bac7c7d9c1bf5cf57f6f3704b4d260f",
        "efbcddcc0dd5d00262d7f8b7b781274e9da491ff17b17fb820d7890fbefc21e2",
    ],
    [
        "870bcb36b5ce71cb09635c94cf91c2fd25b7c5681f26bd7c3ff5ecc30d2ea8bd",
        "be60c556301826cac354b2e5d9fc89712d330d68a84f67bb6f12e6bfa7749071",
    ],
    [
        "4fece78235e4bb83cfc6cdd7105a0acc9d4448b7068bef309b362b6b020838a3",
        "a0cdd8e4d3e0808902f4a483423ada5450047cf92c0e55cffa5ed4101703fe6a",
    ],
    [
        "d4230fd87758c5911cb8cd29f9951494c4b32e37a52715f1bc2ce9c1125f3cf2",
        "cffd4b8cf5d7e71762f144a54ece875021885270a63c6f1a0cd9c451e063296e",
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
