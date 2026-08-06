use std::error::Error;
use std::fmt;

use qr_core::EncodedQr;
use qr_core::matrix::{MaskId, Module, ModuleKind, ModuleMatrix};
use qr_core::tables::ErrorCorrection;

use crate::{
    CanvasGeometry, GeometryError, ModuleCount, OuterPadding, OutputProfile, PixelCount,
    PixelDimensions, ProfileError, SymbolGeometry, Version,
};

/// Defensive, target-independent ceiling for a direct RGBA render buffer.
pub const MAX_RGBA_BUFFER_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Rgba {
    pub const BLACK: Self = Self::opaque(0, 0, 0);
    pub const WHITE: Self = Self::opaque(255, 255, 255);

    #[must_use]
    pub const fn opaque(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: u8::MAX,
        }
    }

    #[must_use]
    pub const fn channels(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Background {
    Opaque(Rgba),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataModuleStyle {
    Square,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionModuleStyle {
    Square,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinderStyle {
    StandardSquare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogoStyle {
    None,
}

/// Validated rendering choices for the approved safe preset.
///
/// Fields are private so artifact renderers cannot introduce unapproved
/// combinations or alter encoding decisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    profile: OutputProfile,
    foreground: Rgba,
    background: Background,
    data_module_style: DataModuleStyle,
    function_module_style: FunctionModuleStyle,
    finder_style: FinderStyle,
    logo_style: LogoStyle,
}

impl RenderOptions {
    pub fn safe(profile: OutputProfile) -> Result<Self, RenderError> {
        profile.validate().map_err(RenderError::InvalidProfile)?;
        Ok(Self {
            profile,
            foreground: Rgba::BLACK,
            background: Background::Opaque(Rgba::WHITE),
            data_module_style: DataModuleStyle::Square,
            function_module_style: FunctionModuleStyle::Square,
            finder_style: FinderStyle::StandardSquare,
            logo_style: LogoStyle::None,
        })
    }

    #[must_use]
    pub const fn profile(self) -> OutputProfile {
        self.profile
    }

    #[must_use]
    pub const fn foreground(self) -> Rgba {
        self.foreground
    }

    #[must_use]
    pub const fn background(self) -> Background {
        self.background
    }

    #[must_use]
    pub const fn data_module_style(self) -> DataModuleStyle {
        self.data_module_style
    }

    #[must_use]
    pub const fn function_module_style(self) -> FunctionModuleStyle {
        self.function_module_style
    }

    #[must_use]
    pub const fn finder_style(self) -> FinderStyle {
        self.finder_style
    }

    #[must_use]
    pub const fn logo_style(self) -> LogoStyle {
        self.logo_style
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleDimensions {
    width: ModuleCount,
    height: ModuleCount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModulePoint {
    x: ModuleCount,
    y: ModuleCount,
}

impl ModulePoint {
    fn square_offset(offset: ModuleCount) -> Self {
        Self {
            x: offset,
            y: offset,
        }
    }

    #[must_use]
    pub const fn x(self) -> ModuleCount {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> ModuleCount {
        self.y
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelPoint {
    x: PixelCount,
    y: PixelCount,
}

impl PixelPoint {
    #[must_use]
    pub const fn x(self) -> PixelCount {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> PixelCount {
        self.y
    }
}

impl ModuleDimensions {
    fn square(side: ModuleCount) -> Self {
        Self {
            width: side,
            height: side,
        }
    }

    #[must_use]
    pub const fn width(self) -> ModuleCount {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> ModuleCount {
        self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvgPlacement {
    output_dimensions: PixelDimensions,
    view_box: ModuleDimensions,
    matrix_origin: ModulePoint,
}

impl SvgPlacement {
    #[must_use]
    pub const fn output_dimensions(self) -> PixelDimensions {
        self.output_dimensions
    }

    #[must_use]
    pub const fn view_box(self) -> ModuleDimensions {
        self.view_box
    }

    #[must_use]
    pub const fn matrix_origin(self) -> ModulePoint {
        self.matrix_origin
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PngPlacement {
    symbol: SymbolGeometry,
    canvas: CanvasGeometry,
    matrix_origin: PixelPoint,
    rgba_buffer_len: usize,
}

impl PngPlacement {
    #[must_use]
    pub const fn symbol(self) -> SymbolGeometry {
        self.symbol
    }

    #[must_use]
    pub const fn canvas_dimensions(self) -> PixelDimensions {
        self.canvas.canvas_dimensions()
    }

    #[must_use]
    pub const fn module_scale(self) -> PixelCount {
        self.canvas.module_scale()
    }

    #[must_use]
    pub const fn rendered_symbol_dimensions(self) -> PixelDimensions {
        self.canvas.rendered_symbol_dimensions()
    }

    #[must_use]
    pub const fn outer_padding(self) -> OuterPadding {
        self.canvas.outer_padding()
    }

    #[must_use]
    pub const fn matrix_origin(self) -> PixelPoint {
        self.matrix_origin
    }

    #[must_use]
    pub const fn rgba_buffer_len(self) -> usize {
        self.rgba_buffer_len
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderCell {
    x: u16,
    y: u16,
    module: Module,
}

/// A branding target proven to contain only payload data or remainder bits.
///
/// There is deliberately no public constructor: function modules can only be
/// observed as `RenderCell` and cannot be converted into a branding target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrandableCell {
    cell: RenderCell,
}

impl BrandableCell {
    #[must_use]
    pub const fn x(self) -> u16 {
        self.cell.x()
    }

    #[must_use]
    pub const fn y(self) -> u16 {
        self.cell.y()
    }

    #[must_use]
    pub const fn module(self) -> Module {
        self.cell.module()
    }
}

impl RenderCell {
    #[must_use]
    pub const fn x(self) -> u16 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> u16 {
        self.y
    }

    #[must_use]
    pub const fn module(self) -> Module {
        self.module
    }

    /// Protected cells can never be targets for future branding geometry.
    #[must_use]
    pub const fn is_protected(self) -> bool {
        !matches!(self.module.kind(), ModuleKind::Data | ModuleKind::Remainder)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderModel<'encoded> {
    encoded: &'encoded EncodedQr,
    options: RenderOptions,
    symbol: SymbolGeometry,
    svg_placement: SvgPlacement,
    png_placement: PngPlacement,
}

impl<'encoded> RenderModel<'encoded> {
    pub fn new(encoded: &'encoded EncodedQr, options: RenderOptions) -> Result<Self, RenderError> {
        let png_geometry = options
            .profile()
            .geometry(encoded.version())
            .map_err(RenderError::InvalidGeometry)?;
        let symbol = png_geometry.symbol();
        let rgba_buffer_len = checked_rgba_buffer_len(png_geometry.canvas_dimensions())?;
        let quiet_zone_pixels = symbol
            .quiet_zone_modules_per_side()
            .get()
            .checked_mul(png_geometry.module_scale().get())
            .ok_or(RenderError::DimensionOverflow)?;
        let matrix_x = png_geometry
            .outer_padding()
            .left
            .get()
            .checked_add(quiet_zone_pixels)
            .ok_or(RenderError::DimensionOverflow)?;
        let matrix_y = png_geometry
            .outer_padding()
            .top
            .get()
            .checked_add(quiet_zone_pixels)
            .ok_or(RenderError::DimensionOverflow)?;

        Ok(Self {
            encoded,
            options,
            symbol,
            svg_placement: SvgPlacement {
                output_dimensions: options.profile().svg_dimensions(),
                view_box: ModuleDimensions::square(symbol.extent_modules()),
                matrix_origin: ModulePoint::square_offset(symbol.quiet_zone_modules_per_side()),
            },
            png_placement: PngPlacement {
                symbol,
                canvas: png_geometry,
                matrix_origin: PixelPoint {
                    x: PixelCount::from_u32(matrix_x),
                    y: PixelCount::from_u32(matrix_y),
                },
                rgba_buffer_len,
            },
        })
    }

    #[must_use]
    pub const fn version(&self) -> Version {
        self.encoded.version()
    }

    #[must_use]
    pub const fn ecc(&self) -> ErrorCorrection {
        self.encoded.ecc()
    }

    #[must_use]
    pub const fn mask(&self) -> MaskId {
        self.encoded.mask()
    }

    #[must_use]
    pub const fn matrix(&self) -> &ModuleMatrix {
        self.encoded.modules()
    }

    #[must_use]
    pub const fn options(&self) -> RenderOptions {
        self.options
    }

    #[must_use]
    pub const fn symbol(&self) -> SymbolGeometry {
        self.symbol
    }

    #[must_use]
    pub const fn svg_placement(&self) -> SvgPlacement {
        self.svg_placement
    }

    #[must_use]
    pub const fn png_placement(&self) -> PngPlacement {
        self.png_placement
    }

    pub fn cells(&self) -> impl Iterator<Item = RenderCell> + '_ {
        let size = self.matrix().size();
        (0..size).flat_map(move |y| {
            (0..size).filter_map(move |x| {
                self.matrix()
                    .module(x, y)
                    .map(|module| RenderCell { x, y, module })
            })
        })
    }

    pub fn brandable_cells(&self) -> impl Iterator<Item = BrandableCell> + '_ {
        self.cells()
            .filter_map(|cell| (!cell.is_protected()).then_some(BrandableCell { cell }))
    }
}

fn checked_rgba_buffer_len(dimensions: PixelDimensions) -> Result<usize, RenderError> {
    let width = u64::from(dimensions.width().get());
    let height = u64::from(dimensions.height().get());
    let length = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RenderError::DimensionOverflow)?;
    if length > MAX_RGBA_BUFFER_BYTES {
        return Err(RenderError::AllocationTooLarge {
            required_bytes: length,
            maximum_bytes: MAX_RGBA_BUFFER_BYTES,
        });
    }
    usize::try_from(length).map_err(|_| RenderError::DimensionOverflow)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    InvalidProfile(ProfileError),
    InvalidGeometry(GeometryError),
    DimensionOverflow,
    AllocationTooLarge {
        required_bytes: u64,
        maximum_bytes: u64,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(error) => write!(formatter, "invalid render profile: {error}"),
            Self::InvalidGeometry(error) => write!(formatter, "invalid render geometry: {error}"),
            Self::DimensionOverflow => formatter.write_str("render dimensions overflowed"),
            Self::AllocationTooLarge {
                required_bytes,
                maximum_bytes,
            } => write!(
                formatter,
                "render buffer requires {required_bytes} bytes; maximum is {maximum_bytes} bytes"
            ),
        }
    }
}

impl Error for RenderError {}
