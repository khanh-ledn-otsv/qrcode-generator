#[path = "support/versions.rs"]
mod versions;

use std::error::Error;
use std::fs;
use std::path::Path;

use fixture_tool::{
    DecodeExpectation, ErrorCorrection as FixtureEcc, FixtureManifest, QrVersion, ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};

#[test]
#[ignore = "requires the manifest-pinned ZXing-C++ checkout and reader"]
fn emitted_pngs_independently_decode_across_profiles_and_versions() -> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest_path = workspace.join("tests/fixtures/manifest.json");
    let manifest = FixtureManifest::load_and_verify(&manifest_path)?;
    let source = workspace.join("tests/oracles/zxing-cpp");
    let reader = source.join("build/example/ZXingReader");
    let decoder = ZxingDecoder::new(
        reader,
        manifest.decoder().version(),
        &source,
        manifest.decoder().source_commit(),
    );
    let output = tempfile::tempdir()?;

    let mut case_index = 0;
    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for version in 1..=profile.maximum_version().number() {
            let text = payload_for_version(version, case_index);
            let encoded = encode(EncodeRequest {
                text: &text,
                ecc: ErrorCorrection::Medium,
                max_version: Version::try_from(version)?,
            })?;
            if encoded.version().number() != version {
                return Err(format!("case {case_index} selected the wrong version").into());
            }
            let model = RenderModel::new(&encoded, RenderOptions::safe(profile)?)?;
            let png = render_png(&model)?;
            let artifact = output
                .path()
                .join(format!("png-profile-{profile_index}-version-{version}.png"));
            fs::write(&artifact, png)?;

            decoder.inspect_and_compare(
                &artifact,
                &DecodeExpectation {
                    payload: text.into_bytes(),
                    version: QrVersion::new(version)?,
                    ecc: FixtureEcc::M,
                    eci_assignment: None,
                },
            )?;
            case_index += 1;
        }
    }
    Ok(())
}

fn payload_for_version(version: u8, case_index: usize) -> String {
    let length = versions::first_byte_length(version);
    let prefix = char::from(b'0' + u8::try_from(case_index % 10).unwrap_or_default());
    format!("{prefix}{}", "a".repeat(length - 1))
}
