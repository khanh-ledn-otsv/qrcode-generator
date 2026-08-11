#[path = "support/raster.rs"]
mod raster;
#[path = "support/versions.rs"]
mod versions;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{RenderModel, RenderOptions, SUPPORTED_PROFILES, Version, render_svg};
use sha2::{Digest, Sha256};

const APPROVED_SVG_SHA256: [&str; 5] = [
    "252464e4aebd927f1b49d9fed491a4f166d5c54ddec0059f9bef249f08f7da6e",
    "34f1337c7cff58418f524c4dc148f491d5e9b684a9da235ef71f9596d27c88d7",
    "667ab7321ce239dc46a0f482d8929222e171841bd8de42b75de5856737b364ba",
    "a4b8a18eee28ae6654608e777052a3ddf552d76efd1be1f0e11be39ce3305af6",
    "91db53628c8dd731982393a521f8e3839614f9ec75f203126ba466c6100c8413",
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
        let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
        println!(
            "{:?} {}",
            profile.id(),
            sha256_hex(render_svg(&model).unwrap().as_bytes())
        );
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
        "fc4eb0af5a5143d3636f8f9b7a3f27998afc1b1156b274479ec61c16d4a25b38"
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
    assert_eq!(elements[1].attribute("shape-rendering"), None);
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
fn rounded_one_svg_retains_compact_dots_and_square_finders() {
    let encoded = encoded_qr("ROUNDED ONE MODULES");
    let options = RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap();
    let svg = render_svg(&RenderModel::new(&encoded, options).unwrap()).unwrap();
    assert!(svg.contains("a0.450 0.450"));
    assert!(svg.contains("M4 4h1v1h-1z"));
    assert!(svg.contains("fill=\"#ffffff\""));
}

#[test]
fn opaque_rounded_profile_artifacts_are_structural_and_deterministic() {
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

        let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
        let first = render_svg(&model).unwrap();
        assert_eq!(
            sha256_hex(first.as_bytes()),
            APPROVED_SVG_SHA256[profile_index]
        );
        assert_eq!(first, render_svg(&model).unwrap());

        let document = roxmltree::Document::parse(&first).unwrap();
        let root = document.root_element();
        let rect = root.children().find(|node| node.has_tag_name("rect"));
        assert_eq!(
            rect.and_then(|node| node.attribute("fill")),
            Some("#ffffff")
        );
        let path = root
            .children()
            .find(|node| node.has_tag_name("path"))
            .unwrap();
        assert_eq!(path.attribute("d"), Some(safe_path));
        assert_eq!(path.attribute("fill"), Some("#bd0f72"));
        assert_eq!(path.attribute("shape-rendering"), None);
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

    assert!(pixmap.pixels().iter().any(|pixel| {
        pixel.alpha() == 255
            && pixel.red() > 189
            && pixel.red() < 255
            && pixel.green() > 15
            && pixel.green() < 255
            && pixel.blue() > 114
            && pixel.blue() < 255
    }));
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
                    .strip_suffix("a0.450 0.450 0 1 0 0.900 0a0.450 0.450 0 1 0-0.900 0z")
                    .unwrap();
                let (left, center_y) = coordinates.split_once(' ').unwrap();
                (
                    left.strip_suffix(".050").unwrap().parse().unwrap(),
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
