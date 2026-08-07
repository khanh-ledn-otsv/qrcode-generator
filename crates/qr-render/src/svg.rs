use std::fmt::Write;

use crate::{Background, DataModuleStyle, RenderError, RenderModel, Rgba, logo::bundled_logo_body};

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
    write!(
        svg,
        "<path fill=\"{}\" shape-rendering=\"crispEdges\" d=\"",
        foreground
    )
    .map_err(|_| RenderError::RenderFailure)?;

    let logo_placement = model.logo_placement();
    for cell in model.cells().filter(|cell| cell.module().is_dark()) {
        if logo_placement.is_some_and(|logo| {
            logo.knockout_bounds()
                .contains(u32::from(cell.x()), u32::from(cell.y()))
        }) {
            continue;
        }
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
    svg.push_str("\"/>");
    if let Some(logo) = logo_placement {
        write_logo(&mut svg, model, logo)?;
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn write_logo(
    svg: &mut String,
    model: &RenderModel<'_>,
    logo: crate::LogoPlacement,
) -> Result<(), RenderError> {
    let origin = model.svg_placement().matrix_origin();
    let knockout = logo.knockout_bounds();
    let knockout_x = origin.x().get() + knockout.left().get();
    let knockout_y = origin.y().get() + knockout.top().get();
    write!(
        svg,
        "<rect data-role=\"logo-knockout\" x=\"{knockout_x}\" y=\"{knockout_y}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>",
        knockout.width().get(),
        knockout.height().get(),
    )
    .map_err(|_| RenderError::RenderFailure)?;

    let source = logo.source_bounds();
    let source_x = source
        .left_thousandths()
        .checked_add(origin.x().get() * 1_000)
        .ok_or(RenderError::DimensionOverflow)?;
    let source_y = source
        .top_thousandths()
        .checked_add(origin.y().get() * 1_000)
        .ok_or(RenderError::DimensionOverflow)?;
    let logo_body = bundled_logo_body()?;
    write!(
        svg,
        "<svg data-role=\"bundled-logo\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 1000 602\" preserveAspectRatio=\"xMidYMid meet\" aria-hidden=\"true\">{logo_body}</svg>",
        decimal_thousandths(source_x),
        decimal_thousandths(source_y),
        decimal_thousandths(source.width_thousandths()),
        decimal_thousandths(source.height_thousandths()),
    )
    .map_err(|_| RenderError::RenderFailure)
}

fn decimal_thousandths(value: u32) -> String {
    let whole = value / 1_000;
    let fraction = value % 1_000;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:03}")
    }
}

fn hex_color(color: Rgba) -> String {
    let [red, green, blue, _] = color.channels();
    format!("#{red:02x}{green:02x}{blue:02x}")
}
