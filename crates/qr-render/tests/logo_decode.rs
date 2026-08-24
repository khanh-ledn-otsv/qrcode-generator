#[path = "support/high_versions.rs"]
mod high_versions;
#[path = "support/raster.rs"]
mod raster;

use std::error::Error;
use std::fs;
use std::path::Path;

use fixture_tool::{
    DecodeExpectation, EciAssignment, ErrorCorrection as FixtureEcc, FixtureManifest, QrVersion,
    ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    BRANDED_LOGO_VERSION, LogoStyle, OutputProfile, RenderError, RenderModel, RenderOptions,
    SUPPORTED_PROFILES, render_png, render_svg,
};

#[test]
#[ignore = "requires the manifest-pinned ZXing-C++ checkout and reader"]
fn bundled_logo_decodes_for_every_enabled_profile_version() -> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest =
        FixtureManifest::load_and_verify(workspace.join("tests/fixtures/manifest.json"))?;
    let source = workspace.join("tests/oracles/zxing-cpp");
    let decoder = ZxingDecoder::new(
        source.join("build/example/ZXingReader"),
        manifest.decoder().version(),
        &source,
        manifest.decoder().source_commit(),
    );
    let output = tempfile::tempdir()?;
    let mut failures = Vec::new();
    let mut enabled_rows = 0_usize;

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for version in profile.minimum_version().number()..=profile.maximum_version().number() {
            let text = high_versions::payload_for_high_version(version)?;
            let encoded = encode(EncodeRequest::first_fit(
                &text,
                ErrorCorrection::High,
                Version::try_from(version)?,
            ))?;
            let expected = DecodeExpectation {
                payload: text.into_bytes(),
                version: QrVersion::new(version)?,
                ecc: FixtureEcc::H,
                eci_assignment: None,
            };

            let options = RenderOptions::safe(profile)?.with_logo(LogoStyle::Bundled)?;
            let model = match RenderModel::new(&encoded, options) {
                Ok(model) => model,
                Err(RenderError::UnsafeLogoGeometry) => continue,
                Err(error) => return Err(error.into()),
            };
            enabled_rows += 1;
            let svg = render_svg(&model)?;
            let dimensions = profile.png_dimensions_for(encoded.version())?;
            let pixmap =
                raster::rasterize_svg(&svg, dimensions.width().get(), dimensions.height().get())?;
            let svg_artifact = output
                .path()
                .join(format!("logo-svg-{profile_index}-{version}.png"));
            pixmap.save_png(&svg_artifact)?;
            if let Err(error) = decoder.inspect_and_compare(&svg_artifact, &expected) {
                failures.push(format!(
                    "SVG profile {profile_index} version {version}: {error}"
                ));
            }

            let png_artifact = output
                .path()
                .join(format!("logo-png-{profile_index}-{version}.png"));
            fs::write(&png_artifact, render_png(&model)?)?;
            if let Err(error) = decoder.inspect_and_compare(&png_artifact, &expected) {
                failures.push(format!(
                    "PNG profile {profile_index} version {version}: {error}"
                ));
            }
        }
    }
    let expected_enabled_rows = SUPPORTED_PROFILES
        .iter()
        .filter(|profile| {
            profile.minimum_version() <= BRANDED_LOGO_VERSION
                && profile.maximum_version() >= BRANDED_LOGO_VERSION
        })
        .count();
    if enabled_rows != expected_enabled_rows {
        return Err(format!(
            "expected {expected_enabled_rows} enabled fixed rows, found {enabled_rows}"
        )
        .into());
    }

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        if profile.maximum_version().number() < 6 {
            continue;
        }
        for (payload_label, text) in required_logo_payloads(profile)? {
            let logo_maximum = BRANDED_LOGO_VERSION;
            let encoded = encode(EncodeRequest::with_version_range(
                &text,
                ErrorCorrection::High,
                Version::try_from(6)?,
                logo_maximum,
            ))?;
            let expected = DecodeExpectation {
                payload: text.into_bytes(),
                version: QrVersion::new(encoded.version().number())?,
                ecc: FixtureEcc::H,
                eci_assignment: encoded
                    .eci_assignment()
                    .map(|assignment| EciAssignment::try_from(assignment.number()))
                    .transpose()?,
            };
            let options = RenderOptions::safe(profile)?.with_logo(LogoStyle::Bundled)?;
            let model = match RenderModel::new(&encoded, options) {
                Ok(model) => model,
                Err(RenderError::UnsafeLogoGeometry) => continue,
                Err(error) => return Err(error.into()),
            };
            let dimensions = profile.png_dimensions_for(encoded.version())?;
            let svg = render_svg(&model)?;
            let pixmap =
                raster::rasterize_svg(&svg, dimensions.width().get(), dimensions.height().get())?;
            let svg_artifact = output.path().join(format!(
                "logo-svg-payload-{profile_index}-{payload_label}.png"
            ));
            pixmap.save_png(&svg_artifact)?;
            if let Err(error) = decoder.inspect_and_compare(&svg_artifact, &expected) {
                failures.push(format!(
                    "SVG profile {profile_index} payload {payload_label}: {error}"
                ));
            }

            let png_artifact = output.path().join(format!(
                "logo-png-payload-{profile_index}-{payload_label}.png"
            ));
            fs::write(&png_artifact, render_png(&model)?)?;
            if let Err(error) = decoder.inspect_and_compare(&png_artifact, &expected) {
                failures.push(format!(
                    "PNG profile {profile_index} payload {payload_label}: {error}"
                ));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}

fn required_logo_payloads(
    _profile: OutputProfile,
) -> Result<Vec<(&'static str, String)>, Box<dyn Error>> {
    let logo_maximum = BRANDED_LOGO_VERSION;
    let dense_prefix = "https://example.test/";
    let mut dense_url = None;
    for suffix_length in 0..=1_000 {
        let text = format!("{dense_prefix}{}", "a".repeat(suffix_length));
        if encode(EncodeRequest::first_fit(
            &text,
            ErrorCorrection::High,
            logo_maximum,
        ))
        .is_ok()
        {
            dense_url = Some(text);
        } else {
            break;
        }
    }
    let dense_url = dense_url.ok_or("logo profile cannot fit the dense URL prefix")?;
    let dense = encode(EncodeRequest::first_fit(
        &dense_url,
        ErrorCorrection::High,
        logo_maximum,
    ))?;
    if dense.version() != logo_maximum {
        return Err("dense logo URL did not select the approved logo ceiling version".into());
    }

    let payloads = vec![
        ("short-url", "https://example.test/a".to_owned()),
        ("dense-url", dense_url),
        ("numeric", "12345678901234567890".to_owned()),
        ("alphanumeric", "APPROVED LOGO 123".to_owned()),
        ("ascii-byte", "lowercase-logo-byte".to_owned()),
        ("utf8-eci26", "café logo".to_owned()),
    ];
    Ok(payloads)
}
