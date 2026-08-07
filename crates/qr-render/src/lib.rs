//! Deterministic SVG and PNG rendering for encoded QR symbols.

#![forbid(unsafe_code)]

mod geometry;
mod model;
mod png;
mod profile;
mod svg;

pub use geometry::{
    CanvasGeometry, GeometryError, ModuleCount, OuterPadding, PaddingContent, PixelCount,
    PixelDimensions, SymbolGeometry,
};
pub use model::{
    APPROVED_BACKGROUNDS, APPROVED_DATA_MODULE_STYLES, APPROVED_FOREGROUNDS, Background,
    BrandableCell, ContrastRatio, DataModuleStyle, FinderStyle, Foreground, FunctionModuleStyle,
    LogoStyle, MAX_RGBA_BUFFER_BYTES, ModuleDimensions, ModulePoint, OutputSafety, PixelPoint,
    PngPlacement, RenderCell, RenderError, RenderModel, RenderOptions, Rgba, SvgPlacement,
};
pub use png::render_png;
pub use profile::{OutputProfile, ProfileError, ProfileId, SUPPORTED_PROFILES};
pub use qr_core::{Version, VersionError};
pub use svg::render_svg;
