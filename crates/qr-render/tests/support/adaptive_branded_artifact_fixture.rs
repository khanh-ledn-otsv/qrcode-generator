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
        "b1161220fbc928b6f070d3b8a0e83bea7d55ffe7807fa07f36d32640f99a11b5",
        "aeab1f239dcf9c7733ed3ffbaa426f044a2456f15c74e04cb8961f45af1dc21e",
    ],
    [
        "4b5f64cfb9947c7e8c74b71d83e72bfc8042880bfbfede99b298b9b3923b9d99",
        "a54f95adadb043f4f32c73bce1d7239baee2417d18af84ebd41d076fa9fda1f1",
    ],
    [
        "6de015675dca2f69a9090a2991e7718530b373254212b0cc69094a03cc7954ff",
        "6b34b91742a3e77d3b300ca7d4736a8c73857183b85201dadc9a1e10e648483e",
    ],
    [
        "96453db85efe039f5cc6a68e8999abe1a79e59d1619073a80fb1ee3e33e749dd",
        "850753dfb640429ac3bcfb05fbd4b7a24011d0106e6a3ab511595cab02353d91",
    ],
    [
        "95b0b08f13c34d8f7114ecbb4445ba08a77dfccfb9894fcf2fc34f40c156cf4a",
        "06dc5e3f10fbe006fee721ce25d8071500439907e2b3ffdd5305c7a6a410d814",
    ],
    [
        "f83202cd164e04efffc34d5c1625d159513a6a21b563563bd4270a32370227af",
        "1510d473bc79a40916843516790f04f221bc770314b9811feb85f68cbe62a086",
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
