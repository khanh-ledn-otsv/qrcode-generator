use std::error::Error;

pub fn rasterize_svg(
    svg: &str,
    width: u32,
    height: u32,
) -> Result<resvg::tiny_skia::Pixmap, Box<dyn Error>> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())?;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("could not allocate SVG pixmap")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}
