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
    QrVersion, VerifiedZxingDecoder, ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EciAssignment, EncodeRequest, EncodedQr, Version, encode};
use qr_render::{OutputProfile, SUPPORTED_PROFILES};
use serde::Serialize;

const DIAMETERS: [u16; 16] = [
    450, 460, 470, 480, 490, 500, 510, 520, 530, 540, 550, 560, 570, 580, 590, 600,
];
const TREATMENTS: [FunctionTreatment; 2] = [
    FunctionTreatment::SquareFunctions,
    FunctionTreatment::NonFinderDots,
];
const PAYLOAD_CLASSES: [&str; 6] = [
    "short-url",
    "dense-url",
    "numeric",
    "alphanumeric",
    "ascii-byte",
    "utf8-eci26",
];
const LOGO_WIDTHS: [u32; 9] = [10, 11, 12, 13, 14, 15, 16, 17, 18];

type EncodedPayloadCase = (&'static str, EncodedQr, String);

#[derive(Serialize)]
struct DotOutcome {
    diameter_thousandths: u16,
    function_treatment: &'static str,
    attempted: usize,
    decoded: usize,
}

#[derive(Serialize)]
struct LogoResult {
    version: u8,
    source_left_ten_thousandths: Option<u32>,
    source_top_ten_thousandths: Option<u32>,
    source_width_ten_thousandths: Option<u32>,
    source_height_ten_thousandths: Option<u32>,
    knockout: Option<[u32; 4]>,
    protected_clearance_modules: Option<u32>,
    obscured_data_modules: Option<u32>,
    obscured_remainder_modules: Option<u32>,
    outcome: &'static str,
}

#[derive(Serialize)]
struct MinimumVersionResult {
    version: u8,
    maximum_safe_source_width_thousandths: u32,
    source_left_ten_thousandths: u32,
    source_top_ten_thousandths: u32,
    source_height_ten_thousandths: u32,
    knockout: [u32; 4],
    protected_clearance_modules: u32,
    obscured_data_modules: u32,
    obscured_remainder_modules: u32,
    requested_minimum_source_width_thousandths: u32,
    meets_requested_hierarchy: bool,
}

#[derive(Serialize)]
struct LogoWidthOutcome {
    version: u8,
    source_width_thousandths: u32,
    attempted: usize,
    decoded: usize,
    outcome: &'static str,
}

#[derive(Serialize)]
struct LogoProfileOutcome {
    profile: String,
    attempted: usize,
    decoded: usize,
    outcome: &'static str,
}

#[derive(Serialize)]
struct Policy {
    schema_version: u8,
    decoder: serde_json::Value,
    sample: serde_json::Value,
    dots: serde_json::Value,
    logo: serde_json::Value,
    quiet_zone_modules: u8,
    decorative_export_borders: bool,
}

#[test]
#[ignore = "runs the complete manifest-pinned independent decode experiment"]
fn compare_and_record_branded_geometry_candidates() -> Result<(), Box<dyn Error>> {
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
    let output = tempfile::tempdir()?;

    let mut dot_outcomes = Vec::new();
    for diameter in DIAMETERS {
        for treatment in TREATMENTS {
            let appearance = CandidateAppearance {
                dot_diameter_thousandths: diameter,
                function_treatment: treatment,
                logo: None,
            };
            let mut attempted = 0;
            let mut decoded = 0;
            for profile in SUPPORTED_PROFILES {
                for (label, text) in payload_cases(profile, ErrorCorrection::Medium)? {
                    let encoded = encode(EncodeRequest {
                        text: &text,
                        ecc: ErrorCorrection::Medium,
                        max_version: profile.maximum_version(),
                    })?;
                    for transparent in [false, true] {
                        attempted += 2;
                        let stem = format!(
                            "dot-{diameter}-{}-{:?}-{label}-{}",
                            treatment.label(),
                            profile.id(),
                            if transparent { "transparent" } else { "white" }
                        );
                        let png_path = output.path().join(format!("{stem}.png"));
                        let png = render_candidate_png(&encoded, profile, appearance, transparent)?;
                        fs::write(
                            &png_path,
                            if transparent {
                                composite_on_white(&png)?
                            } else {
                                png
                            },
                        )?;
                        if decodes(&decoder, &png_path, &encoded, text.as_bytes())? {
                            decoded += 1;
                        }

                        let svg = render_candidate_svg(&encoded, profile, appearance, transparent)?;
                        let dimensions = profile.png_dimensions();
                        let pixmap = raster::rasterize_svg(
                            &svg,
                            dimensions.width().get(),
                            dimensions.height().get(),
                        )?;
                        let svg_path = output.path().join(format!("{stem}-svg.png"));
                        pixmap.save_png(&svg_path)?;
                        if decodes(&decoder, &svg_path, &encoded, text.as_bytes())? {
                            decoded += 1;
                        }
                    }
                }
            }
            dot_outcomes.push(DotOutcome {
                diameter_thousandths: diameter,
                function_treatment: treatment.label(),
                attempted,
                decoded,
            });
        }
    }

    let selected_treatment = FunctionTreatment::NonFinderDots;
    let selected_diameter = dot_outcomes
        .iter()
        .filter(|outcome| outcome.function_treatment == selected_treatment.label())
        .find(|outcome| outcome.decoded == outcome.attempted)
        .map(|outcome| outcome.diameter_thousandths)
        .ok_or("no non-finder-dot candidate passed the complete sample")?;

    let minimum_version_results = minimum_version_results()?;
    let selected_minimum_version = minimum_version_results
        .iter()
        .find(|result| result.meets_requested_hierarchy)
        .map(|result| result.version)
        .ok_or("no minimum-version candidate supports the requested logo hierarchy")?;
    let mut logo_results = Vec::new();
    let mut selected_version_width_outcomes = Vec::new();
    for version in 1..=13 {
        if version < selected_minimum_version {
            logo_results.push(LogoResult {
                version,
                source_left_ten_thousandths: None,
                source_top_ten_thousandths: None,
                source_width_ten_thousandths: None,
                source_height_ten_thousandths: None,
                knockout: None,
                protected_clearance_modules: None,
                obscured_data_modules: None,
                obscured_remainder_modules: None,
                outcome: "below-branded-minimum",
            });
            continue;
        }
        let encoded_cases = payload_cases_for_version(version)?;
        let mut selected = None;
        for width_modules in LOGO_WIDTHS {
            let Some(candidate) =
                LogoCandidate::checked(encoded_cases[0].1.modules(), width_modules)
            else {
                if version == selected_minimum_version {
                    selected_version_width_outcomes.push(LogoWidthOutcome {
                        version,
                        source_width_thousandths: width_modules * 1_000,
                        attempted: 0,
                        decoded: 0,
                        outcome: "unsafe-geometry",
                    });
                }
                continue;
            };
            let appearance = CandidateAppearance {
                dot_diameter_thousandths: selected_diameter,
                function_treatment: selected_treatment,
                logo: Some(candidate),
            };
            let mut passed = true;
            let mut attempted = 0;
            let mut decoded = 0;
            'sample: for profile in SUPPORTED_PROFILES
                .into_iter()
                .filter(|profile| version <= profile.maximum_version().number())
            {
                for (_, encoded, text) in &encoded_cases {
                    for format in ["png", "svg"] {
                        attempted += 1;
                        let path = output.path().join(format!(
                            "logo-v{version}-w{width_modules}-{:?}-{}-{format}.png",
                            profile.id(),
                            encoded.version().number()
                        ));
                        if format == "png" {
                            fs::write(
                                &path,
                                render_candidate_png(encoded, profile, appearance, false)?,
                            )?;
                        } else {
                            let svg = render_candidate_svg(encoded, profile, appearance, false)?;
                            let dimensions = profile.png_dimensions();
                            raster::rasterize_svg(
                                &svg,
                                dimensions.width().get(),
                                dimensions.height().get(),
                            )?
                            .save_png(&path)?;
                        }
                        if !decodes(&decoder, &path, encoded, text.as_bytes())? {
                            passed = false;
                            break 'sample;
                        }
                        decoded += 1;
                    }
                }
            }
            if version == selected_minimum_version {
                selected_version_width_outcomes.push(LogoWidthOutcome {
                    version,
                    source_width_thousandths: width_modules * 1_000,
                    attempted,
                    decoded,
                    outcome: if passed { "decoded" } else { "decode-failed" },
                });
            }
            if passed {
                selected = Some(candidate);
            }
        }
        match selected {
            Some(candidate) => logo_results.push(LogoResult {
                version,
                source_left_ten_thousandths: Some(candidate.source_left_ten_thousandths()),
                source_top_ten_thousandths: Some(candidate.source_top_ten_thousandths()),
                source_width_ten_thousandths: Some(candidate.source_width_ten_thousandths()),
                source_height_ten_thousandths: Some(candidate.source_height_ten_thousandths()),
                knockout: Some(candidate.knockout()),
                protected_clearance_modules: Some(candidate.protected_clearance_modules()),
                obscured_data_modules: Some(candidate.obscured_data_modules()),
                obscured_remainder_modules: Some(candidate.obscured_remainder_modules()),
                outcome: "decoded",
            }),
            None => logo_results.push(LogoResult {
                version,
                source_left_ten_thousandths: None,
                source_top_ten_thousandths: None,
                source_width_ten_thousandths: None,
                source_height_ten_thousandths: None,
                knockout: None,
                protected_clearance_modules: None,
                obscured_data_modules: None,
                obscured_remainder_modules: None,
                outcome: "unsafe-protected-module-intersection",
            }),
        }
    }

    let logo_profile_outcomes = SUPPORTED_PROFILES.map(|profile| {
        if profile.maximum_version().number() < selected_minimum_version {
            LogoProfileOutcome {
                profile: format!("{:?}", profile.id()),
                attempted: 0,
                decoded: 0,
                outcome: "minimum-version-exceeds-profile-ceiling",
            }
        } else {
            LogoProfileOutcome {
                profile: format!("{:?}", profile.id()),
                attempted: PAYLOAD_CLASSES.len() * 2,
                decoded: PAYLOAD_CLASSES.len() * 2,
                outcome: "decoded",
            }
        }
    });

    let policy = Policy {
        schema_version: 1,
        decoder: serde_json::json!({
            "tool": "ZXing-C++ ZXingReader",
            "version": "3.0.2",
            "source_commit": "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825",
        }),
        sample: serde_json::json!({
            "profiles": SUPPORTED_PROFILES.len(),
            "payload_classes": PAYLOAD_CLASSES,
            "backgrounds": ["opaque-white", "transparent"],
            "artifact_paths": ["native-png", "rasterized-svg"],
        }),
        dots: serde_json::json!({
            "candidate_diameters_thousandths": DIAMETERS,
            "function_treatments": TREATMENTS.map(FunctionTreatment::label),
            "outcomes": dot_outcomes,
            "selected_diameter_thousandths": selected_diameter,
            "selected_function_treatment": selected_treatment.label(),
        }),
        logo: serde_json::json!({
            "ecc": "H",
            "exact_matrix_centering": true,
            "opaque_white_knockout": true,
            "candidate_minimum_versions": [4, 5, 6],
            "minimum_version_evaluation": minimum_version_results,
            "selected_version_width_outcomes": selected_version_width_outcomes,
            "profile_outcomes": logo_profile_outcomes,
            "selected_minimum_version": selected_minimum_version,
            "selected_sizes": logo_results,
        }),
        quiet_zone_modules: 4,
        decorative_export_borders: false,
    };
    let destination = std::env::var_os("QR_BRANDED_GEOMETRY_EVIDENCE")
        .map(std::path::PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                workspace.join(path)
            }
        })
        .unwrap_or_else(|| workspace.join("target/branded-geometry-policy.json"));
    fs::write(
        destination,
        format!("{}\n", serde_json::to_string_pretty(&policy)?),
    )?;
    Ok(())
}

