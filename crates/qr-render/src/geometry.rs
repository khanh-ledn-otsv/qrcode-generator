use std::error::Error;
use std::fmt;

const QUIET_ZONE_MODULES_PER_SIDE: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PixelCount(u32);

impl PixelCount {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelDimensions {
    width: PixelCount,
    height: PixelCount,
}

impl PixelDimensions {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width: PixelCount(width),
            height: PixelCount(height),
        }
    }

    #[must_use]
    pub const fn square(side: u32) -> Self {
        Self::new(side, side)
    }

    #[must_use]
    pub const fn width(self) -> PixelCount {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> PixelCount {
        self.height
    }

    pub(crate) const fn is_positive(self) -> bool {
        self.width.0 > 0 && self.height.0 > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ModuleCount(u32);

impl ModuleCount {
    pub fn new(count: u32) -> Result<Self, GeometryError> {
        if count == 0 {
            return Err(GeometryError::InvalidModuleCount);
        }
        Ok(Self(count))
    }

    pub(crate) const fn from_nonzero(count: u32) -> Self {
        Self(count)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterPadding {
    pub left: PixelCount,
    pub right: PixelCount,
    pub top: PixelCount,
    pub bottom: PixelCount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanvasGeometry {
    canvas_dimensions: PixelDimensions,
    matrix_modules: ModuleCount,
    symbol_modules: ModuleCount,
    module_scale: PixelCount,
    rendered_symbol_dimensions: PixelDimensions,
    outer_padding: OuterPadding,
}

impl CanvasGeometry {
    pub fn calculate(
        canvas_dimensions: PixelDimensions,
        matrix_modules: ModuleCount,
    ) -> Result<Self, GeometryError> {
        if !canvas_dimensions.is_positive() {
            return Err(GeometryError::InvalidCanvasDimensions);
        }

        let quiet_zone_total = QUIET_ZONE_MODULES_PER_SIDE
            .checked_mul(2)
            .ok_or(GeometryError::DimensionOverflow)?;
        let symbol_modules = matrix_modules
            .0
            .checked_add(quiet_zone_total)
            .ok_or(GeometryError::DimensionOverflow)?;

        let limiting_side = canvas_dimensions.width.0.min(canvas_dimensions.height.0);
        let raw_scale = limiting_side / symbol_modules;
        let module_scale = raw_scale - (raw_scale % 2);
        if module_scale == 0 {
            return Err(GeometryError::NoPositiveEvenScale);
        }

        let rendered_side = symbol_modules
            .checked_mul(module_scale)
            .ok_or(GeometryError::DimensionOverflow)?;
        let horizontal_remainder = canvas_dimensions
            .width
            .0
            .checked_sub(rendered_side)
            .ok_or(GeometryError::DimensionOverflow)?;
        let vertical_remainder = canvas_dimensions
            .height
            .0
            .checked_sub(rendered_side)
            .ok_or(GeometryError::DimensionOverflow)?;

        if horizontal_remainder % 2 != 0 || vertical_remainder % 2 != 0 {
            return Err(GeometryError::OuterPaddingIsNotIntegral);
        }

        let horizontal_padding = PixelCount(horizontal_remainder / 2);
        let vertical_padding = PixelCount(vertical_remainder / 2);

        Ok(Self {
            canvas_dimensions,
            matrix_modules,
            symbol_modules: ModuleCount::from_nonzero(symbol_modules),
            module_scale: PixelCount(module_scale),
            rendered_symbol_dimensions: PixelDimensions::square(rendered_side),
            outer_padding: OuterPadding {
                left: horizontal_padding,
                right: horizontal_padding,
                top: vertical_padding,
                bottom: vertical_padding,
            },
        })
    }

    #[must_use]
    pub const fn canvas_dimensions(self) -> PixelDimensions {
        self.canvas_dimensions
    }

    #[must_use]
    pub const fn matrix_modules(self) -> ModuleCount {
        self.matrix_modules
    }

    #[must_use]
    pub const fn quiet_zone_modules(self) -> ModuleCount {
        ModuleCount::from_nonzero(QUIET_ZONE_MODULES_PER_SIDE)
    }

    #[must_use]
    pub const fn symbol_modules(self) -> ModuleCount {
        self.symbol_modules
    }

    #[must_use]
    pub const fn module_scale(self) -> PixelCount {
        self.module_scale
    }

    #[must_use]
    pub const fn rendered_symbol_dimensions(self) -> PixelDimensions {
        self.rendered_symbol_dimensions
    }

    /// Padding outside the complete symbol, including its quiet zone.
    ///
    /// Renderers must fill this region only with the selected background.
    #[must_use]
    pub const fn outer_padding(self) -> OuterPadding {
        self.outer_padding
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    InvalidCanvasDimensions,
    InvalidModuleCount,
    DimensionOverflow,
    NoPositiveEvenScale,
    OuterPaddingIsNotIntegral,
    VersionExceedsProfile {
        requested: crate::QrVersion,
        maximum: crate::QrVersion,
    },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanvasDimensions => {
                formatter.write_str("canvas dimensions must be positive")
            }
            Self::InvalidModuleCount => formatter.write_str("module count must be positive"),
            Self::DimensionOverflow => formatter.write_str("geometry dimensions overflowed"),
            Self::NoPositiveEvenScale => {
                formatter.write_str("no positive even module scale fits the canvas")
            }
            Self::OuterPaddingIsNotIntegral => {
                formatter.write_str("outer padding cannot be symmetric and integral")
            }
            Self::VersionExceedsProfile { requested, maximum } => write!(
                formatter,
                "QR version {} exceeds the profile maximum of {}",
                requested.number(),
                maximum.number()
            ),
        }
    }
}

impl Error for GeometryError {}
