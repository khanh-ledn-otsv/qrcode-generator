#[path = "support/raster.rs"]
mod raster;
#[path = "support/versions.rs"]
mod versions;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, Version, render_svg};
use sha2::{Digest, Sha256};

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
        "271ca0e86f33cfd9c8febdd031447ba5c9088947d5aa94f65f4de064019b8080"
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
    assert_eq!(elements[1].attribute("fill"), Some("#000000"));
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
fn every_supported_profile_version_has_stable_in_bounds_module_paths() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let encoded = encoded_qr_at_version(version_number);
            let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
            let svg = render_svg(&model).unwrap();
            let document = roxmltree::Document::parse(&svg).unwrap();
            let root = document.root_element();
            let width = profile.svg_dimensions().width().get().to_string();
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
    let black = pixmap.pixel(finder_center_x, finder_center_y).unwrap();
    assert_eq!(
        (black.red(), black.green(), black.blue(), black.alpha()),
        (0, 0, 0, 255)
    );
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

fn dark_module_coordinates(path: &str) -> Vec<(u32, u32)> {
    path.split('M')
        .filter(|command| !command.is_empty())
        .map(|command| {
            let coordinates = command.strip_suffix("h1v1h-1z").unwrap();
            let (x, y) = coordinates.split_once(' ').unwrap();
            (x.parse().unwrap(), y.parse().unwrap())
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
