#[allow(dead_code)]
#[path = "support/styling.rs"]
mod styling;
#[allow(dead_code)]
#[path = "support/versions.rs"]
mod versions;

use std::error::Error;
use std::fs;
use std::path::Path;

use qr_render::{RenderModel, SUPPORTED_PROFILES, render_png, render_svg};
use serde::Deserialize;

#[derive(Deserialize)]
struct ResourceBaselines {
    schema_version: u8,
    approved_matrix: ApprovedMatrixBaselines,
}

#[derive(Deserialize)]
struct ApprovedMatrixBaselines {
    maximum_svg_bytes: usize,
    maximum_png_bytes: usize,
    maximum_rgba_buffer_bytes: usize,
}

#[test]
fn representative_profile_and_logo_artifacts_stay_within_recorded_resource_baselines()
-> Result<(), Box<dyn Error>> {
    let baselines = resource_baselines()?;
    let cases = styling::representative_decode_cases()?;
    assert!(cases.len() < styling::approved_style_tuples().len());

    for case in cases {
        let model = RenderModel::new(&case.encoded, case.options)?;
        let svg = render_svg(&model)?;
        let png = render_png(&model)?;
        assert_within_baselines(
            &baselines,
            svg.len(),
            png.len(),
            model.png_placement().rgba_buffer_len(),
        );
    }
    Ok(())
}

#[test]
fn largest_approved_artifact_stays_within_recorded_resource_baselines() -> Result<(), Box<dyn Error>>
{
    let baselines = resource_baselines()?;
    let profile = SUPPORTED_PROFILES
        .last()
        .ok_or("approved profile is compiled")?;
    let encoded = qr_core::encode(qr_core::EncodeRequest::with_version_range(
        "largest approved artifact",
        qr_core::tables::ErrorCorrection::Medium,
        profile.maximum_version(),
        profile.maximum_version(),
    ))?;
    let model = RenderModel::new(&encoded, qr_render::RenderOptions::safe(*profile)?)?;
    let svg = render_svg(&model)?;
    let png = render_png(&model)?;
    let dimensions = profile.png_dimensions_for(profile.maximum_version())?;
    let observed_rgba = usize::try_from(dimensions.width().get())?
        .checked_mul(usize::try_from(dimensions.height().get())?)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("approved RGBA allocation fits usize")?;

    assert_within_baselines(&baselines, svg.len(), png.len(), observed_rgba);
    assert_eq!(
        observed_rgba,
        baselines.approved_matrix.maximum_rgba_buffer_bytes
    );
    Ok(())
}

#[test]
#[ignore = "exhaustive approved matrix runs in release evidence and extended CI"]
fn approved_matrix_artifacts_and_allocations_stay_within_recorded_baselines()
-> Result<(), Box<dyn Error>> {
    let baselines = resource_baselines()?;
    let mut observed_svg = 0;
    let mut observed_png = 0;
    let mut observed_rgba = 0;
    for case in styling::approved_decode_cases()?
        .into_iter()
        .filter(|case| !case.label.contains("transition-version-"))
    {
        let model = RenderModel::new(&case.encoded, case.options)?;
        observed_svg = observed_svg.max(render_svg(&model)?.len());
        observed_png = observed_png.max(render_png(&model)?.len());
        observed_rgba = observed_rgba.max(model.png_placement().rgba_buffer_len());
    }

    assert_within_baselines(&baselines, observed_svg, observed_png, observed_rgba);
    Ok(())
}

fn resource_baselines() -> Result<ResourceBaselines, Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baselines: ResourceBaselines =
        serde_json::from_slice(&fs::read(workspace.join("tests/baselines/resources.json"))?)?;
    assert_eq!(baselines.schema_version, 1);
    Ok(baselines)
}

fn assert_within_baselines(
    baselines: &ResourceBaselines,
    observed_svg: usize,
    observed_png: usize,
    observed_rgba: usize,
) {
    assert!(
        observed_svg <= baselines.approved_matrix.maximum_svg_bytes
            && observed_png <= baselines.approved_matrix.maximum_png_bytes
            && observed_rgba <= baselines.approved_matrix.maximum_rgba_buffer_bytes,
        "observed SVG/PNG/RGBA maxima were {observed_svg}/{observed_png}/{observed_rgba}; baselines are {}/{}/{}",
        baselines.approved_matrix.maximum_svg_bytes,
        baselines.approved_matrix.maximum_png_bytes,
        baselines.approved_matrix.maximum_rgba_buffer_bytes,
    );
}
