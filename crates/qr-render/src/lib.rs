//! Deterministic SVG and PNG rendering for encoded QR symbols.

#![forbid(unsafe_code)]

mod geometry;
mod logo;
mod model;
mod png;
mod profile;
mod svg;

pub use geometry::{
    CanvasGeometry, GeometryError, ModuleCount, OuterPadding, PaddingContent, PixelCount,
    PixelDimensions, SymbolGeometry,
};
pub use logo::{
    BRANDED_LOGO_VERSION, BUNDLED_LOGO_SVG, LogoKnockoutBounds, LogoPlacement, LogoSourceBounds,
    ModuleCoordinate,
};
pub use model::{
    APPROVED_BACKGROUNDS, APPROVED_FINDERS, APPROVED_FOREGROUNDS, APPROVED_LOGO_STYLES,
    APPROVED_MODULE_STYLES, Background, BrandableCell, ContrastRatio, FinderStyle, Foreground,
    GlyphOwnership, LogoStyle, MAX_RGBA_BUFFER_BYTES, ModuleDimensions, ModulePoint, ModuleStyle,
    OutputSafety, PixelPoint, PngPlacement, RenderCell, RenderError, RenderModel, RenderOptions,
    Rgba, SvgPlacement, SymbolGlyph,
};
pub use png::render_png;
pub use profile::{OutputProfile, ProfileError, ProfileId, SUPPORTED_PROFILES};
pub use qr_core::{Version, VersionError};
pub use svg::render_svg;
