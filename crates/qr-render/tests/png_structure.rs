#[path = "support/png_fixture.rs"]
mod png_fixture;
#[path = "support/versions.rs"]
mod versions;

use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, Version, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FOREGROUNDS, Background, Foreground, GlyphOwnership,
    RenderModel, RenderOptions, Rgba, SUPPORTED_PROFILES, render_png,
};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const APPROVED_PNG_SHA256: [[[&str; 2]; 1]; 5] = [
    [[
        "8b5fb2d1b15846e8a042b8e8760a007f5bbb0daafc9a52efe4b030a9d373bf83",
        "5e39014eea2a4a482221360129bfcb7f41f759fb0672fe24ada661d45df2787e",
    ]],
    [[
        "d54f8f4e9f1a611fe9d01ed834a0f2689db792a25dff858d80bf272db5b55dde",
        "536efc23ea5add3528cb153355c2e3ceca796a9bdd95d63a4d223e1b0c4df1d1",
    ]],
    [[
        "7d2d2509ce3f2c137e6b1c101e6a1f40c463a629224758ee4495a030bf8d6534",
        "0f94b5d4c0d3479c6d28f3ed4cbdcd70029555acafa4b1a04e2b78c335dc7a4f",
    ]],
    [[
        "096241af7fe54c60049ed8f5f3129cd8edbb4be8c6acfb8b4fe60a8eb3b142b5",
        "12d75ecd3e45ab7d611403068a71f6131eaf78df775ed7cc8d131afaa6fb9767",
    ]],
    [[
        "b2449d3047a832ccad08721669878f91cd3a635181bfaabfa0fb53a36a957f68",
        "73c4a5c3f01b6b8e7f6bfb6c376f9f272a10e32e8dd78d332430e28c4b56cde1",
    ]],
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
        for foreground in APPROVED_FOREGROUNDS {
            for background in APPROVED_BACKGROUNDS {
                let model = RenderModel::new(
                    &encoded,
                    RenderOptions::approved(profile, foreground, background).unwrap(),
                )
                .unwrap();
                println!(
                    "{:?} {:?} {:?} {}",
                    profile.id(),
                    foreground,
                    background,
                    png_fixture::sha256_hex(&render_png(&model).unwrap())
                );
            }
        }
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
fn compact_dot_pixels_keep_a_solid_brand_core_and_round_contour() {
    let encoded = encoded_qr_at_version(1);
    let profile = SUPPORTED_PROFILES[1];

    for background in APPROVED_BACKGROUNDS {
        let model = RenderModel::new(
            &encoded,
            RenderOptions::approved(profile, Foreground::Brand, background).unwrap(),
        )
        .unwrap();
        let png = render_png(&model).unwrap();
        let (width, _, pixels) = decode_rgba(&png);
        let dot = model
            .glyphs()
            .find(|glyph| glyph.ownership() != GlyphOwnership::Finder)
            .unwrap();
        let placement = model.png_placement();
        let scale = placement.module_scale().get();
        let left = placement.matrix_origin().x().get() + u32::from(dot.x()) * scale;
        let top = placement.matrix_origin().y().get() + u32::from(dot.y()) * scale;
        let background_pixel = background_pixel(background);
        let brand = Rgba::BRAND.channels();
        let mut contour_pixels = 0;

        for y in 0..scale {
            for x in 0..scale {
                let actual = pixel(&pixels, width, left + x, top + y);
                if actual != background_pixel && actual != brand {
                    contour_pixels += 1;
                }
                assert_eq!(
                    actual,
                    pixel(&pixels, width, left + (scale - 1 - x), top + y),
                    "compact-dot coverage must remain horizontally symmetric"
                );
                assert_eq!(
                    actual,
                    pixel(&pixels, width, left + x, top + (scale - 1 - y)),
                    "compact-dot coverage must remain vertically symmetric"
                );
            }
        }
        assert!(contour_pixels > 0, "round dots need an antialiased contour");
        assert_eq!(pixel(&pixels, width, left, top), background_pixel);
        for y in scale / 2 - 1..=scale / 2 {
            for x in scale / 2 - 1..=scale / 2 {
                assert_eq!(
                    pixel(&pixels, width, left + x, top + y),
                    brand,
                    "the dot interior must use the exact finder/logo brand color"
                );
            }
        }
    }
}

#[test]
fn every_profile_version_matches_the_independent_dot_coverage_reference() {
    for profile in SUPPORTED_PROFILES {
        for version in 1..=profile.maximum_version().number() {
            let encoded = encoded_qr_at_version(version);
            for background in APPROVED_BACKGROUNDS {
                let model = RenderModel::new(
                    &encoded,
                    RenderOptions::approved(profile, Foreground::Brand, background).unwrap(),
                )
                .unwrap();
                let png = render_png(&model).unwrap();
                let (width, height, pixels) = decode_rgba(&png);

                let dimensions = profile.png_dimensions_for(encoded.version()).unwrap();
                assert_eq!(width, dimensions.width().get());
                assert_eq!(height, dimensions.height().get());
                assert_eq!(pixels.len(), model.png_placement().rgba_buffer_len());

                let placement = model.png_placement();
                let origin_x = placement.matrix_origin().x().get();
                let origin_y = placement.matrix_origin().y().get();
                let scale = placement.module_scale().get();
                let matrix_size = u32::from(model.matrix().size());
                let background_pixel = background_pixel(background);

                assert_eq!(pixel(&pixels, width, 0, 0), background_pixel);
                assert_eq!(
                    pixel(
                        &pixels,
                        width,
                        origin_x + matrix_size * scale,
                        origin_y + matrix_size * scale,
                    ),
                    background_pixel
                );

                let finder = model
                    .glyphs()
                    .find(|glyph| glyph.ownership() == GlyphOwnership::Finder)
                    .unwrap();
                assert_glyph_cell_matches(&pixels, width, &model, finder, |_, _| {
                    Rgba::BRAND.channels()
                });

                let dot = model
                    .glyphs()
                    .find(|glyph| glyph.ownership() != GlyphOwnership::Finder)
                    .unwrap();
                assert_glyph_cell_matches(&pixels, width, &model, dot, |x, y| {
                    reference_dot_pixel(scale, x, y, background)
                });

                let separator = model
                    .cells()
                    .find(|cell| cell.ownership() == GlyphOwnership::Separator)
                    .unwrap();
                let separator_x = origin_x + u32::from(separator.x()) * scale;
                let separator_y = origin_y + u32::from(separator.y()) * scale;
                for y in 0..scale {
                    for x in 0..scale {
                        assert_eq!(
                            pixel(&pixels, width, separator_x + x, separator_y + y),
                            background_pixel
                        );
                    }
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

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = usize::try_from((y * width + x) * 4).unwrap();
    pixels[offset..offset + 4].try_into().unwrap()
}

fn assert_glyph_cell_matches(
    pixels: &[u8],
    width: u32,
    model: &RenderModel<'_>,
    glyph: qr_render::SymbolGlyph,
    expected: impl Fn(u32, u32) -> [u8; 4],
) {
    let placement = model.png_placement();
    let scale = placement.module_scale().get();
    let left = placement.matrix_origin().x().get() + u32::from(glyph.x()) * scale;
    let top = placement.matrix_origin().y().get() + u32::from(glyph.y()) * scale;
    for y in 0..scale {
        for x in 0..scale {
            assert_eq!(pixel(pixels, width, left + x, top + y), expected(x, y));
        }
    }
}

fn reference_dot_pixel(scale: u32, x: u32, y: u32, background: Background) -> [u8; 4] {
    const SAMPLES_PER_AXIS: u32 = 8;
    const SAMPLE_COUNT: u32 = SAMPLES_PER_AXIS * SAMPLES_PER_AXIS;
    let center = f64::from(scale) / 2.0;
    let radius = f64::from(scale) * 0.75 / 2.0;
    let mut covered = 0;
    for sample_y in 0..SAMPLES_PER_AXIS {
        for sample_x in 0..SAMPLES_PER_AXIS {
            let sample_x = f64::from(x) + (f64::from(sample_x) + 0.5) / f64::from(SAMPLES_PER_AXIS);
            let sample_y = f64::from(y) + (f64::from(sample_y) + 0.5) / f64::from(SAMPLES_PER_AXIS);
            if (sample_x - center).powi(2) + (sample_y - center).powi(2) <= radius.powi(2) {
                covered += 1;
            }
        }
    }
    if covered == 0 {
        return background_pixel(background);
    }

    let foreground = Rgba::BRAND.channels();
    match background {
        Background::Transparent => [
            foreground[0],
            foreground[1],
            foreground[2],
            reference_blend(u8::MAX, 0, covered, SAMPLE_COUNT),
        ],
        Background::Opaque(background) => {
            let background = background.channels();
            [
                reference_blend(foreground[0], background[0], covered, SAMPLE_COUNT),
                reference_blend(foreground[1], background[1], covered, SAMPLE_COUNT),
                reference_blend(foreground[2], background[2], covered, SAMPLE_COUNT),
                u8::MAX,
            ]
        }
    }
}

fn reference_blend(foreground: u8, background: u8, covered: u32, samples: u32) -> u8 {
    let blended =
        u32::from(foreground) * covered + u32::from(background) * (samples - covered) + samples / 2;
    u8::try_from(blended / samples).unwrap()
}

fn background_pixel(background: Background) -> [u8; 4] {
    match background {
        Background::Opaque(color) => color.channels(),
        Background::Transparent => [0, 0, 0, 0],
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
    encode(EncodeRequest::first_fit(
        &text,
        ErrorCorrection::Medium,
        Version::try_from(version).unwrap(),
    ))
    .unwrap()
}
