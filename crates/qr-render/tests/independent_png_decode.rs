#[path = "support/versions.rs"]
mod versions;

use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use fixture_tool::{
    DecodeExpectation, ErrorCorrection as FixtureEcc, FixtureManifest, QrVersion, ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_DATA_MODULE_STYLES, APPROVED_FOREGROUNDS, Background,
    RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png,
};

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

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        let text = if profile_index == 3 {
            "a".repeat(70)
        } else {
            "APPROVED".to_owned()
        };
        for (foreground_index, foreground) in APPROVED_FOREGROUNDS.into_iter().enumerate() {
            for (background_index, background) in APPROVED_BACKGROUNDS.into_iter().enumerate() {
                for (style_index, style) in APPROVED_DATA_MODULE_STYLES.into_iter().enumerate() {
                    let encoded = encode(EncodeRequest {
                        text: &text,
                        ecc: ErrorCorrection::Medium,
                        max_version: profile.maximum_version(),
                    })?;
                    let model = RenderModel::new(
                        &encoded,
                        RenderOptions::approved_with_data_style(
                            profile, foreground, background, style,
                        )?,
                    )?;
                    let png = render_png(&model)?;
                    let effective_png = match background {
                        Background::Transparent => composite_on_white(&png)?,
                        Background::Opaque(_) => png,
                    };
                    let artifact = output.path().join(format!(
                        "png-approved-{profile_index}-{foreground_index}-{background_index}-{style_index}.png"
                    ));
                    fs::write(&artifact, effective_png)?;
                    decoder.inspect_and_compare(
                        &artifact,
                        &DecodeExpectation {
                            payload: text.as_bytes().to_vec(),
                            version: QrVersion::new(encoded.version().number())?,
                            ecc: FixtureEcc::M,
                            eci_assignment: None,
                        },
                    )
                    .map_err(|error| {
                        format!(
                            "approved PNG tuple {profile_index}/{foreground_index}/{background_index}/{style_index}: {error}"
                        )
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn composite_on_white(source: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let decoder = png::Decoder::new(Cursor::new(source));
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or("PNG output is too large")?
    ];
    let output = reader.next_frame(&mut pixels)?;
    pixels.truncate(output.buffer_size());
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel[..3] {
            let composited = u16::from(*channel) * alpha + 255 * (255 - alpha);
            *channel = u8::try_from((composited + 127) / 255)?;
        }
        pixel[3] = 255;
    }

    let mut composited = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut composited, output.width, output.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&pixels)?;
    }
    Ok(composited)
}

fn payload_for_version(version: u8, case_index: usize) -> String {
    let length = versions::first_byte_length(version);
    let prefix = char::from(b'0' + u8::try_from(case_index % 10).unwrap_or_default());
    format!("{prefix}{}", "a".repeat(length - 1))
}