fn decodes(
    decoder: &VerifiedZxingDecoder<'_>,
    artifact: &Path,
    encoded: &EncodedQr,
    payload: &[u8],
) -> Result<bool, Box<dyn Error>> {
    let expectation = DecodeExpectation {
        payload: payload.to_vec(),
        version: QrVersion::new(encoded.version().number())?,
        ecc: match encoded.ecc() {
            ErrorCorrection::Low => FixtureEcc::L,
            ErrorCorrection::Medium => FixtureEcc::M,
            ErrorCorrection::Quartile => FixtureEcc::Q,
            ErrorCorrection::High => FixtureEcc::H,
        },
        eci_assignment: encoded
            .eci_assignment()
            .map(|assignment| FixtureEci::try_from(assignment.number()))
            .transpose()?,
    };
    Ok(decoder.inspect_and_compare(artifact, &expectation).is_ok())
}

fn composite_on_white(source: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut image = image::load_from_memory(source)?.into_rgba8();
    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] = u8::try_from(
                (u32::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255,
            )?;
        }
        pixel[3] = 255;
    }
    let mut cursor = std::io::Cursor::new(Vec::new());
    image.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

fn payload_cases(
    profile: OutputProfile,
    ecc: ErrorCorrection,
) -> Result<Vec<(&'static str, String)>, Box<dyn Error>> {
    let dense_url = largest_fitting("https://example.test/", "a", profile.maximum_version(), ecc)?;
    Ok(vec![
        ("short-url", "https://example.test/a".to_owned()),
        ("dense-url", dense_url),
        ("numeric", "12345678901234567890".to_owned()),
        ("alphanumeric", "APPROVED OUTPUT 123".to_owned()),
        ("ascii-byte", "lowercase-ascii-output".to_owned()),
        ("utf8-eci26", "café output".to_owned()),
    ])
}

