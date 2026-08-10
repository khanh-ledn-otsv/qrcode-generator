#[path = "support/raster.rs"]
mod raster;
#[path = "support/versions.rs"]
mod versions;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FOREGROUNDS, Background, GlyphOwnership, RenderModel,
    RenderOptions, Rgba, SUPPORTED_PROFILES, Version, render_svg,
};
use sha2::{Digest, Sha256};

const APPROVED_SVG_SHA256: [[[&str; 2]; 1]; 5] = [
    [[
        "407c6be87cf2d5f7076f52e1defe7d5ec61bc6c5d94d8dc8ee318052bfceff91",
        "51ae4f353f7d19205ed8ae77d2bfb55883f7178027910576c1c74287fabcd193",
    ]],
    [[
        "e33ed38e6b67581c31d0fb8caa965493b0235292969d9d1b694fcceef34fdfb3",
        "b974038c3b010f0663de808c4191ca7ea7742a3f4ae86f7e76b45f0e3af0807d",
    ]],
    [[
        "06bdaff6013d2d62414b1d43b44645658b06bcb5437dbe15ced087deab20fb98",
        "06f94904a05951b95fa387d689a5b105c7a8f090559043e8075c558422187f38",
    ]],
    [[
        "b04dbd6103fdba8e39315d46d61a5a40185d6ad5eb1a4a4cf1861f6ef43fe2f1",
        "a6988513ad57ece85122d58ac6524d5e3b45a08ccaf0a457e816063f3cc18bb6",
    ]],
    [[
        "e2e88b1131e07558f725f639187b8e4265d8bd15512a0afd1102dace15a2aa47",
        "686a6779e3bbe233bc0ba9750bd7ab52ff353e6015e4789813edb9d6b6f3a4da",
    ]],
];

#[test]
#[ignore = "explicitly emits golden hashes for reviewed fixture refreshes"]
fn print_svg_hashes_for_fixture_refresh() {
    let payload = r#"safe/<script>alert("payload")</script>"#;
    let encoded = encoded_qr(payload);
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap(),
    )
    .unwrap();
    println!(
        "safe_svg_sha256={}",
        sha256_hex(render_svg(&model).unwrap().as_bytes())
    );

    let encoded = encoded_qr("APPROVED SVG APPEARANCE");
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
                    sha256_hex(render_svg(&model).unwrap().as_bytes())
                );
            }
        }
    }
}

#[test]
fn safe_svg_has_exact_sizing_structure_and_deterministic_bytes() {
    let payload = r#"safe/<script>alert("payload")</script>"#;
    let encoded = encoded_qr(payload);
    let profile = SUPPORTED_PROFILES[1];
    let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();

    let first = render_svg(&model).unwrap();
    let second = render_svg(&model).unwrap();
    assert_eq!(first.as_bytes(), second.as_bytes());
    assert_eq!(
        sha256_hex(first.as_bytes()),
        "97ef83dbdf211f90f1080695ce36bd71feeaedd39aa50bb49c47ce399bfbbd81"
    );
    assert!(!first.contains(payload));

    let document = roxmltree::Document::parse(&first).unwrap();
    let root = document.root_element();
    let extent = model.symbol().extent_modules().get().to_string();
    assert_eq!(root.tag_name().name(), "svg");
    assert_eq!(
        root.tag_name().namespace(),
        Some("http://www.w3.org/2000/svg")
    );
    assert_eq!(root.attribute("width"), Some("120"));
    assert_eq!(root.attribute("height"), Some("120"));
    assert_eq!(
        root.attribute("viewBox"),
        Some(format!("0 0 {extent} {extent}").as_str())
    );

    let elements = root
        .children()
        .filter(roxmltree::Node::is_element)
        .collect::<Vec<_>>();
    assert_eq!(elements.len(), 2);
    assert_eq!(elements[0].tag_name().name(), "rect");
    assert_eq!(elements[0].attribute("width"), Some(extent.as_str()));
    assert_eq!(elements[0].attribute("height"), Some(extent.as_str()));
    assert_eq!(elements[0].attribute("fill"), Some("#ffffff"));
    assert_eq!(elements[1].tag_name().name(), "path");
    assert_eq!(elements[1].attribute("fill"), Some("#bd0f72"));
    assert_eq!(elements[1].attribute("shape-rendering"), Some("crispEdges"));
    assert!(
        elements[1]
            .attribute("d")
            .is_some_and(|path| !path.is_empty())
    );

    for node in document.descendants().filter(roxmltree::Node::is_element) {
        assert!(!matches!(
            node.tag_name().name(),
            "script" | "style" | "foreignObject" | "text" | "image"
        ));
        for attribute in node.attributes() {
            assert!(!attribute.name().starts_with("on"));
            assert!(!matches!(attribute.name(), "href" | "stroke" | "style"));
            assert!(!attribute.value().contains("://"));
        }
    }
}

