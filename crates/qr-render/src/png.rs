use png::{BitDepth, ColorType, Compression, Encoder, Filter};

use crate::logo::{logo_contains_source_point, source_view_box};
use crate::model::COMPACT_DOT_GEOMETRY;
use crate::{GlyphOwnership, PixelDimensions, RenderError, RenderModel, Rgba};

const LOGO_SAMPLES_PER_AXIS: u32 = 4;
const LOGO_SAMPLE_COUNT: u32 = LOGO_SAMPLES_PER_AXIS * LOGO_SAMPLES_PER_AXIS;

/// Renders the validated model as a deterministic, metadata-free PNG artifact.
pub fn render_png(model: &RenderModel<'_>) -> Result<Vec<u8>, RenderError> {
    let pixels = render_rgba(model)?;
    serialize_png(model.png_placement().canvas_dimensions(), &pixels)
}

fn render_rgba(model: &RenderModel<'_>) -> Result<Vec<u8>, RenderError> {
    let placement = model.png_placement();
    let background = model.options().background().channels();
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
    for glyph in model.glyphs() {
        let x = u32::from(glyph.x())
            .checked_mul(scale)
            .and_then(|offset| origin.x().get().checked_add(offset))
            .ok_or(RenderError::DimensionOverflow)?;
        let y = u32::from(glyph.y())
            .checked_mul(scale)
            .and_then(|offset| origin.y().get().checked_add(offset))
            .ok_or(RenderError::DimensionOverflow)?;
        match glyph.ownership() {
            GlyphOwnership::Finder => fill_square(
                &mut pixels,
                dimensions,
                x,
                y,
                scale,
                model.options().foreground(),
            )?,
            GlyphOwnership::Separator => return Err(RenderError::RenderFailure),
            GlyphOwnership::OtherFunction | GlyphOwnership::Data | GlyphOwnership::Remainder => {
                fill_dot(
                    &mut pixels,
                    dimensions,
                    x,
                    y,
                    scale,
                    model.options().foreground(),
                    model.options().background(),
                )?;
            }
        }
    }
    if let Some(logo) = model.logo_placement() {
        render_logo(
            &mut pixels,
            dimensions,
            origin,
            scale,
            logo,
            model.options().foreground(),
        )?;
    }
    Ok(pixels)
}