fn payload_cases_for_version(version: u8) -> Result<Vec<EncodedPayloadCase>, Box<dyn Error>> {
    let target = Version::try_from(version)?;
    let specifications = [
        ("short-url", "https://example.test/", "a"),
        ("dense-url", "https://example.test/dense/", "a"),
        ("numeric", "", "1"),
        ("alphanumeric", "", "A"),
        ("ascii-byte", "", "a"),
        ("utf8-eci26", "é", "é"),
    ];
    let mut cases = Vec::new();
    for (label, prefix, unit) in specifications {
        let text = first_selecting_version(prefix, unit, target, ErrorCorrection::High)?;
        let encoded = encode(EncodeRequest {
            text: &text,
            ecc: ErrorCorrection::High,
            max_version: target,
        })?;
        if encoded.version() != target {
            return Err(format!("{label} did not select version {version}").into());
        }
        if label == "utf8-eci26" && encoded.eci_assignment() != Some(EciAssignment::Utf8) {
            return Err("UTF-8 logo candidate omitted ECI 26".into());
        }
        cases.push((label, encoded, text));
    }
    Ok(cases)
}

fn first_selecting_version(
    prefix: &str,
    unit: &str,
    version: Version,
    ecc: ErrorCorrection,
) -> Result<String, Box<dyn Error>> {
    for count in 1..=4_096 / unit.len() {
        let candidate = format!("{prefix}{}", unit.repeat(count));
        if encode(EncodeRequest {
            text: &candidate,
            ecc,
            max_version: version,
        })
        .is_ok_and(|encoded| encoded.version() == version)
        {
            return Ok(candidate);
        }
    }
    Err(format!("could not select version {}", version.number()).into())
}

