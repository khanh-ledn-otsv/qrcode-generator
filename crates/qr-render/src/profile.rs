use std::error::Error;
use std::fmt;

use qr_core::Version;

use crate::{CanvasGeometry, GeometryError, ModuleCount, PixelDimensions};

const PNG_SCALE_FACTOR: u32 = 3;
const MINIMUM_MAX_VERSION_MODULE_SCALE: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileId {
    Inline,
    Content,
    Landing,
    Print,
    AdaptiveBranded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputProfile {
    id: ProfileId,
    base_dimensions: PixelDimensions,
    png_dimensions: PixelDimensions,
    maximum_version: Version,
}

impl OutputProfile {
    pub fn try_new(
        id: ProfileId,
        base_dimensions: PixelDimensions,
        png_dimensions: PixelDimensions,
        maximum_version: Version,
    ) -> Result<Self, ProfileError> {
        if !base_dimensions.is_positive() || !png_dimensions.is_positive() {
            return Err(ProfileError::DimensionsMustBePositive);
        }
        if base_dimensions.width() != base_dimensions.height()
            || png_dimensions.width() != png_dimensions.height()
        {
            return Err(ProfileError::DimensionsMustBeSquare);
        }

        let expected_png_width = base_dimensions
            .width()
            .get()
            .checked_mul(PNG_SCALE_FACTOR)
            .ok_or(ProfileError::DimensionOverflow)?;
        let expected_png_height = base_dimensions
            .height()
            .get()
            .checked_mul(PNG_SCALE_FACTOR)
            .ok_or(ProfileError::DimensionOverflow)?;
        if png_dimensions != PixelDimensions::new(expected_png_width, expected_png_height) {
            return Err(ProfileError::PngDimensionsAreNotTriple {
                base: base_dimensions,
                png: png_dimensions,
            });
        }

        let profile = Self {
            id,
            base_dimensions,
            png_dimensions,
            maximum_version,
        };
        let maximum_geometry = profile
            .geometry(maximum_version)
            .map_err(ProfileError::InvalidGeometry)?;
        if maximum_geometry.module_scale().get() < MINIMUM_MAX_VERSION_MODULE_SCALE {
            return Err(ProfileError::MaximumVersionScaleBelowSix);
        }
        Ok(profile)
    }

    const fn compiled(id: ProfileId, base_side: u32, png_side: u32, maximum_version: u8) -> Self {
        Self {
            id,
            base_dimensions: PixelDimensions::square(base_side),
            png_dimensions: PixelDimensions::square(png_side),
            maximum_version: match Version::new(maximum_version) {
                Ok(version) => version,
                Err(_) => panic!("compiled output profile has an invalid maximum version"),
            },
        }
    }

    #[must_use]
    pub const fn id(self) -> ProfileId {
        self.id
    }

    #[must_use]
    pub const fn base_dimensions(self) -> PixelDimensions {
        self.base_dimensions
    }

    #[must_use]
    pub const fn png_dimensions(self) -> PixelDimensions {
        self.png_dimensions
    }

    #[must_use]
    pub const fn maximum_version(self) -> Version {
        self.maximum_version
    }

    pub fn validate(self) -> Result<(), ProfileError> {
        Self::try_new(
            self.id,
            self.base_dimensions,
            self.png_dimensions,
            self.maximum_version,
        )
        .map(|_| ())
    }

    #[must_use]
    pub const fn svg_dimensions(self) -> PixelDimensions {
        self.base_dimensions
    }

    pub fn geometry(self, version: Version) -> Result<CanvasGeometry, GeometryError> {
        if version > self.maximum_version {
            return Err(GeometryError::VersionExceedsProfile {
                requested: version,
                maximum: self.maximum_version,
            });
        }
        CanvasGeometry::calculate(
            self.png_dimensions,
            ModuleCount::new(u32::from(version.symbol_size()))?,
        )
    }
}

pub const SUPPORTED_PROFILES: [OutputProfile; 5] = [
    OutputProfile::compiled(ProfileId::Inline, 100, 300, 6),
    OutputProfile::compiled(ProfileId::Content, 120, 360, 8),
    OutputProfile::compiled(ProfileId::Landing, 150, 450, 12),
    OutputProfile::compiled(ProfileId::Print, 160, 480, 13),
    OutputProfile::compiled(ProfileId::AdaptiveBranded, 180, 540, 10),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileError {
    DimensionsMustBePositive,
    DimensionsMustBeSquare,
    DimensionOverflow,
    PngDimensionsAreNotTriple {
        base: PixelDimensions,
        png: PixelDimensions,
    },
    InvalidGeometry(GeometryError),
    MaximumVersionScaleBelowSix,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionsMustBePositive => {
                formatter.write_str("profile dimensions must be positive")
            }
            Self::DimensionsMustBeSquare => {
                formatter.write_str("profile dimensions must be square")
            }
            Self::DimensionOverflow => formatter.write_str("profile dimensions overflowed"),
            Self::PngDimensionsAreNotTriple { .. } => formatter
                .write_str("PNG dimensions must be exactly three times the base dimensions"),
            Self::InvalidGeometry(error) => write!(formatter, "invalid profile geometry: {error}"),
            Self::MaximumVersionScaleBelowSix => formatter.write_str(
                "the profile maximum version must retain at least six pixels per module",
            ),
        }
    }
}

impl Error for ProfileError {}

#[cfg(test)]
mod tests {
    use super::{OutputProfile, ProfileId};

    #[test]
    fn compiled_profile_constructor_matches_its_arguments() {
        let constructor: fn(ProfileId, u32, u32, u8) -> OutputProfile = OutputProfile::compiled;
        let profile = std::hint::black_box(constructor)(ProfileId::Inline, 100, 300, 6);
        assert_eq!(profile.id(), ProfileId::Inline);
        assert_eq!(profile.base_dimensions().width().get(), 100);
        assert_eq!(profile.png_dimensions().width().get(), 300);
        assert_eq!(profile.maximum_version().number(), 6);
    }
}
