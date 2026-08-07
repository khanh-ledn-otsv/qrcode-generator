use png::{BitDepth, ColorType, Compression, Encoder, Filter};

use crate::logo::{logo_contains_source_point, source_view_box};
use crate::{Background, DataModuleStyle, PixelDimensions, RenderError, RenderModel, Rgba};

const COVERAGE_SAMPLES_PER_AXIS: u32 = 8;
const FULL_COVERAGE: u32 = COVERAGE_SAMPLES_PER_AXIS * COVERAGE_SAMPLES_PER_AXIS;

/// Renders the validated model as a deterministic, metadata-free PNG artifact.
pub fn render_png(model: &RenderModel<'_>) -> Result<Vec<u8>, RenderError> {
    let pixels = render_rgba(model)?;
    serialize_png(model.png_placement().canvas_dimensions(), &pixels)
}

fn render_rgba(model: &RenderModel<'_>) -> Result<Vec<u8>, RenderError> {
    let placement = model.png_placement();
    let background = match model.options().background() {
        Background::Opaque(color) => color.channels(),
        Background::Transparent => [0, 0, 0, 0],
    };
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(placement.rgba_buffer_len())
        .map_err(|_| RenderError::RenderFailure)?;
    pixels.resize(placement.rgba_buffer_len(), 0);
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background);
    }

    let dimensions = placement.canvas_dimensions();
    let origin = placement.matrix_origin();
    let scale = placement.module_scale().get();
    let logo_placement = model.logo_placement();
    for cell in model.cells().filter(|cell| cell.module().is_dark()) {
        if logo_placement.is_some_and(|logo| {
            logo.knockout_bounds()
                .contains(u32::from(cell.x()), u32::from(cell.y()))
        }) {
            continue;
        }
        let x = u32::from(cell.x())
            .checked_mul(scale)
            .and_then(|offset| origin.x().get().checked_add(offset))
            .ok_or(RenderError::DimensionOverflow)?;
        let y = u32::from(cell.y())
            .checked_mul(scale)
            .and_then(|offset| origin.y().get().checked_add(offset))
            .ok_or(RenderError::DimensionOverflow)?;
        if model.options().data_module_style() == DataModuleStyle::Rounded && !cell.is_protected() {
            fill_rounded(
                &mut pixels,
                dimensions,
                x,
                y,
                scale,
                model.options().foreground(),
                model.options().background(),
            )?;
        } else {
            fill_square(
                &mut pixels,
                dimensions,
                x,
                y,
                scale,
                model.options().foreground(),
            )?;
        }
    }
    if let Some(logo) = logo_placement {
        render_logo(&mut pixels, dimensions, origin, scale, logo)?;
    }
    Ok(pixels)
}

