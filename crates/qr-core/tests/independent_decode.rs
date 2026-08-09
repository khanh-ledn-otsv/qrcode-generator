use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::process::Command;

const PINNED_COMMIT: &str = "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825";

#[test]
#[ignore = "requires the manifest-pinned ZXing-C++ checkout and reader"]
fn seeded_safe_artifacts_decode_exact_bytes_and_eci_metadata() -> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = workspace.join("tests/oracles/zxing-cpp");
    let reader = source.join("build/example/ZXingReader");
    verify_decoder(&source, &reader)?;

    let output = tempfile::tempdir()?;
    run_cases(&reader, output.path())
}

fn run_cases(reader: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let mut cases = vec![
        ("8675309".to_owned(), ErrorCorrection::Low, Version::MINIMUM),
        (
            "HELLO WORLD".to_owned(),
            ErrorCorrection::Medium,
            Version::MINIMUM,
        ),
        (
            "byte_mode@example.test".to_owned(),
            ErrorCorrection::Quartile,
            Version::MINIMUM,
        ),
        (
            "Xin chào QR".to_owned(),
            ErrorCorrection::High,
            Version::MINIMUM,
        ),
        (
            "  branded payload\r\nkeeps bytes  ".to_owned(),
            ErrorCorrection::High,
            Version::new(6)?,
        ),
    ];
    let mut state = 0x8a5c_2d71_u32;
    for index in 0..128 {
        let mut text = format!("safe/{index:02}/");
        for _ in 0..(8 + index % 31) {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            text.push(char::from(b'a' + u8::try_from(state % 26)?));
        }
        if index % 9 == 0 {
            text.push_str("/café");
        }
        let ecc = match index % 4 {
            0 => ErrorCorrection::Low,
            1 => ErrorCorrection::Medium,
            2 => ErrorCorrection::Quartile,
            _ => ErrorCorrection::High,
        };
        cases.push((text, ecc, Version::MINIMUM));
    }
    for version in [2, 7, 10, 27, 40] {
        cases.push((
            payload_for_version(version)?,
            ErrorCorrection::Low,
            Version::MINIMUM,
        ));
    }

    let mut masks = BTreeSet::new();
    let mut versions = BTreeSet::new();
    for (index, (text, ecc, minimum)) in cases.iter().enumerate() {
        let encoded = encode(EncodeRequest::with_version_range(
            text,
            *ecc,
            *minimum,
            Version::new(40)?,
        ))?;
        masks.insert(encoded.mask().number());
        versions.insert(encoded.version().number());
        let artifact = output.join(format!("case-{index:02}.pgm"));
        std::fs::write(&artifact, pgm(&encoded))?;

        let bytes = Command::new(reader)
            .args(["-formats", "QRCode", "-single", "-bytes"])
            .arg(&artifact)
            .output()?;
        if !bytes.status.success() || bytes.stdout != text.as_bytes() {
            return Err(format!("ZXing byte decode failed for synthetic case {index}").into());
        }
        let metadata = Command::new(reader)
            .args(["-formats", "QRCode", "-single", "-mode", "eci"])
            .arg(&artifact)
            .output()?;
        let metadata = String::from_utf8(metadata.stdout)?;
        let expected_version = encoded.version().number().to_string();
        let expected_eci = if encoded.eci_assignment().is_some() {
            "true"
        } else {
            "false"
        };
        let expected_ecc = match encoded.ecc() {
            ErrorCorrection::Low => "L",
            ErrorCorrection::Medium => "M",
            ErrorCorrection::Quartile => "Q",
            ErrorCorrection::High => "H",
        };
        for (key, expected) in [
            ("Version", expected_version.as_str()),
            ("HasECI", expected_eci),
            ("ECLevel", expected_ecc),
        ] {
            let actual = metadata_value(&metadata, key);
            if actual != Some(expected) {
                let exposed_keys = metadata
                    .lines()
                    .filter_map(|line| line.split_once(':').map(|(name, _)| name.trim()))
                    .collect::<Vec<_>>();
                return Err(format!(
                    "ZXing {key} metadata mismatch for synthetic case {index}: expected {expected}, got {actual:?}; exposed keys {exposed_keys:?}"
                )
                .into());
            }
        }
    }
    if masks != BTreeSet::from([0, 1, 2, 3, 4, 5, 6, 7]) {
        return Err(format!("decode cases covered masks {masks:?}, expected every mask").into());
    }
    if ![1, 2, 7, 10, 27, 40]
        .into_iter()
        .all(|version| versions.contains(&version))
    {
        return Err(format!("decode cases missed structural versions: got {versions:?}").into());
    }
    Ok(())
}

