use fixture_tool::{FixtureManifest, ZxingDecoder};
use std::error::Error;
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn Error>> {
    let Some(command) = arguments.first().and_then(|value| value.to_str()) else {
        return Err(usage().into());
    };
    match command {
        "verify" if arguments.len() == 2 => verify(Path::new(&arguments[1])),
        "decode" if arguments.len() == 6 => decode(
            Path::new(&arguments[1]),
            arguments[2]
                .to_str()
                .ok_or_else(|| invalid_input("fixture id must be valid UTF-8"))?,
            Path::new(&arguments[3]),
            Path::new(&arguments[4]),
            Path::new(&arguments[5]),
        ),
        "diff" if arguments.len() == 2 => diff(
            arguments[1]
                .to_str()
                .ok_or_else(|| invalid_input("git reference must be valid UTF-8"))?,
        ),
        _ => Err(usage().into()),
    }
}

fn usage() -> &'static str {
    "usage: fixture-tool verify MANIFEST | decode MANIFEST FIXTURE_ID ARTIFACT ZXING_SOURCE ZXING_READER | diff GIT_REF"
}

fn verify(manifest_path: &Path) -> Result<(), Box<dyn Error>> {
    let manifest = FixtureManifest::load_and_verify(manifest_path)?;
    let fixture_count = manifest.fixtures().len() + manifest.algorithm_fixtures().len();
    println!("verified {fixture_count} fixture(s)");
    Ok(())
}

fn decode(
    manifest_path: &Path,
    fixture_id: &str,
    artifact: &Path,
    source_checkout: &Path,
    executable: &Path,
) -> Result<(), Box<dyn Error>> {
    let manifest = FixtureManifest::load_and_verify(manifest_path)?;
    let fixture_root = manifest_path
        .parent()
        .ok_or_else(|| invalid_input("manifest path must have a parent directory"))?;
    let expected = manifest
        .fixture(fixture_id)?
        .decode_expectation(fixture_root)?;
    let observation = ZxingDecoder::new(
        executable,
        manifest.decoder().version(),
        source_checkout,
        manifest.decoder().source_commit(),
    )
    .inspect_and_compare(artifact, &expected)?;
    println!(
        "decoded exact payload bytes; version {}, ECC {}, ECI {}",
        observation.version, observation.ecc, observation.has_eci
    );
    Ok(())
}

fn diff(reference: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("git")
        .args([
            "diff",
            "--no-ext-diff",
            "--unified=3",
            reference,
            "--",
            "tests/fixtures/manifest.json",
            "tests/fixtures/matrices",
            "tests/fixtures/payloads",
        ])
        .status()?;
    if !status.success() {
        return Err(format!("git diff failed with {status}").into());
    }
    Ok(())
}

fn invalid_input(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}
