use std::fmt::Write;

use crate::{
    GlyphOwnership, RenderError, RenderModel, Rgba, logo::bundled_logo_body,
    model::COMPACT_DOT_GEOMETRY,
};

/// Renders the validated model as deterministic, payload-free UTF-8 SVG.
pub fn render_svg(model: &RenderModel<'_>) -> Result<String, RenderError> {
    let placement = model.svg_placement();
    let output = placement.output_dimensions();
    let view_box = placement.view_box();
    let origin = placement.matrix_origin();
    let foreground = hex_color(model.options().foreground());

    let estimated_path_bytes = model
        .glyphs()
        .count()
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
    write!(
        svg,
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        view_box.width().get(),
        view_box.height().get(),
        hex_color(model.options().background()),
    )
    .map_err(|_| RenderError::RenderFailure)?;
    write!(svg, "<path fill=\"{}\" d=\"", foreground).map_err(|_| RenderError::RenderFailure)?;

    for glyph in model.glyphs() {
        let x = u32::from(glyph.x())
            .checked_add(origin.x().get())
            .ok_or(RenderError::DimensionOverflow)?;
        let y = u32::from(glyph.y())
            .checked_add(origin.y().get())
            .ok_or(RenderError::DimensionOverflow)?;
        match glyph.ownership() {
            GlyphOwnership::Finder => {
                write!(svg, "M{x} {y}h1v1h-1z").map_err(|_| RenderError::RenderFailure)?;
            }
            GlyphOwnership::Separator => return Err(RenderError::RenderFailure),
            GlyphOwnership::OtherFunction | GlyphOwnership::Data | GlyphOwnership::Remainder => {
                let radius = u32::from(COMPACT_DOT_GEOMETRY.radius_thousandths());
                let diameter = u32::from(COMPACT_DOT_GEOMETRY.diameter_thousandths());
                let left = x
                    .checked_mul(1_000)
                    .and_then(|value| value.checked_add(500 - radius))
                    .ok_or(RenderError::DimensionOverflow)?;
                let center_y = y
                    .checked_mul(1_000)
                    .and_then(|value| value.checked_add(500))
                    .ok_or(RenderError::DimensionOverflow)?;
                write!(
                    svg,
                    "M{} {}a{} {} 0 1 0 {} 0a{} {} 0 1 0-{} 0z",
                    decimal_thousandths(left),
                    decimal_thousandths(center_y),
                    decimal_thousandths(radius),
                    decimal_thousandths(radius),
                    decimal_thousandths(diameter),
                    decimal_thousandths(radius),
                    decimal_thousandths(radius),
                    decimal_thousandths(diameter),
                )
                .map_err(|_| RenderError::RenderFailure)?;
            }
        }
    }
    svg.push_str("\"/>");
    if let Some(logo) = model.logo_placement() {
        write_logo(&mut svg, model, logo)?;
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn decimal_thousandths(value: u32) -> String {
    decimal_fixed(value, 1_000, 3)
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
        .left_ten_thousandths()
        .checked_add(origin.x().get() * 10_000)
        .ok_or(RenderError::DimensionOverflow)?;
    let source_y = source
        .top_ten_thousandths()
        .checked_add(origin.y().get() * 10_000)
        .ok_or(RenderError::DimensionOverflow)?;
    let logo_body = bundled_logo_body()?;
    write!(
        svg,
        "<svg data-role=\"bundled-logo\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"180 180 640 240\" preserveAspectRatio=\"xMidYMid meet\" aria-hidden=\"true\">{logo_body}</svg>",
        decimal_fixed(source_x, 10_000, 4),
        decimal_fixed(source_y, 10_000, 4),
        decimal_fixed(source.width_ten_thousandths(), 10_000, 4),
        decimal_fixed(source.height_ten_thousandths(), 10_000, 4),
    )
    .map_err(|_| RenderError::RenderFailure)
}

fn decimal_fixed(value: u32, units_per_whole: u32, fractional_digits: usize) -> String {
    let whole = value / units_per_whole;
    let fraction = value % units_per_whole;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:0width$}", width = fractional_digits)
    }
}

fn hex_color(color: Rgba) -> String {
    let [red, green, blue, _] = color.channels();
    format!("#{red:02x}{green:02x}{blue:02x}")
}