fn render_logo(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    matrix_origin: crate::PixelPoint,
    scale: u32,
    logo: crate::LogoPlacement,
) -> Result<(), RenderError> {
    let knockout = logo.knockout_bounds();
    let knockout_x = knockout
        .left()
        .get()
        .checked_mul(scale)
        .and_then(|offset| matrix_origin.x().get().checked_add(offset))
        .ok_or(RenderError::DimensionOverflow)?;
    let knockout_y = knockout
        .top()
        .get()
        .checked_mul(scale)
        .and_then(|offset| matrix_origin.y().get().checked_add(offset))
        .ok_or(RenderError::DimensionOverflow)?;
    fill_rectangle(
        pixels,
        dimensions,
        knockout_x,
        knockout_y,
        knockout.width().get() * scale,
        knockout.height().get() * scale,
        Rgba::WHITE,
    )?;

    let source = logo.source_bounds();
    let source_left_pixels = f64::from(matrix_origin.x().get())
        + f64::from(source.left_thousandths()) * f64::from(scale) / 1_000.0;
    let source_top_pixels = f64::from(matrix_origin.y().get())
        + f64::from(source.top_thousandths()) * f64::from(scale) / 1_000.0;
    let source_width_pixels = f64::from(source.width_thousandths()) * f64::from(scale) / 1_000.0;
    let source_height_pixels = f64::from(source.height_thousandths()) * f64::from(scale) / 1_000.0;
    let x_start = source_left_pixels.floor() as u32;
    let y_start = source_top_pixels.floor() as u32;
    let x_end = (source_left_pixels + source_width_pixels).ceil() as u32;
    let y_end = (source_top_pixels + source_height_pixels).ceil() as u32;
    let (view_box_width, view_box_height) = source_view_box();
    let canvas_width = u64::from(dimensions.width().get());
    for y in y_start..y_end {
        for x in x_start..x_end {
            let source_x = (f64::from(x) + 0.5 - source_left_pixels) * f64::from(view_box_width)
                / source_width_pixels;
            let source_y = (f64::from(y) + 0.5 - source_top_pixels) * f64::from(view_box_height)
                / source_height_pixels;
            if !logo_contains_source_point(source_x, source_y) {
                continue;
            }
            let offset = u64::from(y)
                .checked_mul(canvas_width)
                .and_then(|value| value.checked_add(u64::from(x)))
                .and_then(|value| value.checked_mul(4))
                .and_then(|value| usize::try_from(value).ok())
                .ok_or(RenderError::DimensionOverflow)?;
            pixels
                .get_mut(offset..offset + 4)
                .ok_or(RenderError::RenderFailure)?
                .copy_from_slice(&Rgba::BRAND.channels());
        }
    }
    Ok(())
}

fn fill_rectangle(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgba,
) -> Result<(), RenderError> {
    let canvas_width = u64::from(dimensions.width().get());
    let x_end = x.checked_add(width).ok_or(RenderError::DimensionOverflow)?;
    let y_end = y
        .checked_add(height)
        .ok_or(RenderError::DimensionOverflow)?;
    if x_end > dimensions.width().get() || y_end > dimensions.height().get() {
        return Err(RenderError::RenderFailure);
    }
    for row in y..y_end {
        let start = u64::from(row)
            .checked_mul(canvas_width)
            .and_then(|value| value.checked_add(u64::from(x)))
            .and_then(|value| value.checked_mul(4))
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(RenderError::DimensionOverflow)?;
        let end = start
            .checked_add(usize::try_from(width * 4).map_err(|_| RenderError::DimensionOverflow)?)
            .ok_or(RenderError::DimensionOverflow)?;
        for pixel in pixels
            .get_mut(start..end)
            .ok_or(RenderError::RenderFailure)?
            .chunks_exact_mut(4)
        {
            pixel.copy_from_slice(&color.channels());
        }
    }
    Ok(())
}

fn fill_rounded(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    side: u32,
    foreground: Rgba,
    background: Background,
) -> Result<(), RenderError> {
    let x_end = x.checked_add(side).ok_or(RenderError::DimensionOverflow)?;
    let y_end = y.checked_add(side).ok_or(RenderError::DimensionOverflow)?;
    if x_end > dimensions.width().get() || y_end > dimensions.height().get() {
        return Err(RenderError::RenderFailure);
    }

    let width = u64::from(dimensions.width().get());
    for local_y in 0..side {
        for local_x in 0..side {
            let coverage = rounded_pixel_coverage(local_x, local_y, side);
            if coverage == 0 {
                continue;
            }
            let offset = u64::from(y + local_y)
                .checked_mul(width)
                .and_then(|offset| offset.checked_add(u64::from(x + local_x)))
                .and_then(|offset| offset.checked_mul(4))
                .and_then(|offset| usize::try_from(offset).ok())
                .ok_or(RenderError::DimensionOverflow)?;
            let pixel = pixels
                .get_mut(offset..offset + 4)
                .ok_or(RenderError::RenderFailure)?;
            pixel.copy_from_slice(&covered_color(foreground, background, coverage));
        }
    }
    Ok(())
}