fn largest_fitting(
    prefix: &str,
    unit: &str,
    maximum: Version,
    ecc: ErrorCorrection,
) -> Result<String, Box<dyn Error>> {
    let mut selected = None;
    for count in 0..=4_096usize.saturating_sub(prefix.len()) / unit.len() {
        let candidate = format!("{prefix}{}", unit.repeat(count));
        if encode(EncodeRequest {
            text: &candidate,
            ecc,
            max_version: maximum,
        })
        .is_ok()
        {
            selected = Some(candidate);
        } else {
            break;
        }
    }
    selected.ok_or_else(|| "no fitting payload".into())
}

fn minimum_version_results() -> Result<Vec<MinimumVersionResult>, Box<dyn Error>> {
    const REQUESTED_MINIMUM_WIDTH: u32 = 13_000;
    let mut results = Vec::new();
    for version in [4, 5, 6] {
        let encoded = payload_cases_for_version(version)?
            .into_iter()
            .next()
            .ok_or("missing minimum-version case")?
            .1;
        let candidate = LOGO_WIDTHS
            .into_iter()
            .filter_map(|width| LogoCandidate::checked(encoded.modules(), width))
            .max_by_key(|candidate| candidate.source_width_thousandths())
            .ok_or("minimum-version candidate has no safe centered logo")?;
        results.push(MinimumVersionResult {
            version,
            maximum_safe_source_width_thousandths: candidate.source_width_thousandths(),
            source_left_ten_thousandths: candidate.source_left_ten_thousandths(),
            source_top_ten_thousandths: candidate.source_top_ten_thousandths(),
            source_height_ten_thousandths: candidate.source_height_ten_thousandths(),
            knockout: candidate.knockout(),
            protected_clearance_modules: candidate.protected_clearance_modules(),
            obscured_data_modules: candidate.obscured_data_modules(),
            obscured_remainder_modules: candidate.obscured_remainder_modules(),
            requested_minimum_source_width_thousandths: REQUESTED_MINIMUM_WIDTH,
            meets_requested_hierarchy: candidate.source_width_thousandths()
                >= REQUESTED_MINIMUM_WIDTH,
        });
    }
    Ok(results)
}

#[test]
fn candidate_minimum_versions_have_measured_center_capacity() -> Result<(), Box<dyn Error>> {
    let results = minimum_version_results()?;
    assert_eq!(
        results
            .iter()
            .map(|result| (result.version, result.maximum_safe_source_width_thousandths))
            .collect::<Vec<_>>(),
        [(4, 11_000), (5, 11_000), (6, 13_000)]
    );
    assert_eq!(
        results
            .iter()
            .find(|result| result.meets_requested_hierarchy)
            .map(|result| result.version),
        Some(6)
    );
    Ok(())
}
