#[path = "support/png_fixture.rs"]
mod png_fixture;
#[path = "support/versions.rs"]
mod versions;

use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use qr_render::{GlyphOwnership, RenderModel, RenderOptions, Rgba, SUPPORTED_PROFILES, render_png};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const APPROVED_PNG_SHA256: [&str; 5] = [
    "8b5fb2d1b15846e8a042b8e8760a007f5bbb0daafc9a52efe4b030a9d373bf83",
    "d54f8f4e9f1a611fe9d01ed834a0f2689db792a25dff858d80bf272db5b55dde",
    "7d2d2509ce3f2c137e6b1c101e6a1f40c463a629224758ee4495a030bf8d6534",
    "096241af7fe54c60049ed8f5f3129cd8edbb4be8c6acfb8b4fe60a8eb3b142b5",
    "b2449d3047a832ccad08721669878f91cd3a635181bfaabfa0fb53a36a957f68",
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
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == u8::MAX));
    assert!(
        pixels
            .chunks_exact(4)
            .any(|pixel| { pixel != Rgba::WHITE.channels() && pixel != Rgba::BRAND.channels() })
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
