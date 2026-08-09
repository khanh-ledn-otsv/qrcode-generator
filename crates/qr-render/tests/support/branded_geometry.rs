use std::error::Error;
use std::fmt::Write;

use png::{BitDepth, ColorType, Compression, Encoder, Filter};
use qr_core::EncodedQr;
use qr_core::matrix::{ModuleKind, ModuleMatrix};
use qr_render::{OutputProfile, PixelDimensions, Rgba};

const QUIET_ZONE: u32 = 4;
const UNITS: u32 = 1_000;
const SAMPLES: u32 = 8;
const LOGO_BODY: &str = include_str!("../../../../assets/RGB-one-lettermark-magenta.svg");

#[derive(Clone, Copy)]
pub struct CandidateAppearance {
    pub dot_diameter_thousandths: u16,
    pub function_treatment: FunctionTreatment,
    pub logo: Option<LogoCandidate>,
}

#[derive(Clone, Copy)]
pub enum FunctionTreatment {
    SquareFunctions,
    NonFinderDots,
}

impl FunctionTreatment {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SquareFunctions => "square-functions",
            Self::NonFinderDots => "non-finder-dots",
        }
    }
}

#[derive(Clone, Copy)]
pub struct LogoCandidate {
    source_left: u32,
    source_top: u32,
    source_width: u32,
    source_height: u32,
    knockout: [u32; 4],
    clearance: u32,
    obscured_data: u32,
    obscured_remainder: u32,
}

impl LogoCandidate {
    pub fn checked(matrix: &ModuleMatrix, width_modules: u32) -> Option<Self> {
        let matrix_width = u32::from(matrix.size());
        let source_width = width_modules.checked_mul(UNITS)?;
        let source_height = source_width.checked_mul(3)?.checked_div(8)?;
        let source_left = matrix_width
            .checked_mul(UNITS)?
            .checked_sub(source_width)?
            .checked_div(2)?;
        let source_top = matrix_width
            .checked_mul(UNITS)?
            .checked_sub(source_height)?
            .checked_div(2)?;
        let left = source_left.checked_sub(UNITS)? / UNITS;
        let top = source_top.checked_sub(UNITS)? / UNITS;
        let right = div_ceil(
            source_left.checked_add(source_width)?.checked_add(UNITS)?,
            UNITS,
        );
        let bottom = div_ceil(
            source_top.checked_add(source_height)?.checked_add(UNITS)?,
            UNITS,
        );
        let knockout = [left, top, right - left, bottom - top];
        if knockout[2].checked_mul(5)? > matrix_width.checked_mul(2)?
            || knockout[3].checked_mul(5)? > matrix_width.checked_mul(2)?
        {
            return None;
        }

        let mut obscured_data = 0;
        let mut obscured_remainder = 0;
        for y in top..bottom {
            for x in left..right {
                match matrix
                    .module(u16::try_from(x).ok()?, u16::try_from(y).ok()?)?
                    .kind()
                {
                    ModuleKind::Data => obscured_data += 1,
                    ModuleKind::Remainder => obscured_remainder += 1,
                    _ => return None,
                }
            }
        }
        let mut clearance = u32::MAX;
        for y in 0..matrix_width {
            for x in 0..matrix_width {
                let module = matrix.module(u16::try_from(x).ok()?, u16::try_from(y).ok()?)?;
                if matches!(module.kind(), ModuleKind::Data | ModuleKind::Remainder) {
                    continue;
                }
                clearance = clearance.min(axis_gap(x, left, right - left).max(axis_gap(
                    y,
                    top,
                    bottom - top,
                )));
            }
        }
        Some(Self {
            source_left,
            source_top,
            source_width,
            source_height,
            knockout,
            clearance,
            obscured_data,
            obscured_remainder,
        })
    }

    pub const fn source_width_thousandths(self) -> u32 {
        self.source_width
    }

