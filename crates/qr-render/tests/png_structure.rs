#[path = "support/png_fixture.rs"]
mod png_fixture;
#[path = "support/versions.rs"]
mod versions;

use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FOREGROUNDS, Background, GlyphOwnership, RenderModel,
    RenderOptions, SUPPORTED_PROFILES, render_png,
};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const APPROVED_PNG_SHA256: [[[&str; 2]; 1]; 4] = [
    [[
        "b05e2f7a1a5e7c8a3dcbbc6e7d5759a8007a6ce515cfff97e13a7a70ea6963e8",
        "11869c3755e6a80e2d08243b2938f6e720250ba4585f9fadd2fd2db1e1ddc1d7",
    ]],
    [[
        "c49d0359bae6f0c3c209e1d27124bdf1413f71468db98d382ae86385c6fde703",
        "184a95ff139de35bc853138d8a7a772d1e25a25d263724ab00846c87884222c8",
    ]],
    [[
        "e464d40e46ef154fa20a830dffae00980454b14525eeb9bf4176c853e899410c",
        "0f984f28462febe76332c4df24f15a19155858b207241704597ddbc093b65f6a",
    ]],
    [[
        "497bb7bfd94d7c4719eeae748ecdfcc78b769fcf216b5f9208b686c3b720647c",
        "596449c63573908606467378aa3d82b40fa472768365cb3f1c971fbbc3b022bc",
    ]],
];

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
fn decoded_pixels_keep_quiet_space_blank_and_glyphs_inside_their_cells() {
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

            assert_eq!(pixel(&pixels, width, 0, 0), [255; 4]);
            assert_eq!(
                pixel(
                    &pixels,
                    width,
                    origin_x + matrix_size * scale,
                    origin_y + matrix_size * scale,
                ),
                [255; 4]
            );
            for cell in model.cells() {
                let x = origin_x + u32::from(cell.x()) * scale;
                let y = origin_y + u32::from(cell.y()) * scale;
                let expected_corner = if cell
                    .glyph()
                    .is_some_and(|glyph| glyph.ownership() == GlyphOwnership::Finder)
                {
                    [189, 15, 114, 255]
                } else {
                    [255; 4]
                };
                assert_eq!(pixel(&pixels, width, x, y), expected_corner);
                if cell.glyph().is_some() {
                    assert_eq!(
                        pixel(&pixels, width, x + scale / 2 - 1, y + scale / 2 - 1),
                        [189, 15, 114, 255]
                    );
                }
            }
        }
    }
}

#[test]
fn approved_png_color_background_profile_tuples_are_structural_and_deterministic() {
    let encoded = encoded_qr_at_version(1);

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        for (foreground_index, foreground) in APPROVED_FOREGROUNDS.into_iter().enumerate() {
            for (background_index, background) in APPROVED_BACKGROUNDS.into_iter().enumerate() {
                let options = RenderOptions::approved(profile, foreground, background).unwrap();
                let model = RenderModel::new(&encoded, options).unwrap();
                let first = render_png(&model).unwrap();
                assert_eq!(
                    png_fixture::sha256_hex(&first),
                    APPROVED_PNG_SHA256[profile_index][foreground_index][background_index],
                );
                assert_eq!(first, render_png(&model).unwrap());

                let (width, _, pixels) = decode_rgba(&first);
                let background_pixel = match background {
                    Background::Opaque(color) => color.channels(),
                    Background::Transparent => [0, 0, 0, 0],
                };
                assert_eq!(&pixels[0..4], &background_pixel);

                let dark = model
                    .glyphs()
                    .find(|glyph| glyph.ownership() == GlyphOwnership::Finder)
                    .unwrap();
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

#[test]
fn png_uses_solid_finders_and_antialiased_compact_dots_within_their_envelope() {
    let encoded = encoded_qr_at_version(1);
    let profile = SUPPORTED_PROFILES[0];

    for background in [
        Background::Opaque(qr_render::Rgba::WHITE),
        Background::Transparent,
    ] {
        let model = RenderModel::new(
            &encoded,
            RenderOptions::approved(profile, qr_render::Foreground::Brand, background).unwrap(),
        )
        .unwrap();
        let (width, _, pixels) = decode_rgba(&render_png(&model).unwrap());
        let placement = model.png_placement();
        let scale = placement.module_scale().get();
        assert_eq!(scale, 8);

        let finder = model
            .glyphs()
            .find(|glyph| glyph.ownership() == GlyphOwnership::Finder)
            .unwrap();
        let finder_x = placement.matrix_origin().x().get() + u32::from(finder.x()) * scale;
        let finder_y = placement.matrix_origin().y().get() + u32::from(finder.y()) * scale;
        for y in finder_y..finder_y + scale {
            for x in finder_x..finder_x + scale {
                assert_eq!(pixel(&pixels, width, x, y), [189, 15, 114, 255]);
            }
        }

        let dot = model
            .glyphs()
            .find(|glyph| glyph.ownership() != GlyphOwnership::Finder)
            .unwrap();
        let dot_x = placement.matrix_origin().x().get() + u32::from(dot.x()) * scale;
        let dot_y = placement.matrix_origin().y().get() + u32::from(dot.y()) * scale;
        let expected_background = match background {
            Background::Opaque(color) => color.channels(),
            Background::Transparent => [0, 0, 0, 0],
        };
        for offset_y in 0..scale {
            for offset_x in 0..scale {
                let actual = pixel(&pixels, width, dot_x + offset_x, dot_y + offset_y);
                if !(2..=5).contains(&offset_x) || !(2..=5).contains(&offset_y) {
                    assert_eq!(actual, expected_background);
                }
            }
        }
        let mut dot_pixels = Vec::new();
        for offset_y in 2..=5 {
            for offset_x in 2..=5 {
                dot_pixels.push(pixel(&pixels, width, dot_x + offset_x, dot_y + offset_y));
            }
        }
        assert!(dot_pixels.contains(&[189, 15, 114, 255]));
        match background {
            Background::Opaque(_) => assert!(dot_pixels.iter().any(|pixel| {
                pixel[3] == 255 && *pixel != [189, 15, 114, 255] && *pixel != [255; 4]
            })),
            Background::Transparent => assert!(
                dot_pixels
                    .iter()
                    .any(|pixel| { pixel[..3] == [189, 15, 114] && (1..255).contains(&pixel[3]) })
            ),
        }
    }
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = usize::try_from((y * width + x) * 4).unwrap();
    pixels[offset..offset + 4].try_into().unwrap()
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
