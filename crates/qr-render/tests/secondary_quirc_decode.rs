#[path = "support/versions.rs"]
mod versions;

use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{LogoStyle, ProfileId, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};

const QUIRC_COMMIT: &str = "542848dd6b9b0eaa9587bbf25b9bc67bd8a71fca";
const QUIRC_READER_VERSION: &str = "quirc-reader quirc 1.2";

#[test]
#[ignore = "requires the manifest-pinned quirc checkout and reader"]
fn representative_ascii_rasters_decode_to_exact_payload_bytes() -> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = workspace.join("tests/oracles/quirc");
    let reader = workspace.join("tests/oracles/quirc-reader");
    verify_quirc(&source, &reader)?;
    let output = tempfile::tempdir()?;

    let profile = |id| {
        SUPPORTED_PROFILES
            .into_iter()
            .find(|profile| profile.id() == id)
            .ok_or("representative quirc profile is missing")
    };
    let standard = profile(ProfileId::Standard)?;
    let hero_campaign = profile(ProfileId::HeroCampaign)?;
    let business_card = profile(ProfileId::BusinessCard)?;
    let dense = "a".repeat(versions::first_byte_length(12));
    let adaptive_branded = format!("https://example.test/{}", "a".repeat(88));
    let cases = [
        (
            "ordinary-unbranded",
            "https://example.test/a".to_owned(),
            standard,
            false,
        ),
        ("dense-unbranded", dense, hero_campaign, false),
        (
            "branded-v6",
            "https://example.test/logo".to_owned(),
            standard,
            true,
        ),
        ("branded-adaptive", adaptive_branded, business_card, true),
    ];

    for (label, payload, profile, branded) in cases {
        let ecc = if branded {
            ErrorCorrection::High
        } else {
            ErrorCorrection::Medium
        };
        let request = if branded {
            EncodeRequest::with_version_range(
                &payload,
                ecc,
                Version::new(6)?,
                profile.maximum_version(),
            )
        } else {
            EncodeRequest::first_fit(&payload, ecc, profile.maximum_version())
        };
        let encoded = encode(request)?;
        let options = RenderOptions::safe(profile)?.with_logo(if branded {
            LogoStyle::Bundled
        } else {
            LogoStyle::None
        })?;
        let png = render_png(&RenderModel::new(&encoded, options)?)?;
        let pgm = output.path().join(format!("{label}.pgm"));
        write_grayscale_pgm(&png, &pgm)?;
        let decoded = Command::new(&reader).arg(&pgm).output()?;
        if !decoded.status.success() {
            return Err(format!(
                "quirc failed for {label}: {}",
                String::from_utf8_lossy(&decoded.stderr)
            )
            .into());
        }
        assert_eq!(
            String::from_utf8(decoded.stdout)?.trim(),
            hex(payload.as_bytes())
        );
    }
    Ok(())
}

fn verify_quirc(source: &Path, reader: &Path) -> Result<(), Box<dyn Error>> {
    let commit = Command::new("git")
        .args([
            "-C",
            source.to_str().ok_or("invalid quirc path")?,
            "rev-parse",
            "HEAD",
        ])
        .output()?;
    if !commit.status.success() || String::from_utf8(commit.stdout)?.trim() != QUIRC_COMMIT {
        return Err("quirc source checkout is not at the manifest-pinned commit".into());
    }
    let status = Command::new("git")
        .args([
            "-C",
            source.to_str().ok_or("invalid quirc path")?,
            "status",
            "--porcelain",
            "--untracked-files=no",
        ])
        .output()?;
    if !status.status.success() || !status.stdout.is_empty() {
        return Err("quirc source checkout has tracked modifications".into());
    }
    let version = Command::new(reader).arg("--version").output()?;
    if !version.status.success()
        || String::from_utf8(version.stdout)?.trim() != QUIRC_READER_VERSION
    {
        return Err("quirc reader does not report the manifest-pinned version".into());
    }
    Ok(())
}

fn write_grayscale_pgm(png: &[u8], path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let mut reader = png::Decoder::new(Cursor::new(png)).read_info()?;
    let mut rgba = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or("PNG output is too large")?
    ];
    let output = reader.next_frame(&mut rgba)?;
    rgba.truncate(output.buffer_size());
    let mut pgm = format!("P5\n{} {}\n255\n", output.width, output.height).into_bytes();
    pgm.reserve(rgba.len() / 4);
    for pixel in rgba.as_chunks::<4>().0 {
        if pixel[3] != u8::MAX {
            return Err("quirc campaign requires opaque input".into());
        }
        let luminance = (299 * u32::from(pixel[0])
            + 587 * u32::from(pixel[1])
            + 114 * u32::from(pixel[2])
            + 500)
            / 1_000;
        pgm.push(u8::try_from(luminance)?);
    }
    fs::write(path, pgm)?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
