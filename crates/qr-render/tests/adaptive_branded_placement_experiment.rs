#[allow(dead_code)]
#[path = "support/branded_geometry.rs"]
mod branded_geometry;
#[path = "support/raster.rs"]
mod raster;

use std::error::Error;
use std::fs;
use std::path::Path;

use branded_geometry::{
    CandidateAppearance, FunctionTreatment, LogoCandidate, render_candidate_png,
    render_candidate_svg,
};
use fixture_tool::{
    DecodeExpectation, EciAssignment as FixtureEci, ErrorCorrection as FixtureEcc, FixtureManifest,
    QrVersion, ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{ProfileId, SUPPORTED_PROFILES};
use serde::{Deserialize, Serialize};

const VERSION: u8 = 10;
const WIDTHS: [u32; 4] = [10, 11, 12, 13];
const VERTICAL_OFFSETS: [i32; 3] = [-6, 0, 6];
const PAYLOADS: [(&str, &str); 7] = [
    ("short-url", "https://example.test/a"),
    (
        "one-news-url",
        "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya",
    ),
    ("numeric", "12345678901234567890"),
    ("alphanumeric", "APPROVED LOGO 123"),
    ("ascii-byte", "lowercase-logo-byte"),
    ("utf8-eci26", "café logo"),
    (
        "dense-url",
        "https://example.test/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ),
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u8,
    decoder: Decoder,
    profile: String,
    version: u8,
    ecc: String,
    payload_classes: Vec<String>,
    artifact_paths: Vec<String>,
    candidate_widths_modules: Vec<u32>,
    candidate_vertical_offsets_modules: Vec<i32>,
    outcomes: Vec<CandidateOutcome>,
    selected: SelectedCandidate,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Decoder {
    tool: String,
    version: String,
    source_commit: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateOutcome {
    source_width_modules: u32,
    vertical_offset_modules: i32,
    source_ten_thousandths: Option<[u32; 4]>,
    knockout_modules: Option<[u32; 4]>,
    attempted: usize,
    decoded: usize,
    outcome: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SelectedCandidate {
    source_width_modules: u32,
    vertical_offset_modules: i32,
    reason: String,
}

#[test]
#[ignore = "runs the focused manifest-pinned adaptive placement experiment"]
fn compare_and_record_adaptive_branded_placement_candidates() -> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest =
        FixtureManifest::load_and_verify(workspace.join("tests/fixtures/manifest.json"))?;
    let checkout = workspace.join("tests/oracles/zxing-cpp");
    let decoder = ZxingDecoder::new(
        checkout.join("build/example/ZXingReader"),
        manifest.decoder().version(),
        &checkout,
        manifest.decoder().source_commit(),
    );
    let decoder = decoder.verify()?;
    let profile = SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == ProfileId::AdaptiveBranded)
        .ok_or("Adaptive Branded profile is missing")?;
    let version = Version::new(VERSION)?;
    let output = tempfile::tempdir()?;
    let geometry_seed = encode(EncodeRequest::with_version_range(
        PAYLOADS[1].1,
        ErrorCorrection::High,
        version,
        version,
    ))?;
    let mut outcomes = Vec::new();

    for width in WIDTHS {
        for vertical_offset in VERTICAL_OFFSETS {
            let Some(candidate) =
                LogoCandidate::checked_shifted(geometry_seed.modules(), width, vertical_offset)
            else {
                outcomes.push(CandidateOutcome {
                    source_width_modules: width,
                    vertical_offset_modules: vertical_offset,
                    source_ten_thousandths: None,
                    knockout_modules: None,
                    attempted: 0,
                    decoded: 0,
                    outcome: "unsafe-protected-module-intersection".to_owned(),
                });
                continue;
            };
            let appearance = CandidateAppearance {
                dot_diameter_thousandths: 450,
                function_treatment: FunctionTreatment::NonFinderDots,
                logo: Some(candidate),
            };
            let mut decoded = 0;
            for (label, payload) in PAYLOADS {
                let encoded = encode(EncodeRequest::with_version_range(
                    payload,
                    ErrorCorrection::High,
                    version,
                    version,
                ))?;
                let expected = DecodeExpectation {
                    payload: payload.as_bytes().to_vec(),
                    version: QrVersion::new(VERSION)?,
                    ecc: FixtureEcc::H,
                    eci_assignment: encoded
                        .eci_assignment()
                        .map(|assignment| FixtureEci::try_from(assignment.number()))
                        .transpose()?,
                };
                let stem = format!("adaptive-v10-w{width}-y{vertical_offset}-{label}");
                let png_path = output.path().join(format!("{stem}-png.png"));
                fs::write(
                    &png_path,
                    render_candidate_png(&encoded, profile, appearance, false)?,
                )?;
                decoder.inspect_and_compare(&png_path, &expected)?;
                decoded += 1;

                let svg = render_candidate_svg(&encoded, profile, appearance, false)?;
                let dimensions = profile.png_dimensions();
                let svg_path = output.path().join(format!("{stem}-svg.png"));
                raster::rasterize_svg(&svg, dimensions.width().get(), dimensions.height().get())?
                    .save_png(&svg_path)?;
                decoder.inspect_and_compare(&svg_path, &expected)?;
                decoded += 1;
            }
            outcomes.push(CandidateOutcome {
                source_width_modules: width,
                vertical_offset_modules: vertical_offset,
                source_ten_thousandths: Some([
                    candidate.source_left_ten_thousandths(),
                    candidate.source_top_ten_thousandths(),
                    candidate.source_width_ten_thousandths(),
                    candidate.source_height_ten_thousandths(),
                ]),
                knockout_modules: Some(candidate.knockout()),
                attempted: PAYLOADS.len() * 2,
                decoded,
                outcome: "decoded".to_owned(),
            });
        }
    }

    let policy = Policy {
        schema_version: 1,
        decoder: Decoder {
            tool: "ZXing-C++ ZXingReader".to_owned(),
            version: manifest.decoder().version().to_owned(),
            source_commit: manifest.decoder().source_commit().to_owned(),
        },
        profile: "Adaptive Branded".to_owned(),
        version: VERSION,
        ecc: "H".to_owned(),
        payload_classes: PAYLOADS
            .iter()
            .map(|(label, _)| (*label).to_owned())
            .collect(),
        artifact_paths: vec!["native-png".to_owned(), "rasterized-svg".to_owned()],
        candidate_widths_modules: WIDTHS.to_vec(),
        candidate_vertical_offsets_modules: VERTICAL_OFFSETS.to_vec(),
        outcomes,
        selected: SelectedCandidate {
            source_width_modules: 13,
            vertical_offset_modules: -6,
            reason:
                "largest fully decoded function-safe candidate; upward wins the equal-distance tie"
                    .to_owned(),
        },
    };
    let destination = workspace.join("docs/generated/adaptive-branded-placement-policy.json");
    fs::write(
        destination,
        format!("{}\n", serde_json::to_string_pretty(&policy)?),
    )?;
    Ok(())
}

#[test]
fn committed_adaptive_placement_policy_covers_every_candidate_and_full_decode() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/generated/adaptive-branded-placement-policy.json");
    let policy: Policy = serde_json::from_slice(
        &fs::read(path).expect("adaptive placement policy must be committed"),
    )
    .expect("adaptive placement policy must use the strict schema");
    assert_eq!(policy.schema_version, 1);
    assert_eq!(policy.decoder.tool, "ZXing-C++ ZXingReader");
    assert_eq!(policy.decoder.version, "3.0.2");
    assert_eq!(
        policy.decoder.source_commit,
        "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825"
    );
    assert_eq!(policy.profile, "Adaptive Branded");
    assert_eq!(policy.version, VERSION);
    assert_eq!(policy.ecc, "H");
    assert_eq!(policy.candidate_widths_modules, WIDTHS);
    assert_eq!(policy.candidate_vertical_offsets_modules, VERTICAL_OFFSETS);
    assert_eq!(policy.outcomes.len(), WIDTHS.len() * VERTICAL_OFFSETS.len());
    for outcome in &policy.outcomes {
        if outcome.outcome == "decoded" {
            assert_eq!(outcome.attempted, PAYLOADS.len() * 2);
            assert_eq!(outcome.decoded, outcome.attempted);
            assert!(outcome.source_ten_thousandths.is_some());
            assert!(outcome.knockout_modules.is_some());
        } else {
            assert_eq!(outcome.outcome, "unsafe-protected-module-intersection");
            assert_eq!(outcome.attempted, 0);
            assert_eq!(outcome.decoded, 0);
        }
    }
    assert_eq!(policy.selected.source_width_modules, 13);
    assert_eq!(policy.selected.vertical_offset_modules, -6);
}
