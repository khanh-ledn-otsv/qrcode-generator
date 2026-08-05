//! Deterministic SVG and PNG rendering for encoded QR symbols.

#![forbid(unsafe_code)]

mod geometry;
mod profile;

pub use geometry::{
    CanvasGeometry, GeometryError, ModuleCount, OuterPadding, PaddingContent, PixelCount,
    PixelDimensions,
};
pub use profile::{OutputProfile, ProfileError, ProfileId, SUPPORTED_PROFILES};
pub use qr_core::{Version, VersionError};