    pub const fn source_height_thousandths(self) -> u32 {
        self.source_height
    }

    pub const fn knockout(self) -> [u32; 4] {
        self.knockout
    }

    pub const fn protected_clearance_modules(self) -> u32 {
        self.clearance
    }

    pub const fn obscured_data_modules(self) -> u32 {
        self.obscured_data
    }

    pub const fn obscured_remainder_modules(self) -> u32 {
        self.obscured_remainder
    }

    fn contains_knockout(self, x: u32, y: u32) -> bool {
        x >= self.knockout[0]
            && x < self.knockout[0] + self.knockout[2]
            && y >= self.knockout[1]
            && y < self.knockout[1] + self.knockout[3]
    }
}

pub fn render_candidate_svg(
    encoded: &EncodedQr,
    profile: OutputProfile,
    appearance: CandidateAppearance,
    transparent: bool,
) -> Result<String, Box<dyn Error>> {
    let size = u32::from(encoded.modules().size());
    let extent = size + QUIET_ZONE * 2;
    let output = profile.svg_dimensions();
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {extent} {extent}\">",
        output.width().get(),
        output.height().get()
    );
    if !transparent {
        write!(
            svg,
            "<rect width=\"{extent}\" height=\"{extent}\" fill=\"#ffffff\"/>"
        )?;
    }
    svg.push_str("<g fill=\"#bd0f72\">");
    for (x, y, kind) in visible_modules(encoded, appearance.logo) {
        let x = x + QUIET_ZONE;
        let y = y + QUIET_ZONE;
        if is_square(kind, appearance.function_treatment) {
            write!(svg, "<rect x=\"{x}\" y=\"{y}\" width=\"1\" height=\"1\"/>")?;
        } else {
            let center_x = x * UNITS + UNITS / 2;
            let center_y = y * UNITS + UNITS / 2;
            let radius = u32::from(appearance.dot_diameter_thousandths) / 2;
            write!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                decimal(center_x),
                decimal(center_y),
                decimal(radius)
            )?;
        }
    }
    svg.push_str("</g>");
    if let Some(logo) = appearance.logo {
        let [left, top, width, height] = logo.knockout;
        write!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" fill=\"#ffffff\"/>",
            left + QUIET_ZONE,
            top + QUIET_ZONE,
        )?;
        let body_start = LOGO_BODY.find('>').ok_or("logo root")? + 1;
        let body_end = LOGO_BODY.rfind("</svg>").ok_or("logo close")?;
        write!(
            svg,
            "<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"180 180 640 240\">{}</svg>",
            decimal(logo.source_left + QUIET_ZONE * UNITS),
            decimal(logo.source_top + QUIET_ZONE * UNITS),
            decimal(logo.source_width),
            decimal(logo.source_height),
            &LOGO_BODY[body_start..body_end]
        )?;
    }
    svg.push_str("</svg>");
    Ok(svg)
}

pub fn render_candidate_png(
    encoded: &EncodedQr,
    profile: OutputProfile,
    appearance: CandidateAppearance,
    transparent: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let geometry = profile.geometry(encoded.version())?;
    let dimensions = geometry.canvas_dimensions();
    let width = dimensions.width().get();
    let height = dimensions.height().get();
    let scale = geometry.module_scale().get();
    let quiet_pixels = QUIET_ZONE * scale;
    let origin_x = geometry.outer_padding().left.get() + quiet_pixels;
    let origin_y = geometry.outer_padding().top.get() + quiet_pixels;
    let pixel_count = usize::try_from(u64::from(width) * u64::from(height) * 4)?;
    let mut pixels = if transparent {
        vec![0_u8; pixel_count]
    } else {
        vec![255_u8; pixel_count]
    };
    for (x, y, kind) in visible_modules(encoded, appearance.logo) {
        let pixel_x = origin_x + x * scale;
        let pixel_y = origin_y + y * scale;
        if is_square(kind, appearance.function_treatment) {
            paint_rect(
                &mut pixels,
                dimensions,
                pixel_x,
                pixel_y,
                scale,
                scale,
                Rgba::BRAND,
            )?;
        } else {
            paint_dot(
                &mut pixels,
                dimensions,
                pixel_x,
                pixel_y,
                scale,
                appearance.dot_diameter_thousandths,
                transparent,
            )?;
        }
    }
    if let Some(logo) = appearance.logo {
        let [left, top, knockout_width, knockout_height] = logo.knockout;
        paint_rect(
            &mut pixels,
            dimensions,
            origin_x + left * scale,
            origin_y + top * scale,
            knockout_width * scale,
            knockout_height * scale,
            Rgba::WHITE,
        )?;
        paint_logo(&mut pixels, dimensions, origin_x, origin_y, scale, logo)?;
    }
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::High);
    encoder.set_filter(Filter::NoFilter);
    encoder.write_header()?.write_image_data(&pixels)?;
    Ok(bytes)
}