fn payload_for_version(target: u8) -> Result<String, Box<dyn Error>> {
    let maximum = Version::new(40)?;
    let mut low = 1_usize;
    let mut high = 2900_usize;
    while low < high {
        let length = low + (high - low) / 2;
        let text = "a".repeat(length);
        let encoded = encode(EncodeRequest::first_fit(
            &text,
            ErrorCorrection::Low,
            maximum,
        ))?;
        if encoded.version().number() < target {
            low = length + 1;
        } else {
            high = length;
        }
    }
    let text = "a".repeat(low);
    let selected = encode(EncodeRequest::first_fit(
        &text,
        ErrorCorrection::Low,
        maximum,
    ))?;
    if selected.version().number() != target {
        return Err(format!("no synthetic payload selected Version {target}").into());
    }
    Ok(text)
}

fn metadata_value<'a>(metadata: &'a str, key: &str) -> Option<&'a str> {
    metadata.lines().find_map(|line| {
        line.split_once(':')
            .filter(|(candidate, _)| candidate.trim() == key)
            .map(|(_, value)| value.trim())
    })
}

fn verify_decoder(source: &Path, reader: &Path) -> Result<(), Box<dyn Error>> {
    let source = source.canonicalize()?;
    let reader = reader.canonicalize()?;
    if !reader.starts_with(&source) {
        return Err("ZXingReader is not built inside the pinned checkout".into());
    }
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&source)
        .output()?;
    if String::from_utf8(commit.stdout)?.trim() != PINNED_COMMIT {
        return Err("ZXing source checkout is not at the manifest-pinned commit".into());
    }
    for arguments in [
        ["diff", "--quiet"].as_slice(),
        ["diff", "--cached", "--quiet"].as_slice(),
    ] {
        if !Command::new("git")
            .args(arguments)
            .current_dir(&source)
            .status()?
            .success()
        {
            return Err("ZXing source checkout has tracked modifications".into());
        }
    }
    let submodules = Command::new("git")
        .args(["submodule", "status", "--recursive"])
        .current_dir(&source)
        .output()?;
    if !submodules.status.success()
        || submodules
            .stdout
            .split(|byte| *byte == b'\n')
            .any(|line| matches!(line.first(), Some(b'-' | b'+' | b'U')))
    {
        return Err("ZXing source checkout has uninitialized or modified submodules".into());
    }
    let version = Command::new(&reader).arg("-version").output()?;
    if String::from_utf8(version.stdout)?.trim() != "ZXingReader version 3.0.2" {
        return Err("ZXingReader is not the manifest-pinned version".into());
    }
    Ok(())
}

fn pgm(encoded: &EncodedQr) -> Vec<u8> {
    const QUIET: usize = 4;
    const SCALE: usize = 8;
    let modules = usize::from(encoded.modules().size());
    let pixels = (modules + QUIET * 2) * SCALE;
    let mut output = format!("P5\n{pixels} {pixels}\n255\n").into_bytes();
    for y in 0..pixels {
        for x in 0..pixels {
            let module_x = x / SCALE;
            let module_y = y / SCALE;
            let dark = module_x >= QUIET
                && module_y >= QUIET
                && module_x < modules + QUIET
                && module_y < modules + QUIET
                && encoded
                    .modules()
                    .module(
                        u16::try_from(module_x - QUIET).unwrap_or_default(),
                        u16::try_from(module_y - QUIET).unwrap_or_default(),
                    )
                    .is_some_and(|module| module.is_dark());
            output.push(if dark { 0 } else { 255 });
        }
    }
    output
}
