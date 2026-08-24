#[path = "support/png_fixture.rs"]
mod png_fixture;
#[path = "support/versions.rs"]
mod versions;

use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use qr_render::{GlyphOwnership, RenderModel, RenderOptions, Rgba, SUPPORTED_PROFILES, render_png};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const APPROVED_PNG_SHA256: [&str; 7] = [
    "48e5185bcdd5a796e9c27bd0cb514ad670d57f1623c15e9db2e00ee15af55c32",
    "a46afd15bc93b51a6d078bfb8d7816a80e868f9ca5ca56a86589b750a12c30f9",
    "d43efaad97e5422da89785f9a1806056796d053ae8844b834657e025231cab34",
    "5dd4104ba8b75d8bcf4ffb8ee36d6a5cf7409c9df2e089503cf9e1fc683c7ed6",
    "f41be9f0e417373a47695048d9602e04670b25435aab54433e3fd09819939d68",
    "0e7b0f2d1920804acd769e2bd635a522e21d8f3e36a0e030a552084b2a79bf81",
    "af49eba629ca970f04f865c01095efcaec5556e2ac8a3e57805aabdf699a3087",
];

#[test]
#[ignore = "explicitly emits golden hashes for reviewed fixture refreshes"]
fn print_png_hashes_for_fixture_refresh() {
    println!(
        "safe_png_sha256={}",
        png_fixture::sha256_hex(&png_fixture::artifact())
    );
    let encoded = encoded_qr_at_version(1);
    for profile in SUPPORTED_PROFILES {
        let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
        println!(
            "{:?} {}",
            profile.id(),
            png_fixture::sha256_hex(&render_png(&model).unwrap())
        );
    }
}

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
fn rounded_one_png_uses_opaque_antialiased_dots_outside_finders() {
    let encoded = encoded_qr_at_version(1);
    let options = RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap();
    let model = RenderModel::new(&encoded, options).unwrap();
    let png = render_png(&model).unwrap();
    let (width, _, pixels) = decode_rgba(&png);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == u8::MAX)
    );
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| { *pixel != Rgba::WHITE.channels() && *pixel != Rgba::BRAND.channels() })
    );

    let placement = model.png_placement();
    let origin = placement.matrix_origin();
    let scale = placement.module_scale().get();
    let finder = model
        .glyphs()
        .find(|glyph| glyph.ownership() == GlyphOwnership::Finder)
        .unwrap();
    let finder_left = origin.x().get() + u32::from(finder.x()) * scale;
    let finder_top = origin.y().get() + u32::from(finder.y()) * scale;
    for y in finder_top..finder_top + scale {
        for x in finder_left..finder_left + scale {
            assert_eq!(pixel(&pixels, width, x, y), Rgba::BRAND.channels());
        }
    }

    let rounded = model
        .glyphs()
        .find(|glyph| glyph.ownership() != GlyphOwnership::Finder)
        .unwrap();
    let rounded_left = origin.x().get() + u32::from(rounded.x()) * scale;
    let rounded_top = origin.y().get() + u32::from(rounded.y()) * scale;
    assert_eq!(
        pixel(&pixels, width, rounded_left, rounded_top),
        Rgba::WHITE.channels()
    );
    for offset_y in [scale / 2 - 1, scale / 2] {
        for offset_x in [scale / 2 - 1, scale / 2] {
            assert_eq!(
                pixel(
                    &pixels,
                    width,
                    rounded_left + offset_x,
                    rounded_top + offset_y,
                ),
                Rgba::BRAND.channels(),
                "the approved circle retains a solid brand-color core"
            );
        }
    }
}

#[test]
fn opaque_rounded_profile_artifacts_are_structural_and_deterministic() {
    let encoded = encoded_qr_at_version(1);

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
        let first = render_png(&model).unwrap();
        assert_eq!(
            png_fixture::sha256_hex(&first),
            APPROVED_PNG_SHA256[profile_index]
        );
        assert_eq!(first, render_png(&model).unwrap());
        let (_, _, pixels) = decode_rgba(&first);
        assert_eq!(&pixels[0..4], &Rgba::WHITE.channels());
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
    encode(EncodeRequest::first_fit(
        &text,
        ErrorCorrection::Medium,
        Version::try_from(version).unwrap(),
    ))
    .unwrap()
}
