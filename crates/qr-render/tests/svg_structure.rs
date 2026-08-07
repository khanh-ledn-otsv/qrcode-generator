#[path = "support/raster.rs"]
mod raster;
#[path = "support/versions.rs"]
mod versions;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FOREGROUNDS, Background, RenderModel, RenderOptions, Rgba,
    SUPPORTED_PROFILES, Version, render_svg,
};
use sha2::{Digest, Sha256};

const APPROVED_SVG_SHA256: [[[&str; 2]; 2]; 4] = [
    [
        [
            "849d1aef21b475dc0a11456ba549ef943cc79dc7a56cc4965a6b2e4a14703f54",
            "23be314543a9de2efa53314dd1854ebef6c76a30ec81cdf37f2c100596ff13be",
        ],
        [
            "c256fe4504c1e8405aad99cda54baffd98a7252436ef5718ddd6554157b0fa66",
            "58fb401c4209df67ebbfbcca1f83bec2a6d8573df7b93861a7c0a480e4a0f47a",
        ],
    ],
    [
        [
            "6f2a728b61fb1d7509e0bc0cea252e54d81c42227b6844899ff8c29c99cb95f6",
            "b3cd70d017677efaf6ec31ca2f3cc258eb0dc9435a08e088d0c1714bbee83b59",
        ],
        [
            "48e1d78364fee0610e62ffc16e8a29c6115bb792e9ad38cff3cac500c055646f",
            "20ca1a1f6aaefda98549b7eefdaf0dba8cc889b17d153a1edf1afdbc690dec52",
        ],
    ],
    [
        [
            "44e070b6bbf4ff0e826d38a95f1f55d68689248e02987fcac608f5acab7a6656",
            "1814fb0fdd7a431381a897021a5e87bf364f79e16f784098ed61d566197c9d9e",
        ],
        [
            "d84b1a3e4390f036419ac3cea90179f96e88ea21bd8c849685235a9e503d7bee",
            "4eec3097d53b6ef7a1b41c07892124db842502e9435646dc53a371cc42a76caf",
        ],
    ],
    [
        [
            "0909ca246ec8411d4de15ed1dc159a1fc4b656ba3535e80348f8e1cb97943d42",
            "e1b969668bc4ecebf8b485de57c8eb7782375c043034635b7dc950ed65cd6f97",
        ],
        [
            "a85c6114cb3886c0bd0e7c60db19072ef06d78ffa28a87909f6354ff10c9dfab",
            "b2992cabc1cf14dbdff3bebdfd9dfb970df4f2acaa1ef83ccdda575a13b74a64",
        ],
    ],
];

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
                let expected_foreground = if foreground.rgba() == Rgba::BLACK {
                    "#000000"
                } else {
                    "#bd0f72"
                };
                assert_eq!(path.attribute("fill"), Some(expected_foreground));
            }
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
