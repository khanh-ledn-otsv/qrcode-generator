//! Deterministic SVG and PNG rendering for encoded QR symbols.

#![forbid(unsafe_code)]

mod geometry;
mod profile;

pub use geometry::{
    CanvasGeometry, GeometryError, ModuleCount, OuterPadding, PixelCount, PixelDimensions,
};
pub use profile::{OutputProfile, ProfileError, ProfileId, QrVersion, SUPPORTED_PROFILES};
