#[path = "support/adverse.rs"]
mod adverse;

use std::error::Error;
use std::fs;
use std::path::Path;

use fixture_tool::{
    DecodeExpectation, ErrorCorrection as FixtureEcc, FixtureManifest, QrVersion, ZxingDecoder,
};
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    Background, Foreground, LogoStyle, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png,
};

#[test]
fn adverse_manifest_records_every_required_deterministic_transform() -> Result<(), Box<dyn Error>> {
    let suite = adverse::TransformSuite::load()?;

    assert_eq!(suite.seed(), 20_260_807);
    assert_eq!(
        suite.kinds(),
        [
            "blur",
            "scaling",
            "jpeg",
            "rotation",
            "perspective",
            "contrast",
            "brightness",
            "background",
            "background",
            "background",
            "dot_gain",
            "ink_loss",
            "grayscale",
        ]
    );
    Ok(())
}

#[test]
fn adverse_transforms_are_reproducible_and_preserve_canvas_dimensions() -> Result<(), Box<dyn Error>>
{
    let encoded = encode(EncodeRequest {
        text: "https://example.test/adverse",
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: SUPPORTED_PROFILES[1].maximum_version(),
    })?;
    let options = RenderOptions::approved(
        SUPPORTED_PROFILES[1],
        Foreground::Brand,
        Background::Transparent,
    )?;
    let source = render_png(&RenderModel::new(&encoded, options)?)?;
    let suite = adverse::TransformSuite::load()?;

    for transform in suite.transforms() {
        let first = transform.apply(&source, suite.seed())?;
        let second = transform.apply(&source, suite.seed())?;
        assert_eq!(first, second, "{} is not deterministic", transform.id());
        assert_eq!(
            adverse::png_dimensions(&first)?,
            adverse::png_dimensions(&source)?,
            "{} changed the exported canvas",
            transform.id()
        );
        assert!(
            adverse::pixels_differ(&first, &source)?,
            "{} had no observable pixel effect",
            transform.id()
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires the manifest-pinned ZXing-C++ checkout and reader"]
fn adverse_transform_envelope_independently_decodes_and_records_evidence()
-> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest =
        FixtureManifest::load_and_verify(workspace.join("tests/fixtures/manifest.json"))?;
    let source_checkout = workspace.join("tests/oracles/zxing-cpp");
    let decoder = ZxingDecoder::new(
        source_checkout.join("build/example/ZXingReader"),
        manifest.decoder().version(),
        &source_checkout,
        manifest.decoder().source_commit(),
    );
    let safe_payload = "https://example.test/adverse-envelope";
    let safe_encoded = encode(EncodeRequest {
        text: safe_payload,
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: SUPPORTED_PROFILES[1].maximum_version(),
    })?;
    let safe_source = render_png(&RenderModel::new(
        &safe_encoded,
        RenderOptions::approved(
            SUPPORTED_PROFILES[1],
            Foreground::Brand,
            Background::Opaque(qr_render::Rgba::WHITE),
        )?,
    )?)?;
    let safe_expected = DecodeExpectation {
        payload: safe_payload.as_bytes().to_vec(),
        version: QrVersion::new(safe_encoded.version().number())?,
        ecc: FixtureEcc::M,
        eci_assignment: None,
    };

    let transparent_payload = "https://example.test/transparent-caution";
    let transparent_encoded = encode(EncodeRequest {
        text: transparent_payload,
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: SUPPORTED_PROFILES[1].maximum_version(),
    })?;
    let transparent_source = render_png(&RenderModel::new(
        &transparent_encoded,
        RenderOptions::approved(
            SUPPORTED_PROFILES[1],
            Foreground::Brand,
            Background::Transparent,
        )?,
    )?)?;
    let transparent_source = adverse::composite_on(&transparent_source, [255, 255, 255, 255])?;
    let transparent_expected = DecodeExpectation {
        payload: transparent_payload.as_bytes().to_vec(),
        version: QrVersion::new(transparent_encoded.version().number())?,
        ecc: FixtureEcc::M,
        eci_assignment: None,
    };

    let logo_payload = "https://example.test/logo-caution";
    let logo_encoded = encode(EncodeRequest {
        text: logo_payload,
        ecc: ErrorCorrection::High,
        min_version: qr_core::Version::MINIMUM,
        max_version: SUPPORTED_PROFILES[1].maximum_version(),
    })?;
    let logo_source = render_png(&RenderModel::new(
        &logo_encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1])?.with_logo(LogoStyle::Bundled)?,
    )?)?;
    let logo_expected = DecodeExpectation {
        payload: logo_payload.as_bytes().to_vec(),
        version: QrVersion::new(logo_encoded.version().number())?,
        ecc: FixtureEcc::H,
        eci_assignment: None,
    };

    let configurations = [
        ("safe-square", "safe", safe_source, safe_expected),
        (
            "transparent",
            "caution",
            transparent_source,
            transparent_expected,
        ),
        ("logo", "caution", logo_source, logo_expected),
    ];
    let suite = adverse::TransformSuite::load()?;
    let output = tempfile::tempdir()?;
    let mut outcomes = Vec::new();

    for (configuration, safety, source, expected) in configurations {
        for transform in suite.transforms() {
            let included = match configuration {
                "safe-square" => true,
                "transparent" => matches!(
                    transform.kind(),
                    "blur"
                        | "scaling"
                        | "jpeg"
                        | "rotation"
                        | "contrast"
                        | "brightness"
                        | "background"
                        | "grayscale"
                ),
                "logo" => matches!(
                    transform.kind(),
                    "blur" | "scaling" | "jpeg" | "contrast" | "brightness" | "grayscale"
                ),
                _ => false,
            };
            if !included {
                continue;
            }
            let artifact = output
                .path()
                .join(format!("{configuration}-{}.png", transform.id()));
            let transformed = transform.apply(&source, suite.seed())?;
            assert!(
                adverse::pixels_differ(&transformed, &source)?,
                "{configuration}/{} had no observable pixel effect",
                transform.id()
            );
            fs::write(&artifact, transformed)?;
            decoder
                .inspect_and_compare(&artifact, &expected)
                .map_err(|error| format!("{configuration}/{}: {error}", transform.id()))?;
            outcomes.push(serde_json::json!({
                "configuration": configuration,
                "safety": safety,
                "transform": transform.id(),
                "decoder": manifest.decoder().version(),
                "outcome": "decoded",
            }));
        }
    }

    if let Some(evidence_dir) = std::env::var_os("QR_RELEASE_EVIDENCE_DIR") {
        let configured = Path::new(&evidence_dir);
        let evidence_dir = if configured.is_absolute() {
            configured.to_path_buf()
        } else {
            workspace.join(configured)
        };
        fs::create_dir_all(&evidence_dir)?;
        fs::write(
            evidence_dir.join("adverse-decode.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "parameters": "tests/adverse/parameters.json",
                "seed": suite.seed(),
                "outcomes": outcomes,
            }))?,
        )?;
    }
    Ok(())
}
