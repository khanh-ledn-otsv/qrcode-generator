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
        "dfc18037980a1a848d02bccf6cae97ecabacf268fdb84bbe5c421032e2b00061",
        "f7603652aa0f911b0cd641d3f3f8b40c611c7d116cff4fe885771d7cb3e71710",
    ],
    [
        "7d5bcab343b6d275870fd485cb473a68f088d180b8d648691714c3ecc3cf3109",
        "c21d9a95a01fecaa600bb38e11df41f5a7ff8d4fdae38fc7608874a33e7e0c46",
    ],
    [
        "caf225b9e4683d763cb12e463c852befb308156fc501d78e3fff1d47a1308d77",
        "70135597571215a404d67d81439f7e39642308c69da243b52c7aa710841331ba",
    ],
    [
        "b55d3f15f1afe4c162af7ccad385390231ff7d1eb1c4ed4fd7f6a1c140c03dd1",
        "82eb00d2b45d5832d8e410720ca76b23230f6a485d87256a9d5b7b0771775765",
    ],
    [
        "5f1cc32a6d17e8fee82b688fdeda14c098ccd246427bd08af25e57131e4d3b7f",
        "d198c58ad515ac4928f1acf868919e0ee3420591b37a9633aae4975920d6d062",
    ],
    [
        "96ae8bd6ae3d4820afa945dddb059caebf0bc0ce090cff368da3e5ebfc44d4d1",
        "90bb858cff8c329277751457531c837d28cdf533bd931ce4df008c3a9f16ba46",
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