fn visible_modules(
    encoded: &EncodedQr,
    logo: Option<LogoCandidate>,
) -> impl Iterator<Item = (u32, u32, ModuleKind)> + '_ {
    let size = usize::from(encoded.modules().size());
    encoded
        .modules()
        .modules()
        .enumerate()
        .filter_map(move |(index, module)| {
            let x = u32::try_from(index % size).ok()?;
            let y = u32::try_from(index / size).ok()?;
            (module.is_dark() && !logo.is_some_and(|logo| logo.contains_knockout(x, y)))
                .then_some((x, y, module.kind()))
        })
}

fn is_square(kind: ModuleKind, treatment: FunctionTreatment) -> bool {
    kind == ModuleKind::Finder
        || matches!(treatment, FunctionTreatment::SquareFunctions)
            && !matches!(kind, ModuleKind::Data | ModuleKind::Remainder)
}

fn paint_dot(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    scale: u32,
    diameter: u16,
    transparent: bool,
) -> Result<(), Box<dyn Error>> {
    let center = f64::from(scale) / 2.0;
    let radius = f64::from(scale) * f64::from(diameter) / 2_000.0;
    for offset_y in 0..scale {
        for offset_x in 0..scale {
            let mut covered = 0;
            for sample_y in 0..SAMPLES {
                for sample_x in 0..SAMPLES {
                    let sx = f64::from(offset_x) + (f64::from(sample_x) + 0.5) / f64::from(SAMPLES);
                    let sy = f64::from(offset_y) + (f64::from(sample_y) + 0.5) / f64::from(SAMPLES);
                    if (sx - center).powi(2) + (sy - center).powi(2) <= radius.powi(2) {
                        covered += 1;
                    }
                }
            }
            if covered > 0 {
                blend_pixel(
                    pixels,
                    dimensions,
                    x + offset_x,
                    y + offset_y,
                    covered,
                    SAMPLES * SAMPLES,
                    transparent,
                )?;
            }
        }
    }
    Ok(())
}

fn paint_logo(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    origin_x: u32,
    origin_y: u32,
    scale: u32,
    logo: LogoCandidate,
) -> Result<(), Box<dyn Error>> {
    let left = f64::from(origin_x) + f64::from(logo.source_left) * f64::from(scale) / 1_000.0;
    let top = f64::from(origin_y) + f64::from(logo.source_top) * f64::from(scale) / 1_000.0;
    let width = f64::from(logo.source_width) * f64::from(scale) / 1_000.0;
    let height = f64::from(logo.source_height) * f64::from(scale) / 1_000.0;
    for y in top.floor() as u32..(top + height).ceil() as u32 {
        for x in left.floor() as u32..(left + width).ceil() as u32 {
            let mut covered = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = f64::from(x) + (f64::from(sx) + 0.5) / f64::from(SAMPLES);
                    let py = f64::from(y) + (f64::from(sy) + 0.5) / f64::from(SAMPLES);
                    let source_x = 180.0 + (px - left) * 640.0 / width;
                    let source_y = 180.0 + (py - top) * 240.0 / height;
                    if logo_contains(source_x, source_y) {
                        covered += 1;
                    }
                }
            }
            if covered > 0 {
                blend_pixel(pixels, dimensions, x, y, covered, SAMPLES * SAMPLES, false)?;
            }
        }
    }
    Ok(())
}