#[test]
fn approved_svg_color_background_profile_tuples_are_structural_and_deterministic() {
    let encoded = encoded_qr("APPROVED SVG APPEARANCE");

    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        let safe_model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
        let safe_svg = render_svg(&safe_model).unwrap();
        let safe_document = roxmltree::Document::parse(&safe_svg).unwrap();
        let safe_path = safe_document
            .descendants()
            .find(|node| node.has_tag_name("path"))
            .and_then(|node| node.attribute("d"))
            .unwrap();

        for (foreground_index, foreground) in APPROVED_FOREGROUNDS.into_iter().enumerate() {
            for (background_index, background) in APPROVED_BACKGROUNDS.into_iter().enumerate() {
                let options = RenderOptions::approved(profile, foreground, background).unwrap();
                let model = RenderModel::new(&encoded, options).unwrap();
                let first = render_svg(&model).unwrap();
                assert_eq!(
                    sha256_hex(first.as_bytes()),
                    APPROVED_SVG_SHA256[profile_index][foreground_index][background_index],
                );
                assert_eq!(first, render_svg(&model).unwrap());

                let document = roxmltree::Document::parse(&first).unwrap();
                let root = document.root_element();
                let rect = root.children().find(|node| node.has_tag_name("rect"));
                match background {
                    Background::Opaque(Rgba::WHITE) => {
                        assert_eq!(
                            rect.and_then(|node| node.attribute("fill")),
                            Some("#ffffff")
                        );
                    }
                    Background::Transparent => assert!(rect.is_none()),
                    Background::Opaque(_) => panic!("only approved backgrounds are enumerated"),
                }
                let path = root
                    .children()
                    .find(|node| node.has_tag_name("path"))
                    .unwrap();
                assert_eq!(path.attribute("d"), Some(safe_path));
                assert_eq!(path.attribute("fill"), Some("#bd0f72"));
                assert_eq!(path.attribute("shape-rendering"), Some("crispEdges"));
            }
        }
    }
}

#[test]
fn every_supported_profile_version_has_stable_in_bounds_glyph_paths() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let encoded = encoded_qr_at_version(version_number);
            let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
            let svg = render_svg(&model).unwrap();
            let document = roxmltree::Document::parse(&svg).unwrap();
            let root = document.root_element();
            let width = profile
                .svg_dimensions_for(encoded.version())
                .unwrap()
                .width()
                .get()
                .to_string();
            let extent = model.symbol().extent_modules().get();
            let quiet = model.symbol().quiet_zone_modules_per_side().get();

            assert_eq!(root.attribute("width"), Some(width.as_str()));
            assert_eq!(root.attribute("height"), Some(width.as_str()));
            assert_eq!(
                root.attribute("viewBox"),
                Some(format!("0 0 {extent} {extent}").as_str())
            );

            let path = root
                .children()
                .find(|node| node.has_tag_name("path"))
                .and_then(|node| node.attribute("d"))
                .unwrap();
            let actual = dark_module_coordinates(path);
            let expected = expected_dark_module_coordinates(&encoded, quiet);
            assert_eq!(actual, expected);
            assert!(actual.iter().all(|&(x, y)| {
                x >= quiet && y >= quiet && x < extent - quiet && y < extent - quiet
            }));
        }
    }
}

