use std::error::Error;

pub fn rasterize_svg(
    svg: &str,
    width: u32,
    height: u32,
) -> Result<resvg::tiny_skia::Pixmap, Box<dyn Error>> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())?;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("could not allocate SVG pixmap")?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    let scale_x = width as f32 / tree.size().width();
    let scale_y = height as f32 / tree.size().height();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}
