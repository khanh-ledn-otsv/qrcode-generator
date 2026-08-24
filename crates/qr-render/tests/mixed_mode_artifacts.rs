use std::fs;
use std::path::Path;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodingMode, Version, encode};
use qr_render::{OutputProfile, ProfileId, RenderModel, RenderOptions, render_png, render_svg};
use sha2::{Digest, Sha256};

#[test]
fn mixed_mode_matrices_and_render_artifacts_match_reviewed_evidence() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture: serde_json::Value = serde_json::from_slice(
        &fs::read(workspace.join("docs/generated/mixed-mode-oracle-fixtures.json"))
            .expect("mixed-mode evidence is readable"),
    )
    .expect("mixed-mode evidence is valid JSON");

    for case in fixture["cases"]
        .as_array()
        .expect("mixed-mode cases are an array")
    {
        let payload = case["payload"].as_str().expect("payload is text");
        let encoded = encode(EncodeRequest::first_fit(
            payload,
            ErrorCorrection::Low,
            Version::new(8).expect("Version 8 is valid"),
        ))
        .expect("mixed-mode evidence payload encodes");
        assert_eq!(encoded.mode(), EncodingMode::Mixed);
        assert_eq!(
            u64::from(encoded.version().number()),
            case["selected_version"]
                .as_u64()
                .expect("version is unsigned")
        );
        assert_eq!(
            u64::from(encoded.mask().number()),
            case["selected_mask"].as_u64().expect("mask is unsigned")
        );
        assert_eq!(
            format!("{:016x}", matrix_fingerprint(&encoded)),
            case["matrix_fnv1a64"]
                .as_str()
                .expect("matrix fingerprint is text")
        );

        let evidence_profile = OutputProfile::try_new(
            ProfileId::Standard,
            qr_render::PixelDimensions::square(120),
            qr_render::PixelDimensions::square(360),
            Version::new(1).expect("Version 1 is valid"),
            Version::new(8).expect("Version 8 is valid"),
        )
        .expect("mixed-mode evidence profile is valid");
        let model = RenderModel::new(
            &encoded,
            RenderOptions::safe(evidence_profile).expect("evidence profile is safe"),
        )
        .expect("mixed-mode matrix renders");
        assert_eq!(
            sha256_hex(render_svg(&model).expect("SVG renders").as_bytes()),
            case["content_svg_sha256"]
                .as_str()
                .expect("SVG hash is text")
        );
        assert_eq!(
            sha256_hex(&render_png(&model).expect("PNG renders")),
            case["content_png_sha256"]
                .as_str()
                .expect("PNG hash is text")
        );
    }
}

fn matrix_fingerprint(encoded: &qr_core::EncodedQr) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    let size = encoded.modules().size();
    for y in 0..size {
        for x in 0..size {
            let byte = if encoded
                .modules()
                .module(x, y)
                .expect("evidence matrix is complete")
                .is_dark()
            {
                b'1'
            } else {
                b'0'
            };
            value = (value ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
        value = (value ^ u64::from(b'\n')).wrapping_mul(0x100_0000_01b3);
    }
    value
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
