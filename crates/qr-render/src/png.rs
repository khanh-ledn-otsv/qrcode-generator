use png::{BitDepth, ColorType, Compression, Encoder, Filter};

use crate::{Background, PixelDimensions, RenderError, RenderModel, Rgba};

/// Renders the validated model as a deterministic, metadata-free PNG artifact.
pub fn render_png(model: &RenderModel<'_>) -> Result<Vec<u8>, RenderError> {
    let pixels = render_rgba(model)?;
    serialize_png(model.png_placement().canvas_dimensions(), &pixels)
}

fn render_rgba(model: &RenderModel<'_>) -> Result<Vec<u8>, RenderError> {
    let placement = model.png_placement();
    let Background::Opaque(background) = model.options().background();
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(placement.rgba_buffer_len())
        .map_err(|_| RenderError::RenderFailure)?;
    pixels.resize(placement.rgba_buffer_len(), 0);
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background.channels());
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
        fill_square(
            &mut pixels,
            dimensions,
            x,
            y,
            scale,
            model.options().foreground(),
        )?;
    }
    Ok(pixels)
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
