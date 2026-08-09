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
fn logo_artifacts_embed_the_source_artwork_through_a_trimmed_presentation_box() {
    let encoded = encode(EncodeRequest::first_fit(
        "logo",
        ErrorCorrection::High,
        SUPPORTED_PROFILES[0].maximum_version(),
    ))
    .unwrap();
    let options = RenderOptions::safe(SUPPORTED_PROFILES[0])
        .unwrap()
        .with_logo(LogoStyle::Bundled)
        .unwrap();
    let model = RenderModel::new(&encoded, options).unwrap();
    let svg = render_svg(&model).unwrap();
    assert_eq!(svg, render_svg(&model).unwrap());
    assert!(!svg.contains("#000000"));
    assert!(svg.contains("data-role=\"logo-knockout\""));
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.contains("data-role=\"bundled-logo\""));
    assert!(svg.contains("viewBox=\"180 180 640 240\""));
    assert!(svg.contains("preserveAspectRatio=\"xMidYMid meet\""));

    let source_document = roxmltree::Document::parse(BUNDLED_LOGO_SVG).unwrap();
    let rendered_document = roxmltree::Document::parse(&svg).unwrap();
    let rendered_logo = rendered_document
        .descendants()
        .find(|node| node.attribute("data-role") == Some("bundled-logo"))
        .unwrap();
    let source_shapes = shape_attributes(source_document.root_element());
    let rendered_shapes = shape_attributes(rendered_logo);
    assert_eq!(rendered_shapes, source_shapes);

    let placement = model.logo_placement().unwrap();
    let knockout = placement.knockout_bounds();
    let matrix_width = u32::from(encoded.version().symbol_size());
    assert!(knockout.left().get() + knockout.width().get() <= matrix_width);
    assert!(knockout.top().get() + knockout.height().get() <= matrix_width);

    let png = render_png(&model).unwrap();
    assert_eq!(png, render_png(&model).unwrap());
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
    assert!(pixels.chunks_exact(4).any(|pixel| {
        pixel[3] == 255 && pixel != [255, 255, 255, 255] && pixel != [189, 15, 114, 255]
    }));
    let row_bytes = usize::try_from(output.width).unwrap() * 4;
    assert!(
        pixels[..row_bytes]
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255])
    );
    assert!(
        pixels[pixels.len() - row_bytes..]
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255])
    );
}

fn shape_attributes(root: roxmltree::Node<'_, '_>) -> Vec<(String, String, String)> {
    root.descendants()
        .filter_map(|node| match node.tag_name().name() {
            "path" => Some((
                "path".to_owned(),
                node.attribute("fill")?.to_owned(),
                node.attribute("d")?.to_owned(),
            )),
            "polygon" => Some((
                "polygon".to_owned(),
                node.attribute("fill")?.to_owned(),
                node.attribute("points")?.to_owned(),
            )),
            _ => None,
        })
        .collect()
}
