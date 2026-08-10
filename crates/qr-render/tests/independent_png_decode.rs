#[allow(dead_code)]
#[path = "support/styling.rs"]
mod styling;
#[allow(dead_code)]
#[path = "support/versions.rs"]
mod versions;

use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use fixture_tool::{
    DecodeExpectation, EciAssignment, ErrorCorrection as FixtureEcc, FixtureManifest, QrVersion,
    ZxingDecoder,
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
    let mut matrix_outcomes = Vec::new();
    let mut decoded_matrix_labels = HashSet::new();

    let mut case_index = 0;
    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for version in 1..=profile.maximum_version().number() {
            let text = payload_for_version(version, case_index);
            let encoded = encode(EncodeRequest::first_fit(
                &text,
                ErrorCorrection::Medium,
                Version::try_from(version)?,
            ))?;
            if encoded.version().number() != version {
                return Err(format!("case {case_index} selected the wrong version").into());
            }
            let model = RenderModel::new(&encoded, RenderOptions::safe(profile)?)?;
            let png = render_png(&model)?;
            let artifact = output
                .path()
                .join(format!("png-profile-{profile_index}-version-{version}.png"));
            fs::write(&artifact, png)?;

            decoder
                .inspect_and_compare(
                    &artifact,
                    &DecodeExpectation {
                        payload: text.into_bytes(),
                        version: QrVersion::new(version)?,
                        ecc: FixtureEcc::M,
                        eci_assignment: None,
                    },
                )
                .map_err(|error| {
                    format!("plain PNG profile {profile_index} version {version}: {error}")
                })?;
            case_index += 1;
        }
    }

    for case in styling::approved_decode_cases()? {
        let label = case.label.clone();
        let model = RenderModel::new(&case.encoded, case.options)?;
        let png = render_png(&model)?;
        let artifact_sha256 = styling::sha256_hex(&png);
        let effective_png = png;
        let decoder_input_sha256 = styling::sha256_hex(&effective_png);
        let artifact = output
            .path()
            .join(format!("png-approved-{}.png", case.label));
        fs::write(&artifact, effective_png)?;
        decoder
            .inspect_and_compare(
                &artifact,
                &DecodeExpectation {
                    payload: case.payload.clone(),
                    version: QrVersion::new(case.encoded.version().number())?,
                    ecc: fixture_ecc(case.ecc),
                    eci_assignment: case
                        .eci_assignment
                        .map(EciAssignment::try_from)
                        .transpose()?,
                },
            )
            .map_err(|error| format!("approved PNG case {}: {error}", case.label))?;
        decoded_matrix_labels.insert(label);
        matrix_outcomes.push(styling::decoded_evidence(
            &case,
            "png",
            artifact_sha256,
            decoder_input_sha256,
        ));
    }

    let records = styling::approved_combination_records()?;
    let expected_renderable = records
        .iter()
        .filter(|record| record.outcome.is_renderable())
        .count();
    if decoded_matrix_labels.len() != expected_renderable {
        return Err(format!(
            "decoded {} approved PNG matrix rows; expected {expected_renderable}",
            decoded_matrix_labels.len()
        )
        .into());
    }
    for record in records {
        if let Some(error) = record.outcome.expected_error() {
            matrix_outcomes.push(styling::invalid_evidence(&record, "png", error));
        }
    }
    matrix_outcomes.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    if let Some(evidence_dir) = std::env::var_os("QR_RELEASE_EVIDENCE_DIR") {
        let configured = Path::new(&evidence_dir);
        let evidence_dir = if configured.is_absolute() {
            configured.to_path_buf()
        } else {
            workspace.join(configured)
        };
        fs::create_dir_all(&evidence_dir)?;
        fs::write(
            evidence_dir.join("approved-output-png.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "rows": matrix_outcomes,
            }))?,
        )?;
    }
    Ok(())
}

fn fixture_ecc(ecc: ErrorCorrection) -> FixtureEcc {
    match ecc {
        ErrorCorrection::Low => FixtureEcc::L,
        ErrorCorrection::Medium => FixtureEcc::M,
        ErrorCorrection::Quartile => FixtureEcc::Q,
        ErrorCorrection::High => FixtureEcc::H,
    }
}

fn payload_for_version(version: u8, _case_index: usize) -> String {
    let length = versions::first_byte_length(version);
    "a".repeat(length)
}
