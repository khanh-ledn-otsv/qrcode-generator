#[path = "support/raster.rs"]
mod raster;

use std::error::Error;
use std::fs;
use std::path::Path;

use fixture_tool::{
    DecodeExpectation, ErrorCorrection as FixtureEcc, FixtureManifest, QrVersion, ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    LogoStyle, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg,
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
            let text = payload_for_high_version(version)?;
            let encoded = encode(EncodeRequest {
                text: &text,
                ecc: ErrorCorrection::High,
                max_version: Version::try_from(version)?,
            })?;
            let options = RenderOptions::safe(profile)?.with_logo(LogoStyle::Bundled)?;
            let model = RenderModel::new(&encoded, options)?;
            let expected = DecodeExpectation {
                payload: text.into_bytes(),
                version: QrVersion::new(version)?,
                ecc: FixtureEcc::H,
                eci_assignment: None,
            };

            let svg = render_svg(&model)?;
            let dimensions = profile.svg_dimensions();
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

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}

fn payload_for_high_version(version: u8) -> Result<String, Box<dyn Error>> {
    for length in 1..=1_000 {
        let text = "a".repeat(length);
        if encode(EncodeRequest {
            text: &text,
            ecc: ErrorCorrection::High,
            max_version: Version::try_from(version)?,
        })
        .is_ok_and(|encoded| encoded.version().number() == version)
        {
            return Ok(text);
        }
    }
    Err(format!("no byte payload selected H-level version {version}").into())
}
