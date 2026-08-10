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
        "9d0e8347404fd61239801d7db7de43071ab46d2f3bad7e6e9580f63be29b296c",
        "f7603652aa0f911b0cd641d3f3f8b40c611c7d116cff4fe885771d7cb3e71710",
    ],
    [
        "47290bcc76ce5d0b8c85a8d9a07cc148abe6fc252ddbcc8da6ad33dc6d3e5466",
        "c21d9a95a01fecaa600bb38e11df41f5a7ff8d4fdae38fc7608874a33e7e0c46",
    ],
    [
        "ba1ec2918aeb33b7a72d08252a27084aadac749274e2d3cc90ab7609c45c2fe6",
        "70135597571215a404d67d81439f7e39642308c69da243b52c7aa710841331ba",
    ],
    [
        "7cfefc0002a220abfcd875a1d35fe092dd252765609c1701739a7b3261a28fea",
        "82eb00d2b45d5832d8e410720ca76b23230f6a485d87256a9d5b7b0771775765",
    ],
    [
        "29aa76534949ce0b0e986ba70022892e776b45341050a3a6dafd51a83a17bcbb",
        "d198c58ad515ac4928f1acf868919e0ee3420591b37a9633aae4975920d6d062",
    ],
    [
        "81b86b0c8158195fd07020afba292c72b041174d2517014af5dee5259144da3b",
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
