use std::fmt::Write;

use crate::{Background, DataModuleStyle, RenderError, RenderModel, Rgba};

/// Renders the validated model as deterministic, payload-free UTF-8 SVG.
pub fn render_svg(model: &RenderModel<'_>) -> Result<String, RenderError> {
    let placement = model.svg_placement();
    let output = placement.output_dimensions();
    let view_box = placement.view_box();
    let origin = placement.matrix_origin();
    let foreground = hex_color(model.options().foreground());

    let dark_modules = model.cells().filter(|cell| cell.module().is_dark()).count();
    let estimated_path_bytes = dark_modules
        .checked_mul(96)
        .ok_or(RenderError::DimensionOverflow)?;
    let estimated_bytes = estimated_path_bytes
        .checked_add(256)
        .ok_or(RenderError::DimensionOverflow)?;
    let mut svg = String::new();
    svg.try_reserve(estimated_bytes)
        .map_err(|_| RenderError::RenderFailure)?;

    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        output.width().get(),
        output.height().get(),
        view_box.width().get(),
        view_box.height().get(),
    )
    .map_err(|_| RenderError::RenderFailure)?;
    if let Background::Opaque(background) = model.options().background() {
        write!(
            svg,
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            view_box.width().get(),
            view_box.height().get(),
            hex_color(background),
        )
        .map_err(|_| RenderError::RenderFailure)?;
    }
    write!(svg, "<path fill=\"{}\" d=\"", foreground).map_err(|_| RenderError::RenderFailure)?;

    for cell in model.cells().filter(|cell| cell.module().is_dark()) {
        let x = u32::from(cell.x())
            .checked_add(origin.x().get())
            .ok_or(RenderError::DimensionOverflow)?;
        let y = u32::from(cell.y())
            .checked_add(origin.y().get())
            .ok_or(RenderError::DimensionOverflow)?;
        if model.options().data_module_style() == DataModuleStyle::Rounded && !cell.is_protected() {
            write!(
                svg,
                "M{x}.25 {y}h.5a.25.25 0 0 1 .25.25v.5a.25.25 0 0 1-.25.25h-.5a.25.25 0 0 1-.25-.25v-.5a.25.25 0 0 1 .25-.25z"
            )
            .map_err(|_| RenderError::RenderFailure)?;
        } else {
            write!(svg, "M{x} {y}h1v1h-1z").map_err(|_| RenderError::RenderFailure)?;
        }
    }
    svg.push_str("\"/></svg>");
    Ok(svg)
}

fn hex_color(color: Rgba) -> String {
    let [red, green, blue, _] = color.channels();
    format!("#{red:02x}{green:02x}{blue:02x}")
}
