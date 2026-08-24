use std::error::Error;
use std::fmt;

use qr_core::Version;

use crate::{CanvasGeometry, GeometryError, ModuleCount, PixelDimensions};

const PNG_SCALE_FACTOR: u32 = 3;
const MINIMUM_MAX_VERSION_MODULE_SCALE: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileId {
    Small,
    Standard,
    PrimaryCta,
    HeroCampaign,
    BusinessCard,
    FlyerBrochure,
    PosterPackage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputProfile {
    id: ProfileId,
    base_dimensions: PixelDimensions,
    png_dimensions: PixelDimensions,
    minimum_version: Version,
    maximum_version: Version,
}

impl OutputProfile {
    pub fn try_new(
        id: ProfileId,
        base_dimensions: PixelDimensions,
        png_dimensions: PixelDimensions,
        minimum_version: Version,
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
        if minimum_version > maximum_version {
            return Err(ProfileError::InvertedVersionRange {
                minimum: minimum_version,
                maximum: maximum_version,
            });
        }

        let profile = Self {
            id,
            base_dimensions,
            png_dimensions,
            minimum_version,
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

    const fn compiled(
        id: ProfileId,
        base_side: u32,
        png_side: u32,
        minimum_version: u8,
        maximum_version: u8,
    ) -> Self {
        Self {
            id,
            base_dimensions: PixelDimensions::square(base_side),
            png_dimensions: PixelDimensions::square(png_side),
            minimum_version: match Version::new(minimum_version) {
                Ok(version) => version,
                Err(_) => panic!("compiled output profile has an invalid minimum version"),
            },
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
    pub const fn minimum_version(self) -> Version {
        self.minimum_version
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
            self.minimum_version,
            self.maximum_version,
        )
        .map(|_| ())
    }

    #[must_use]
    pub const fn svg_dimensions(self) -> PixelDimensions {
        self.base_dimensions
    }

    pub fn svg_dimensions_for(self, version: Version) -> Result<PixelDimensions, GeometryError> {
        self.dimensions_for(version, self.base_dimensions)
    }

    pub fn png_dimensions_for(self, version: Version) -> Result<PixelDimensions, GeometryError> {
        self.dimensions_for(version, self.png_dimensions)
    }

    fn dimensions_for(
        self,
        version: Version,
        fixed_dimensions: PixelDimensions,
    ) -> Result<PixelDimensions, GeometryError> {
        if version > self.maximum_version {
            return Err(GeometryError::VersionExceedsProfile {
                requested: version,
                maximum: self.maximum_version,
            });
        }
        Ok(fixed_dimensions)
    }

    pub fn geometry(self, version: Version) -> Result<CanvasGeometry, GeometryError> {
        CanvasGeometry::calculate(
            self.png_dimensions_for(version)?,
            ModuleCount::new(u32::from(version.symbol_size()))?,
        )
    }
}

pub const SUPPORTED_PROFILES: [OutputProfile; 7] = [
    OutputProfile::compiled(ProfileId::Small, 100, 300, 5, 6),
    OutputProfile::compiled(ProfileId::Standard, 120, 360, 5, 8),
    OutputProfile::compiled(ProfileId::PrimaryCta, 160, 480, 5, 12),
    OutputProfile::compiled(ProfileId::HeroCampaign, 200, 600, 8, 12),
    OutputProfile::compiled(ProfileId::BusinessCard, 148, 444, 5, 12),
    OutputProfile::compiled(ProfileId::FlyerBrochure, 177, 531, 5, 12),
    OutputProfile::compiled(ProfileId::PosterPackage, 236, 708, 5, 12),
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
    InvertedVersionRange {
        minimum: Version,
        maximum: Version,
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
            Self::InvertedVersionRange { .. } => {
                formatter.write_str("profile minimum version must not exceed its maximum version")
            }
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
        let constructor: fn(ProfileId, u32, u32, u8, u8) -> OutputProfile = OutputProfile::compiled;
        let profile = std::hint::black_box(constructor)(ProfileId::Small, 100, 300, 5, 6);
        assert_eq!(profile.id(), ProfileId::Small);
        assert_eq!(profile.base_dimensions().width().get(), 100);
        assert_eq!(profile.png_dimensions().width().get(), 300);
        assert_eq!(profile.minimum_version().number(), 5);
        assert_eq!(profile.maximum_version().number(), 6);
    }
}