fn logo_contains(x: f64, y: f64) -> bool {
    let outer_o = (191.6667..=383.3334).contains(&x) && (192.6667..=409.3334).contains(&y);
    let inner_o = (237.5..=337.5).contains(&x) && (234.3334..=367.6667).contains(&y);
    let e = [
        (808.3333, 234.3333),
        (808.3333, 192.6667),
        (641.6667, 192.6667),
        (641.6667, 409.3333),
        (808.3333, 409.3333),
        (808.3333, 367.6667),
        (687.5, 367.6667),
        (687.5, 321.8333),
        (808.3333, 321.8333),
        (808.3333, 280.1667),
        (687.5, 280.1667),
        (687.5, 234.3333),
    ];
    let n = [
        (566.6667, 334.3333),
        (454.1667, 192.6667),
        (412.5, 192.6667),
        (412.5, 409.3333),
        (458.3333, 409.3333),
        (458.3333, 267.6667),
        (570.8333, 409.3333),
        (612.5, 409.3333),
        (612.5, 192.6667),
        (566.6667, 192.6667),
    ];
    (outer_o && !inner_o) || point_in_polygon(x, y, &e) || point_in_polygon(x, y, &n)
}

fn point_in_polygon(x: f64, y: f64, points: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let (cx, cy) = points[current];
        let (px, py) = points[previous];
        if (cy > y) != (py > y) && x < (px - cx) * (y - cy) / (py - cy) + cx {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn paint_rect(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgba,
) -> Result<(), Box<dyn Error>> {
    for row in y..y + height {
        for column in x..x + width {
            let offset = pixel_offset(dimensions, column, row)?;
            pixels[offset..offset + 4].copy_from_slice(&color.channels());
        }
    }
    Ok(())
}

fn blend_pixel(
    pixels: &mut [u8],
    dimensions: PixelDimensions,
    x: u32,
    y: u32,
    covered: u32,
    samples: u32,
    transparent: bool,
) -> Result<(), Box<dyn Error>> {
    let offset = pixel_offset(dimensions, x, y)?;
    let brand = Rgba::BRAND.channels();
    if transparent {
        pixels[offset..offset + 3].copy_from_slice(&brand[..3]);
        pixels[offset + 3] = u8::try_from((255 * covered + samples / 2) / samples)?;
    } else {
        for channel in 0..3 {
            pixels[offset + channel] = u8::try_from(
                (u32::from(brand[channel]) * covered + 255 * (samples - covered) + samples / 2)
                    / samples,
            )?;
        }
        pixels[offset + 3] = 255;
    }
    Ok(())
}

fn pixel_offset(dimensions: PixelDimensions, x: u32, y: u32) -> Result<usize, Box<dyn Error>> {
    if x >= dimensions.width().get() || y >= dimensions.height().get() {
        return Err("candidate paint escaped canvas".into());
    }
    Ok(usize::try_from(
        (u64::from(y) * u64::from(dimensions.width().get()) + u64::from(x)) * 4,
    )?)
}

fn axis_gap(point: u32, start: u32, length: u32) -> u32 {
    if point < start {
        start - point - 1
    } else {
        point.saturating_sub(start + length)
    }
}

fn div_ceil(value: u32, divisor: u32) -> u32 {
    value / divisor + u32::from(!value.is_multiple_of(divisor))
}

fn decimal(value: u32) -> String {
    let whole = value / UNITS;
    let fraction = value % UNITS;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:03}")
    }
}
