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
    APPROVED_DATA_MODULE_STYLES, Background, Foreground, LogoStyle, OutputProfile, RenderModel,
    RenderOptions, Rgba, SUPPORTED_PROFILES, render_png, render_svg,
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

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for version in 1..=profile.maximum_version().number() {
            let text = high_versions::payload_for_high_version(version)?;
            let encoded = encode(EncodeRequest {
                text: &text,
                ecc: ErrorCorrection::High,
                max_version: Version::try_from(version)?,
            })?;
            let expected = DecodeExpectation {
                payload: text.into_bytes(),
                version: QrVersion::new(version)?,
                ecc: FixtureEcc::H,
                eci_assignment: None,
            };

            for (style_index, style) in APPROVED_DATA_MODULE_STYLES.into_iter().enumerate() {
                let options = RenderOptions::approved_with_data_style(
                    profile,
                    Foreground::Brand,
                    Background::Opaque(Rgba::WHITE),
                    style,
                )?
                .with_logo(LogoStyle::Bundled)?;
                let model = RenderModel::new(&encoded, options)?;
                let svg = render_svg(&model)?;
                let dimensions = profile.svg_dimensions();
                let pixmap = raster::rasterize_svg(
                    &svg,
                    dimensions.width().get(),
                    dimensions.height().get(),
                )?;
                let svg_artifact = output.path().join(format!(
                    "logo-svg-{profile_index}-{version}-{style_index}.png"
                ));
                pixmap.save_png(&svg_artifact)?;
                if let Err(error) = decoder.inspect_and_compare(&svg_artifact, &expected) {
                    failures.push(format!(
                        "SVG profile {profile_index} version {version} style {style_index}: {error}"
                    ));
                }

                let png_artifact = output.path().join(format!(
                    "logo-png-{profile_index}-{version}-{style_index}.png"
                ));
                fs::write(&png_artifact, render_png(&model)?)?;
                if let Err(error) = decoder.inspect_and_compare(&png_artifact, &expected) {
                    failures.push(format!(
                        "PNG profile {profile_index} version {version} style {style_index}: {error}"
                    ));
                }
            }
        }
    }

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for (style_index, style) in APPROVED_DATA_MODULE_STYLES.into_iter().enumerate() {
            for (payload_label, text) in required_logo_payloads(profile)? {
                let encoded = encode(EncodeRequest {
                    text: &text,
                    ecc: ErrorCorrection::High,
                    max_version: profile.maximum_version(),
                })?;
                let expected = DecodeExpectation {
                    payload: text.into_bytes(),
                    version: QrVersion::new(encoded.version().number())?,
                    ecc: FixtureEcc::H,
                    eci_assignment: encoded
                        .eci_assignment()
                        .map(|assignment| EciAssignment::try_from(assignment.number()))
                        .transpose()?,
                };
                let options = RenderOptions::approved_with_data_style(
                    profile,
                    Foreground::Brand,
                    Background::Opaque(Rgba::WHITE),
                    style,
                )?
                .with_logo(LogoStyle::Bundled)?;
                let model = RenderModel::new(&encoded, options)?;
                let dimensions = profile.svg_dimensions();
                let svg = render_svg(&model)?;
                let pixmap = raster::rasterize_svg(
                    &svg,
                    dimensions.width().get(),
                    dimensions.height().get(),
                )?;
                let svg_artifact = output.path().join(format!(
                    "logo-svg-payload-{profile_index}-{style_index}-{payload_label}.png"
                ));
                pixmap.save_png(&svg_artifact)?;
                if let Err(error) = decoder.inspect_and_compare(&svg_artifact, &expected) {
                    failures.push(format!(
                        "SVG profile {profile_index} style {style_index} payload {payload_label}: {error}"
                    ));
                }

                let png_artifact = output.path().join(format!(
                    "logo-png-payload-{profile_index}-{style_index}-{payload_label}.png"
                ));
                fs::write(&png_artifact, render_png(&model)?)?;
                if let Err(error) = decoder.inspect_and_compare(&png_artifact, &expected) {
                    failures.push(format!(
                        "PNG profile {profile_index} style {style_index} payload {payload_label}: {error}"
                    ));
                }
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
    profile: OutputProfile,
) -> Result<Vec<(&'static str, String)>, Box<dyn Error>> {
    let dense_prefix = "https://example.test/";
    let mut dense_url = None;
    for suffix_length in 0..=1_000 {
        let text = format!("{dense_prefix}{}", "a".repeat(suffix_length));
        if encode(EncodeRequest {
            text: &text,
            ecc: ErrorCorrection::High,
            max_version: profile.maximum_version(),
        })
        .is_ok()
        {
            dense_url = Some(text);
        } else {
            break;
        }
    }
    let dense_url = dense_url.ok_or("logo profile cannot fit the dense URL prefix")?;
    let dense = encode(EncodeRequest {
        text: &dense_url,
        ecc: ErrorCorrection::High,
        max_version: profile.maximum_version(),
    })?;
    if dense.version() != profile.maximum_version() {
        return Err("dense logo URL did not select the profile ceiling version".into());
    }

    Ok(vec![
        ("short-url", "https://example.test/a".to_owned()),
        ("dense-url", dense_url),
        ("numeric", "12345678901234567890".to_owned()),
        ("alphanumeric", "APPROVED LOGO 123".to_owned()),
        ("ascii-byte", "lowercase-logo-byte".to_owned()),
        ("utf8-eci26", "café logo".to_owned()),
    ])
}
