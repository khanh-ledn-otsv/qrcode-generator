use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage, imageops};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TransformSuite {
    schema_version: u8,
    seed: u64,
    transforms: Vec<Transform>,
}

impl TransformSuite {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = workspace_root().join("tests/adverse/parameters.json");
        let suite: Self = serde_json::from_slice(&fs::read(path)?)?;
        if suite.schema_version != 1 || suite.transforms.is_empty() {
            return Err("unsupported or empty adverse-transform manifest".into());
        }
        Ok(suite)
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub fn kinds(&self) -> Vec<&'static str> {
        self.transforms.iter().map(Transform::kind).collect()
    }

    pub fn transforms(&self) -> &[Transform] {
        &self.transforms
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Transform {
    Blur {
        id: String,
        sigma: f32,
    },
    Scaling {
        id: String,
        percent: u32,
    },
    Jpeg {
        id: String,
        quality: u8,
    },
    Rotation {
        id: String,
        degrees: f32,
    },
    Perspective {
        id: String,
        top_inset_percent: u32,
        bottom_inset_percent: u32,
    },
    Contrast {
        id: String,
        delta: f32,
    },
    Brightness {
        id: String,
        delta: i32,
    },
    Background {
        id: String,
        rgba: [u8; 4],
        alternate_rgba: Option<[u8; 4]>,
        cell_size: Option<u32>,
    },
    DotGain {
        id: String,
        radius: u32,
    },
    InkLoss {
        id: String,
        period: u64,
    },
    Grayscale {
        id: String,
    },
}

impl Transform {
    pub fn id(&self) -> &str {
        match self {
            Self::Blur { id, .. }
            | Self::Scaling { id, .. }
            | Self::Jpeg { id, .. }
            | Self::Rotation { id, .. }
            | Self::Perspective { id, .. }
            | Self::Contrast { id, .. }
            | Self::Brightness { id, .. }
            | Self::Background { id, .. }
            | Self::DotGain { id, .. }
            | Self::InkLoss { id, .. }
            | Self::Grayscale { id } => id,
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Blur { .. } => "blur",
            Self::Scaling { .. } => "scaling",
            Self::Jpeg { .. } => "jpeg",
            Self::Rotation { .. } => "rotation",
            Self::Perspective { .. } => "perspective",
            Self::Contrast { .. } => "contrast",
            Self::Brightness { .. } => "brightness",
            Self::Background { .. } => "background",
            Self::DotGain { .. } => "dot_gain",
            Self::InkLoss { .. } => "ink_loss",
            Self::Grayscale { .. } => "grayscale",
        }
    }

    pub fn apply(&self, source: &[u8], seed: u64) -> Result<Vec<u8>, Box<dyn Error>> {
        let source = image::load_from_memory_with_format(source, ImageFormat::Png)?.into_rgba8();
        let transformed = match *self {
            Self::Blur { sigma, .. } => imageops::blur(&source, sigma),
            Self::Scaling { percent, .. } => scale_simulation(&source, percent)?,
            Self::Jpeg { quality, .. } => jpeg_simulation(&source, quality)?,
            Self::Rotation { degrees, .. } => rotate(&source, degrees),
            Self::Perspective {
                top_inset_percent,
                bottom_inset_percent,
                ..
            } => perspective(&source, top_inset_percent, bottom_inset_percent),
            Self::Contrast { delta, .. } => imageops::contrast(&source, delta),
            Self::Brightness { delta, .. } => imageops::brighten(&source, delta),
            Self::Background {
                rgba,
                alternate_rgba,
                cell_size,
                ..
            } => replace_background(&source, Rgba(rgba), alternate_rgba.map(Rgba), cell_size),
            Self::DotGain { radius, .. } => dot_gain(&source, radius),
            Self::InkLoss { period, .. } => ink_loss(&source, seed, period)?,
            Self::Grayscale { .. } => {
                DynamicImage::ImageLuma8(imageops::grayscale(&source)).into_rgba8()
            }
        };
        encode_png(&transformed)
    }
}

pub fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32), Box<dyn Error>> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)?;
    Ok((image.width(), image.height()))
}

pub fn pixels_differ(left: &[u8], right: &[u8]) -> Result<bool, Box<dyn Error>> {
    let left = image::load_from_memory_with_format(left, ImageFormat::Png)?.into_rgba8();
    let right = image::load_from_memory_with_format(right, ImageFormat::Png)?.into_rgba8();
    Ok(left != right)
}

pub fn composite_on(source: &[u8], rgba: [u8; 4]) -> Result<Vec<u8>, Box<dyn Error>> {
    let source = image::load_from_memory_with_format(source, ImageFormat::Png)?.into_rgba8();
    encode_png(&composite_background(&source, Rgba(rgba)))
}

fn scale_simulation(source: &RgbaImage, percent: u32) -> Result<RgbaImage, Box<dyn Error>> {
    if percent == 0 || percent >= 100 {
        return Err("scaling percent must be between 1 and 99".into());
    }
    let width = source.width().saturating_mul(percent).div_ceil(100).max(1);
    let height = source.height().saturating_mul(percent).div_ceil(100).max(1);
    let reduced = imageops::resize(source, width, height, FilterType::Triangle);
    Ok(imageops::resize(
        &reduced,
        source.width(),
        source.height(),
        FilterType::Nearest,
    ))
}

