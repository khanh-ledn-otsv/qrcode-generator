//! Deterministic SVG and PNG rendering for encoded QR symbols.

#![forbid(unsafe_code)]

mod geometry;
mod model;
mod profile;

pub use geometry::{
    CanvasGeometry, GeometryError, ModuleCount, OuterPadding, PaddingContent, PixelCount,
    PixelDimensions, SymbolGeometry,
};
pub use model::{
    Background, BrandableCell, DataModuleStyle, FinderStyle, FunctionModuleStyle, LogoStyle,
    MAX_RGBA_BUFFER_BYTES, ModuleDimensions, ModulePoint, PixelPoint, PngPlacement, RenderCell,
    RenderError, RenderModel, RenderOptions, Rgba, SvgPlacement,
};
pub use profile::{OutputProfile, ProfileError, ProfileId, SUPPORTED_PROFILES};
pub use qr_core::{Version, VersionError};
