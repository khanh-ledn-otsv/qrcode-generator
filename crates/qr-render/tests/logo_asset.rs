use std::io::Cursor;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    BUNDLED_LOGO_SVG, LogoStyle, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png,
    render_svg,
};

#[test]
fn bundled_logo_is_the_sanitized_magenta_one_lettermark() {
    let document = roxmltree::Document::parse(BUNDLED_LOGO_SVG).unwrap();
    let root = document.root_element();
    assert_eq!(root.tag_name().name(), "svg");
    assert_eq!(root.attribute("viewBox"), Some("0 0 1000 602"));

    for node in document.descendants().filter(roxmltree::Node::is_element) {
        assert!(!matches!(
            node.tag_name().name(),
            "script" | "style" | "image" | "text"
        ));
        for attribute in node.attributes() {
            assert!(!attribute.name().starts_with("on"));
            assert!(!attribute.value().contains("url("));
            assert!(!attribute.value().contains("href"));
            if attribute.name() == "fill" {
                assert_eq!(attribute.value(), "#bd0f72");
            }
        }
    }
    assert_eq!(
        document
            .descendants()
            .filter(|node| node.has_tag_name("path"))
            .count(),
        1
    );
    assert_eq!(
        document
            .descendants()
            .filter(|node| node.has_tag_name("polygon"))
            .count(),
        2
    );
}

#[test]
fn logo_artifacts_embed_the_complete_source_box_over_an_opaque_white_knockout() {
    let encoded = encode(EncodeRequest {
        text: "logo",
        ecc: ErrorCorrection::High,
        max_version: SUPPORTED_PROFILES[0].maximum_version(),
    })
    .unwrap();
    let options = RenderOptions::safe(SUPPORTED_PROFILES[0])
        .unwrap()
        .with_logo(LogoStyle::Bundled)
        .unwrap();
    let model = RenderModel::new(&encoded, options).unwrap();
    let svg = render_svg(&model).unwrap();
    assert!(!svg.contains("#000000"));
    assert!(svg.contains("data-role=\"logo-knockout\""));
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.contains("data-role=\"bundled-logo\""));
    assert!(svg.contains("viewBox=\"0 0 1000 602\""));
    assert!(svg.contains("preserveAspectRatio=\"xMidYMid meet\""));

    let png = render_png(&model).unwrap();
    let decoder = png::Decoder::new(Cursor::new(png));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let output = reader.next_frame(&mut pixels).unwrap();
    pixels.truncate(output.buffer_size());
    assert!(!pixels.chunks_exact(4).any(|pixel| pixel == [0, 0, 0, 255]));
    assert!(
        pixels
            .chunks_exact(4)
            .any(|pixel| pixel == [189, 15, 114, 255])
    );
}
