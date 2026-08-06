use std::fmt::Write;

use crate::{Background, RenderError, RenderModel, Rgba};

/// Renders the validated model as deterministic, payload-free UTF-8 SVG.
pub fn render_svg(model: &RenderModel<'_>) -> Result<String, RenderError> {
    let placement = model.svg_placement();
    let output = placement.output_dimensions();
    let view_box = placement.view_box();
    let origin = placement.matrix_origin();
    let foreground = hex_color(model.options().foreground());
    let background = match model.options().background() {
        Background::Opaque(color) => hex_color(color),
    };

    let dark_modules = model.cells().filter(|cell| cell.module().is_dark()).count();
    let estimated_path_bytes = dark_modules
        .checked_mul(20)
        .ok_or(RenderError::DimensionOverflow)?;
    let estimated_bytes = estimated_path_bytes
        .checked_add(256)
        .ok_or(RenderError::DimensionOverflow)?;
    let mut svg = String::new();
    svg.try_reserve(estimated_bytes)
        .map_err(|_| RenderError::RenderFailure)?;

    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"{}\" height=\"{}\" fill=\"{}\"/><path fill=\"{}\" d=\"",
        output.width().get(),
        output.height().get(),
        view_box.width().get(),
        view_box.height().get(),
        view_box.width().get(),
        view_box.height().get(),
        background,
        foreground,
    )
    .map_err(|_| RenderError::RenderFailure)?;

    for cell in model.cells().filter(|cell| cell.module().is_dark()) {
        let x = u32::from(cell.x())
            .checked_add(origin.x().get())
            .ok_or(RenderError::DimensionOverflow)?;
        let y = u32::from(cell.y())
            .checked_add(origin.y().get())
            .ok_or(RenderError::DimensionOverflow)?;
        write!(svg, "M{x} {y}h1v1h-1z").map_err(|_| RenderError::RenderFailure)?;
    }
    svg.push_str("\"/></svg>");
    Ok(svg)
}

fn hex_color(color: Rgba) -> String {
    let [red, green, blue, _] = color.channels();
    format!("#{red:02x}{green:02x}{blue:02x}")
}