fn jpeg_simulation(source: &RgbaImage, quality: u8) -> Result<RgbaImage, Box<dyn Error>> {
    let flattened = composite_background(source, Rgba([255, 255, 255, 255]));
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, quality)
        .encode_image(&DynamicImage::ImageRgba8(flattened))?;
    Ok(image::load_from_memory_with_format(&jpeg, ImageFormat::Jpeg)?.into_rgba8())
}

fn rotate(source: &RgbaImage, degrees: f32) -> RgbaImage {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let center_x = (source.width() as f32 - 1.0) / 2.0;
    let center_y = (source.height() as f32 - 1.0) / 2.0;
    RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let dx = x as f32 - center_x;
        let dy = y as f32 - center_y;
        let source_x = cos * dx + sin * dy + center_x;
        let source_y = -sin * dx + cos * dy + center_y;
        sample_nearest(source, source_x, source_y)
    })
}

fn perspective(source: &RgbaImage, top_percent: u32, bottom_percent: u32) -> RgbaImage {
    let width = source.width() as f32;
    let height = source.height() as f32;
    RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let vertical = y as f32 / (height - 1.0).max(1.0);
        let inset_percent =
            top_percent as f32 * (1.0 - vertical) + bottom_percent as f32 * vertical;
        let inset = width * inset_percent / 100.0;
        let span = (width - 1.0 - 2.0 * inset).max(1.0);
        let x = x as f32;
        if x < inset || x > inset + span {
            Rgba([255, 255, 255, 255])
        } else {
            sample_nearest(source, (x - inset) * (width - 1.0) / span, y as f32)
        }
    })
}

fn sample_nearest(source: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    if x < 0.0 || y < 0.0 || x >= source.width() as f32 || y >= source.height() as f32 {
        return Rgba([255, 255, 255, 255]);
    }
    let sampled_x = (x.round() as u32).min(source.width() - 1);
    let sampled_y = (y.round() as u32).min(source.height() - 1);
    *source.get_pixel(sampled_x, sampled_y)
}

fn composite_background(source: &RgbaImage, background: Rgba<u8>) -> RgbaImage {
    RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let foreground = source.get_pixel(x, y).0;
        let alpha = u16::from(foreground[3]);
        let mut output = background.0;
        for channel in 0..3 {
            let blended = u16::from(foreground[channel]) * alpha
                + u16::from(background[channel]) * (255 - alpha);
            output[channel] = ((blended + 127) / 255) as u8;
        }
        output[3] = 255;
        Rgba(output)
    })
}

fn replace_background(
    source: &RgbaImage,
    background: Rgba<u8>,
    alternate: Option<Rgba<u8>>,
    cell_size: Option<u32>,
) -> RgbaImage {
    let composited = composite_background(source, background);
    RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let original = *source.get_pixel(x, y);
        if is_ink(original) {
            *composited.get_pixel(x, y)
        } else {
            match (alternate, cell_size) {
                (Some(alternate), Some(cell_size)) if cell_size != 0 => {
                    if (x / cell_size + y / cell_size).is_multiple_of(2) {
                        background
                    } else {
                        alternate
                    }
                }
                _ => background,
            }
        }
    })
}

fn dot_gain(source: &RgbaImage, radius: u32) -> RgbaImage {
    RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let original = *source.get_pixel(x, y);
        if is_ink(original) {
            return original;
        }
        let x_start = x.saturating_sub(radius);
        let x_end = x.saturating_add(radius).min(source.width() - 1);
        let y_start = y.saturating_sub(radius);
        let y_end = y.saturating_add(radius).min(source.height() - 1);
        for neighbor_y in y_start..=y_end {
            for neighbor_x in x_start..=x_end {
                let candidate = *source.get_pixel(neighbor_x, neighbor_y);
                if is_ink(candidate) {
                    return candidate;
                }
            }
        }
        original
    })
}

fn ink_loss(source: &RgbaImage, seed: u64, period: u64) -> Result<RgbaImage, Box<dyn Error>> {
    if period == 0 {
        return Err("ink-loss period must be positive".into());
    }
    Ok(RgbaImage::from_fn(
        source.width(),
        source.height(),
        |x, y| {
            let pixel = *source.get_pixel(x, y);
            let coordinate = u64::from(y) * u64::from(source.width()) + u64::from(x);
            let mixed = coordinate
                .wrapping_add(seed)
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            if is_ink(pixel) && mixed % period == 0 {
                Rgba([pixel.0[0], pixel.0[1], pixel.0[2], 0])
            } else {
                pixel
            }
        },
    ))
}

fn is_ink(pixel: Rgba<u8>) -> bool {
    pixel.0[3] != 0 && pixel.0[..3].iter().any(|channel| *channel < 240)
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone()).write_to(&mut bytes, ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
