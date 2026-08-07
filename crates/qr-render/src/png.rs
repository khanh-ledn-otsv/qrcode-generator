use png::{BitDepth, ColorType, Compression, Encoder, Filter};

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
    for cell in model.cells().filter(|cell| cell.module().is_dark()) {
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
    Ok(pixels)
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
