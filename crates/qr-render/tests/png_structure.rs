#[path = "support/versions.rs"]
mod versions;

use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png};
use sha2::{Digest, Sha256};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[test]
fn safe_png_has_fixed_structure_and_deterministic_bytes() {
    let payload = r#"safe/<script>alert("payload")</script>"#;
    let encoded = encoded_qr(payload);
    let profile = SUPPORTED_PROFILES[1];
    let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();

    let first = render_png(&model).unwrap();
    let second = render_png(&model).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        sha256_hex(&first),
        "139610a415ccf86ad47d932318abd86ec7d7dbbffe267df8a12f2001b2ef505d"
    );
    assert!(first.starts_with(PNG_SIGNATURE));
    assert!(
        !first
            .windows(payload.len())
            .any(|window| window == payload.as_bytes())
    );

    let chunks = png_chunks(&first);
    assert_eq!(
        chunks.iter().map(|chunk| chunk.kind).collect::<Vec<_>>(),
        vec![*b"IHDR", *b"IDAT", *b"IEND"]
    );

    let ihdr = chunks[0].data;
    let dimensions = profile.png_dimensions();
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

fn encoded_qr(text: &str) -> EncodedQr {
    encode(EncodeRequest {
        text,
        ecc: ErrorCorrection::Medium,
        max_version: Version::try_from(8).unwrap(),
    })
    .unwrap()
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

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
