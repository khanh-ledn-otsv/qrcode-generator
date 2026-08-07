#[allow(dead_code)]
#[path = "support/styling.rs"]
mod styling;
#[allow(dead_code)]
#[path = "support/versions.rs"]
mod versions;

use std::error::Error;
use std::fs;
use std::path::Path;

use qr_render::{RenderModel, render_png, render_svg};
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
fn approved_matrix_artifacts_and_allocations_stay_within_recorded_baselines()
-> Result<(), Box<dyn Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baselines: ResourceBaselines =
        serde_json::from_slice(&fs::read(workspace.join("tests/baselines/resources.json"))?)?;
    assert_eq!(baselines.schema_version, 1);

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

    assert!(
        observed_svg <= baselines.approved_matrix.maximum_svg_bytes
            && observed_png <= baselines.approved_matrix.maximum_png_bytes
            && observed_rgba <= baselines.approved_matrix.maximum_rgba_buffer_bytes,
        "observed SVG/PNG/RGBA maxima were {observed_svg}/{observed_png}/{observed_rgba}; baselines are {}/{}/{}",
        baselines.approved_matrix.maximum_svg_bytes,
        baselines.approved_matrix.maximum_png_bytes,
        baselines.approved_matrix.maximum_rgba_buffer_bytes,
    );
    Ok(())
}