fn rounded_pixel_coverage(pixel_x: u32, pixel_y: u32, side: u32) -> u32 {
    // Sixteen integer units per output pixel place eight samples at exact
    // subpixel centers. A radius of `side * 4` is exactly one quarter of the
    // cell in those units, so coverage is target-independent and needs no
    // resized intermediate image.
    let cell_side = u64::from(side) * 16;
    let radius = u64::from(side) * 4;
    let far_edge = cell_side - radius;
    let radius_squared = radius * radius;
    let mut coverage = 0;
    for sample_y in 0..COVERAGE_SAMPLES_PER_AXIS {
        let point_y = u64::from(pixel_y) * 16 + u64::from(sample_y * 2 + 1);
        let distance_y = corner_distance(point_y, radius, far_edge);
        for sample_x in 0..COVERAGE_SAMPLES_PER_AXIS {
            let point_x = u64::from(pixel_x) * 16 + u64::from(sample_x * 2 + 1);
            let distance_x = corner_distance(point_x, radius, far_edge);
            if distance_x * distance_x + distance_y * distance_y <= radius_squared {
                coverage += 1;
            }
        }
    }
    coverage
}

fn corner_distance(point: u64, near_edge: u64, far_edge: u64) -> u64 {
    if point < near_edge {
        near_edge - point
    } else {
        point.saturating_sub(far_edge)
    }
}

fn covered_color(foreground: Rgba, background: Background, coverage: u32) -> [u8; 4] {
    let foreground = foreground.channels();
    match background {
        Background::Transparent => [
            foreground[0],
            foreground[1],
            foreground[2],
            u8::try_from((u32::from(u8::MAX) * coverage + FULL_COVERAGE / 2) / FULL_COVERAGE)
                .unwrap_or(u8::MAX),
        ],
        Background::Opaque(background) => {
            let background = background.channels();
            let blend = |channel: usize| {
                let value = u32::from(foreground[channel]) * coverage
                    + u32::from(background[channel]) * (FULL_COVERAGE - coverage);
                u8::try_from((value + FULL_COVERAGE / 2) / FULL_COVERAGE).unwrap_or(u8::MAX)
            };
            [blend(0), blend(1), blend(2), u8::MAX]
        }
    }
}

fn fill_square(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    side: u32,
    color: Rgba,
) -> Result<(), RenderError> {
    let width = u64::from(dimensions.width().get());
    let x_end = x.checked_add(side).ok_or(RenderError::DimensionOverflow)?;
    let y_end = y.checked_add(side).ok_or(RenderError::DimensionOverflow)?;
    if x_end > dimensions.width().get() || y_end > dimensions.height().get() {
        return Err(RenderError::RenderFailure);
    }

    let channels = color.channels();
    for row in y..y_end {
        let start = u64::from(row)
            .checked_mul(width)
            .and_then(|offset| offset.checked_add(u64::from(x)))
            .and_then(|offset| offset.checked_mul(4))
            .and_then(|offset| usize::try_from(offset).ok())
            .ok_or(RenderError::DimensionOverflow)?;
        let byte_width = u64::from(side)
            .checked_mul(4)
            .and_then(|length| usize::try_from(length).ok())
            .ok_or(RenderError::DimensionOverflow)?;
        let end = start
            .checked_add(byte_width)
            .ok_or(RenderError::DimensionOverflow)?;
        let row_pixels = pixels
            .get_mut(start..end)
            .ok_or(RenderError::RenderFailure)?;
        for pixel in row_pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&channels);
        }
    }
    Ok(())
}

fn serialize_png(dimensions: PixelDimensions, pixels: &[u8]) -> Result<Vec<u8>, RenderError> {
    let mut artifact = Vec::new();
    {
        let mut encoder = Encoder::new(
            &mut artifact,
            dimensions.width().get(),
            dimensions.height().get(),
        );
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Balanced);
        encoder.set_filter(Filter::NoFilter);
        encoder.validate_sequence(true);
        let mut writer = encoder
            .write_header()
            .map_err(|_| RenderError::RenderFailure)?;
        writer
            .write_image_data(pixels)
            .map_err(|_| RenderError::RenderFailure)?;
        writer.finish().map_err(|_| RenderError::RenderFailure)?;
    }
    Ok(artifact)
}
