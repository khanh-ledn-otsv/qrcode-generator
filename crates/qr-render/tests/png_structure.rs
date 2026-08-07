#[path = "support/png_fixture.rs"]
mod png_fixture;
#[path = "support/versions.rs"]
mod versions;

use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FOREGROUNDS, Background, RenderModel, RenderOptions,
    SUPPORTED_PROFILES, render_png,
};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[test]
fn safe_png_has_fixed_structure_and_deterministic_bytes() {
    let first = png_fixture::artifact();
    let second = png_fixture::artifact();

    assert_eq!(first, second);
    assert_eq!(png_fixture::sha256_hex(&first), png_fixture::SHA256);
    assert!(first.starts_with(PNG_SIGNATURE));
    assert!(
        !first
            .windows(png_fixture::PAYLOAD.len())
            .any(|window| window == png_fixture::PAYLOAD.as_bytes())
    );

    let chunks = png_chunks(&first);
    assert_eq!(
        chunks.iter().map(|chunk| chunk.kind).collect::<Vec<_>>(),
        vec![*b"IHDR", *b"IDAT", *b"IEND"]
    );

    let ihdr = chunks[0].data;
    let dimensions = SUPPORTED_PROFILES[1].png_dimensions();
    assert_eq!(
        u32::from_be_bytes(ihdr[0..4].try_into().unwrap()),
        dimensions.width().get()
    );
    assert_eq!(
        u32::from_be_bytes(ihdr[4..8].try_into().unwrap()),
        dimensions.height().get()
    );
    assert_eq!(&ihdr[8..], &[8, 6, 0, 0, 0]);
}

#[test]
fn decoded_pixels_are_exact_background_or_integer_module_rectangles() {
    for profile in SUPPORTED_PROFILES {
        for version in 1..=profile.maximum_version().number() {
            let encoded = encoded_qr_at_version(version);
            let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
            let png = render_png(&model).unwrap();
            let (width, height, pixels) = decode_rgba(&png);

            assert_eq!(width, profile.png_dimensions().width().get());
            assert_eq!(height, profile.png_dimensions().height().get());
            assert_eq!(pixels.len(), model.png_placement().rgba_buffer_len());

            let placement = model.png_placement();
            let origin_x = placement.matrix_origin().x().get();
            let origin_y = placement.matrix_origin().y().get();
            let scale = placement.module_scale().get();
            let matrix_size = u32::from(model.matrix().size());

            for y in 0..height {
                for x in 0..width {
                    let expected = if x >= origin_x
                        && y >= origin_y
                        && x < origin_x + matrix_size * scale
                        && y < origin_y + matrix_size * scale
                    {
                        let module_x = u16::try_from((x - origin_x) / scale).unwrap();
                        let module_y = u16::try_from((y - origin_y) / scale).unwrap();
                        if model
                            .matrix()
                            .module(module_x, module_y)
                            .is_some_and(|module| module.is_dark())
                        {
                            [0, 0, 0, 255]
                        } else {
                            [255, 255, 255, 255]
                        }
                    } else {
                        [255, 255, 255, 255]
                    };
                    let offset = usize::try_from((y * width + x) * 4).unwrap();
                    assert_eq!(&pixels[offset..offset + 4], &expected);
                }
            }
        }
    }
}

#[test]
fn approved_png_color_background_profile_tuples_are_structural_and_deterministic() {
    let encoded = encoded_qr_at_version(1);

    for profile in SUPPORTED_PROFILES {
        for foreground in APPROVED_FOREGROUNDS {
            for background in APPROVED_BACKGROUNDS {
                let options = RenderOptions::approved(profile, foreground, background).unwrap();
                let model = RenderModel::new(&encoded, options).unwrap();
                let first = render_png(&model).unwrap();
                assert_eq!(first, render_png(&model).unwrap());

                let (width, _, pixels) = decode_rgba(&first);
                let background_pixel = match background {
                    Background::Opaque(color) => color.channels(),
                    Background::Transparent => [0, 0, 0, 0],
                };
                assert_eq!(&pixels[0..4], &background_pixel);

                let dark = model.cells().find(|cell| cell.module().is_dark()).unwrap();
                let placement = model.png_placement();
                let scale = placement.module_scale().get();
                let x = placement.matrix_origin().x().get() + u32::from(dark.x()) * scale;
                let y = placement.matrix_origin().y().get() + u32::from(dark.y()) * scale;
                let offset = usize::try_from((y * width + x) * 4).unwrap();
                assert_eq!(&pixels[offset..offset + 4], &foreground.rgba().channels());
                if matches!(background, Background::Transparent) {
                    assert!(pixels.chunks_exact(4).any(|pixel| pixel == [0, 0, 0, 0]));
                    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == u8::MAX));
                }
            }
        }
    }
}

struct PngChunk<'bytes> {
    kind: [u8; 4],
    data: &'bytes [u8],
}

fn png_chunks(bytes: &[u8]) -> Vec<PngChunk<'_>> {
    assert!(bytes.starts_with(PNG_SIGNATURE));
    let mut chunks = Vec::new();
    let mut offset = PNG_SIGNATURE.len();
    while offset < bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = bytes[offset + 4..offset + 8].try_into().unwrap();
        let data_start = offset + 8;
        let data_end = data_start + length;
        chunks.push(PngChunk {
            kind,
            data: &bytes[data_start..data_end],
        });
        offset = data_end + 4;
    }
    assert_eq!(offset, bytes.len());
    chunks
}

fn decode_rgba(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let output = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(output.color_type, png::ColorType::Rgba);
    assert_eq!(output.bit_depth, png::BitDepth::Eight);
    pixels.truncate(output.buffer_size());
    (output.width, output.height, pixels)
}

fn encoded_qr_at_version(version: u8) -> EncodedQr {
    let text = "a".repeat(versions::first_byte_length(version));
    encode(EncodeRequest {
        text: &text,
        ecc: ErrorCorrection::Medium,
        max_version: Version::try_from(version).unwrap(),
    })
    .unwrap()
}