#[test]
fn independent_rasterization_preserves_background_quiet_zone_and_square_modules() {
    let encoded = encoded_qr("RASTER STRUCTURE");
    let profile = SUPPORTED_PROFILES[1];
    let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
    let svg = render_svg(&model).unwrap();
    let dimensions = profile.svg_dimensions();
    let pixmap =
        raster::rasterize_svg(&svg, dimensions.width().get(), dimensions.height().get()).unwrap();

    let white = pixmap.pixel(0, 0).unwrap();
    assert_eq!(
        (white.red(), white.green(), white.blue(), white.alpha()),
        (255, 255, 255, 255)
    );

    let extent = model.symbol().extent_modules().get();
    let finder_center_x = ((4 * 2 + 1) * dimensions.width().get()) / (extent * 2);
    let finder_center_y = ((4 * 2 + 1) * dimensions.height().get()) / (extent * 2);
    let brand = pixmap.pixel(finder_center_x, finder_center_y).unwrap();
    assert_eq!(
        (brand.red(), brand.green(), brand.blue(), brand.alpha()),
        (189, 15, 114, 255)
    );

    assert!(pixmap.pixels().iter().all(|pixel| {
        matches!(
            (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
            (255, 255, 255, 255) | (189, 15, 114, 255)
        )
    }));
}

#[test]
fn svg_uses_full_cell_finders_and_exact_centered_compact_dots() {
    let encoded = encoded_qr("COMPACT DOT SVG");
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap(),
    )
    .unwrap();
    let svg = render_svg(&model).unwrap();
    let path = roxmltree::Document::parse(&svg)
        .unwrap()
        .descendants()
        .find(|node| node.has_tag_name("path"))
        .and_then(|node| node.attribute("d"))
        .unwrap()
        .to_owned();
    let quiet = model.symbol().quiet_zone_modules_per_side().get();
    let finder = model
        .glyphs()
        .find(|glyph| glyph.ownership() == GlyphOwnership::Finder)
        .unwrap();
    let dot = model
        .glyphs()
        .find(|glyph| glyph.ownership() != GlyphOwnership::Finder)
        .unwrap();

    let finder_x = u32::from(finder.x()) + quiet;
    let finder_y = u32::from(finder.y()) + quiet;
    assert!(path.contains(format!("M{finder_x} {finder_y}h1v1h-1z").as_str()));

    let dot_left = u32::from(dot.x()) + quiet;
    let dot_top = u32::from(dot.y()) + quiet;
    assert!(
        path.contains(
            format!(
                "M{dot_left}.275 {dot_top}.500a0.225 0.225 0 1 0 0.450 0a0.225 0.225 0 1 0-0.450 0z"
            )
            .as_str()
        )
    );
    assert_eq!(
        path.matches("h1v1h-1z").count(),
        model
            .glyphs()
            .filter(|glyph| glyph.ownership() == GlyphOwnership::Finder)
            .count()
    );
}

fn encoded_qr(text: &str) -> EncodedQr {
    encode(EncodeRequest::first_fit(
        text,
        ErrorCorrection::Medium,
        Version::try_from(8).unwrap(),
    ))
    .unwrap()
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

fn dark_module_coordinates(path: &str) -> Vec<(u32, u32)> {
    path.split('M')
        .filter(|command| !command.is_empty())
        .map(|command| {
            if let Some(coordinates) = command.strip_suffix("h1v1h-1z") {
                let (x, y) = coordinates.split_once(' ').unwrap();
                (x.parse().unwrap(), y.parse().unwrap())
            } else {
                let coordinates = command
                    .strip_suffix("a0.225 0.225 0 1 0 0.450 0a0.225 0.225 0 1 0-0.450 0z")
                    .unwrap();
                let (left, center_y) = coordinates.split_once(' ').unwrap();
                (
                    left.strip_suffix(".275").unwrap().parse().unwrap(),
                    center_y.strip_suffix(".500").unwrap().parse().unwrap(),
                )
            }
        })
        .collect()
}

fn expected_dark_module_coordinates(encoded: &EncodedQr, quiet: u32) -> Vec<(u32, u32)> {
    let size = encoded.modules().size();
    let mut coordinates = Vec::new();
    for y in 0..size {
        for x in 0..size {
            if encoded
                .modules()
                .module(x, y)
                .is_some_and(|module| module.is_dark())
            {
                coordinates.push((u32::from(x) + quiet, u32::from(y) + quiet));
            }
        }
    }
    coordinates
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