fn render_logo(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    matrix_origin: crate::PixelPoint,
    scale: u32,
    logo: crate::LogoPlacement,
    foreground: Rgba,
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
        + f64::from(source.left_ten_thousandths()) * f64::from(scale) / 10_000.0;
    let source_top_pixels = f64::from(matrix_origin.y().get())
        + f64::from(source.top_ten_thousandths()) * f64::from(scale) / 10_000.0;
    let source_width_pixels =
        f64::from(source.width_ten_thousandths()) * f64::from(scale) / 10_000.0;
    let source_height_pixels =
        f64::from(source.height_ten_thousandths()) * f64::from(scale) / 10_000.0;
    let x_start = source_left_pixels.floor() as u32;
    let y_start = source_top_pixels.floor() as u32;
    let x_end = (source_left_pixels + source_width_pixels).ceil() as u32;
    let y_end = (source_top_pixels + source_height_pixels).ceil() as u32;
    let (view_box_left, view_box_top, view_box_width, view_box_height) = source_view_box();
    let canvas_width = u64::from(dimensions.width().get());
    for y in y_start..y_end {
        for x in x_start..x_end {
            let mut covered_samples = 0;
            for sample_y in 0..LOGO_SAMPLES_PER_AXIS {
                for sample_x in 0..LOGO_SAMPLES_PER_AXIS {
                    let pixel_x = f64::from(x)
                        + (f64::from(sample_x) + 0.5) / f64::from(LOGO_SAMPLES_PER_AXIS);
                    let pixel_y = f64::from(y)
                        + (f64::from(sample_y) + 0.5) / f64::from(LOGO_SAMPLES_PER_AXIS);
                    let source_x = f64::from(view_box_left)
                        + (pixel_x - source_left_pixels) * f64::from(view_box_width)
                            / source_width_pixels;
                    let source_y = f64::from(view_box_top)
                        + (pixel_y - source_top_pixels) * f64::from(view_box_height)
                            / source_height_pixels;
                    if logo_contains_source_point(source_x, source_y) {
                        covered_samples += 1;
                    }
                }
            }
            if covered_samples == 0 {
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
                .copy_from_slice(&logo_pixel(covered_samples, foreground)?);
        }
    }
    Ok(())
}

fn logo_pixel(covered_samples: u32, foreground: Rgba) -> Result<[u8; 4], RenderError> {
    if covered_samples == LOGO_SAMPLE_COUNT {
        return Ok(foreground.channels());
    }
    let [red, green, blue, _] = foreground.channels();
    Ok([
        blend_logo_channel(red, covered_samples)?,
        blend_logo_channel(green, covered_samples)?,
        blend_logo_channel(blue, covered_samples)?,
        u8::MAX,
    ])
}

fn blend_logo_channel(channel: u8, covered_samples: u32) -> Result<u8, RenderError> {
    let uncovered_samples = LOGO_SAMPLE_COUNT - covered_samples;
    let blended = u32::from(channel) * covered_samples + 255 * uncovered_samples;
    u8::try_from((blended + LOGO_SAMPLE_COUNT / 2) / LOGO_SAMPLE_COUNT)
        .map_err(|_| RenderError::RenderFailure)
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

fn fill_dot(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    side: u32,
    foreground: Rgba,
    background: Rgba,
) -> Result<(), RenderError> {
    let x_end = x.checked_add(side).ok_or(RenderError::DimensionOverflow)?;
    let y_end = y.checked_add(side).ok_or(RenderError::DimensionOverflow)?;
    if x_end > dimensions.width().get() || y_end > dimensions.height().get() {
        return Err(RenderError::RenderFailure);
    }
    let samples = u32::from(COMPACT_DOT_GEOMETRY.samples_per_axis());
    let center = u64::from(side)
        .checked_mul(u64::from(samples))
        .ok_or(RenderError::DimensionOverflow)?;
    let radius = center
        .checked_mul(u64::from(COMPACT_DOT_GEOMETRY.diameter_thousandths()))
        .ok_or(RenderError::DimensionOverflow)?;
    let radius_squared = radius
        .checked_mul(radius)
        .ok_or(RenderError::DimensionOverflow)?;
    for offset_y in 0..side {
        for offset_x in 0..side {
            let mut covered = 0;
            for sample_y in 0..samples {
                for sample_x in 0..samples {
                    let sample_x =
                        u64::from(offset_x) * u64::from(samples) * 2 + u64::from(sample_x) * 2 + 1;
                    let sample_y =
                        u64::from(offset_y) * u64::from(samples) * 2 + u64::from(sample_y) * 2 + 1;
                    let dx = sample_x.abs_diff(center);
                    let dy = sample_y.abs_diff(center);
                    let distance = dx
                        .checked_mul(dx)
                        .and_then(|value| dy.checked_mul(dy).and_then(|dy| value.checked_add(dy)))
                        .and_then(|value| value.checked_mul(1_000_000))
                        .ok_or(RenderError::DimensionOverflow)?;
                    if distance <= radius_squared {
                        covered += 1;
                    }
                }
            }
            if covered > 0 {
                blend_dot_pixel(
                    pixels,
                    dimensions,
                    x + offset_x,
                    y + offset_y,
                    foreground,
                    background,
                    covered,
                )?;
            }
        }
    }
    Ok(())
}

fn blend_dot_pixel(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    foreground: Rgba,
    background: Rgba,
    covered: u32,
) -> Result<(), RenderError> {
    let offset = u64::from(y)
        .checked_mul(u64::from(dimensions.width().get()))
        .and_then(|value| value.checked_add(u64::from(x)))
        .and_then(|value| value.checked_mul(4))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(RenderError::DimensionOverflow)?;
    let pixel = pixels
        .get_mut(offset..offset + 4)
        .ok_or(RenderError::RenderFailure)?;
    let foreground = foreground.channels();
    let background = background.channels();
    let samples = u32::from(COMPACT_DOT_GEOMETRY.samples_per_axis());
    let total = samples * samples;
    for channel in 0..3 {
        let blended = u32::from(foreground[channel]) * covered
            + u32::from(background[channel]) * (total - covered);
        pixel[channel] =
            u8::try_from((blended + total / 2) / total).map_err(|_| RenderError::RenderFailure)?;
    }
    pixel[3] = u8::MAX;
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
