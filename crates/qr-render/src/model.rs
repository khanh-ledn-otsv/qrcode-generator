use std::error::Error;
use std::fmt;

use qr_core::EncodedQr;
use qr_core::matrix::{MaskId, Module, ModuleKind, ModuleMatrix};
use qr_core::tables::ErrorCorrection;

use crate::logo::calculate_logo_placement;
use crate::{
    CanvasGeometry, GeometryError, LogoPlacement, ModuleCount, OuterPadding, OutputProfile,
    PixelCount, PixelDimensions, ProfileError, SymbolGeometry, Version,
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
    pub const BRAND: Self = Self::opaque(189, 15, 114);
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
    Transparent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Foreground {
    Brand,
}

impl Foreground {
    #[must_use]
    pub const fn rgba(self) -> Rgba {
        match self {
            Self::Brand => Rgba::BRAND,
        }
    }
}

pub const APPROVED_FOREGROUNDS: [Foreground; 1] = [Foreground::Brand];
pub const APPROVED_BACKGROUNDS: [Background; 2] =
    [Background::Opaque(Rgba::WHITE), Background::Transparent];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ContrastRatio(u16);

impl ContrastRatio {
    pub const MINIMUM_OPAQUE: Self = Self::from_hundredths(450);

    #[must_use]
    pub const fn from_hundredths(hundredths: u16) -> Self {
        Self(hundredths)
    }

    #[must_use]
    pub const fn hundredths(self) -> u16 {
        self.0
    }

    fn between(first: Rgba, second: Rgba) -> Self {
        let ratio = contrast_value(first, second);
        // Floor the displayed measurement so a rejected value just below the
        // threshold can never be presented as meeting that threshold.
        Self((ratio * 100.0).floor() as u16)
    }
}

fn contrast_value(first: Rgba, second: Rgba) -> f64 {
    let first_luminance = relative_luminance(first);
    let second_luminance = relative_luminance(second);
    let (lighter, darker) = if first_luminance >= second_luminance {
        (first_luminance, second_luminance)
    } else {
        (second_luminance, first_luminance)
    };
    (lighter + 0.05) / (darker + 0.05)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputSafety {
    Safe,
    Caution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataModuleStyle {
    Square,
    Rounded,
}

pub const APPROVED_DATA_MODULE_STYLES: [DataModuleStyle; 2] =
    [DataModuleStyle::Square, DataModuleStyle::Rounded];

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
    Bundled,
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
        Self::approved(profile, Foreground::Brand, Background::Opaque(Rgba::WHITE))
    }

    pub fn approved(
        profile: OutputProfile,
        foreground: Foreground,
        background: Background,
    ) -> Result<Self, RenderError> {
        Self::approved_with_data_style(profile, foreground, background, DataModuleStyle::Square)
    }

    pub fn approved_with_data_style(
        profile: OutputProfile,
        foreground: Foreground,
        background: Background,
        data_module_style: DataModuleStyle,
    ) -> Result<Self, RenderError> {
        Self::try_new_with_data_style(profile, foreground.rgba(), background, data_module_style)
    }

    pub fn try_new(
        profile: OutputProfile,
        foreground: Rgba,
        background: Background,
    ) -> Result<Self, RenderError> {
        Self::try_new_with_data_style(profile, foreground, background, DataModuleStyle::Square)
    }

    pub fn try_new_with_data_style(
        profile: OutputProfile,
        foreground: Rgba,
        background: Background,
        data_module_style: DataModuleStyle,
    ) -> Result<Self, RenderError> {
        profile.validate().map_err(RenderError::InvalidProfile)?;
        if let Background::Opaque(background_color) = background {
            let actual = ContrastRatio::between(foreground, background_color);
            if contrast_value(foreground, background_color) < 4.5 {
                return Err(RenderError::UnsafeContrast {
                    actual,
                    minimum: ContrastRatio::MINIMUM_OPAQUE,
                });
            }
        }
        if !APPROVED_FOREGROUNDS
            .into_iter()
            .any(|approved| approved.rgba() == foreground)
            || !APPROVED_BACKGROUNDS.contains(&background)
        {
            return Err(RenderError::UnapprovedColorCombination);
        }
        Ok(Self {
            profile,
            foreground,
            background,
            data_module_style,
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

    pub fn with_logo(mut self, logo_style: LogoStyle) -> Result<Self, RenderError> {
        if logo_style == LogoStyle::Bundled && self.background != Background::Opaque(Rgba::WHITE) {
            return Err(RenderError::LogoRequiresOpaqueWhite);
        }
        self.logo_style = logo_style;
        Ok(self)
    }

    #[must_use]
    pub const fn safety(self) -> OutputSafety {
        match (self.logo_style, self.background) {
            (LogoStyle::Bundled, _) | (_, Background::Transparent) => OutputSafety::Caution,
            (LogoStyle::None, Background::Opaque(_)) => OutputSafety::Safe,
        }
    }

    #[must_use]
    pub fn contrast_ratio(self) -> Option<ContrastRatio> {
        match self.background {
            Background::Opaque(background) => {
                Some(ContrastRatio::between(self.foreground, background))
            }
            Background::Transparent => None,
        }
    }
}

fn relative_luminance(color: Rgba) -> f64 {
    let [red, green, blue, _] = color.channels();
    0.2126 * linear_channel(red) + 0.7152 * linear_channel(green) + 0.0722 * linear_channel(blue)
}

fn linear_channel(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
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
    logo_placement: Option<LogoPlacement>,
}

impl<'encoded> RenderModel<'encoded> {
    pub fn new(encoded: &'encoded EncodedQr, options: RenderOptions) -> Result<Self, RenderError> {
        if options.logo_style() == LogoStyle::Bundled && encoded.ecc() != ErrorCorrection::High {
            return Err(RenderError::LogoRequiresHighEcc);
        }
        let logo_placement = match options.logo_style() {
            LogoStyle::None => None,
            LogoStyle::Bundled => Some(calculate_logo_placement(encoded.modules())?),
        };
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
            logo_placement,
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

    #[must_use]
    pub const fn logo_placement(&self) -> Option<LogoPlacement> {
        self.logo_placement
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
    UnsafeContrast {
        actual: ContrastRatio,
        minimum: ContrastRatio,
    },
    UnapprovedColorCombination,
    LogoRequiresHighEcc,
    LogoRequiresOpaqueWhite,
    UnsafeLogoGeometry,
    RenderFailure,
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
            Self::UnsafeContrast { actual, minimum } => write!(
                formatter,
                "opaque foreground/background contrast is {}.{:02}:1; minimum is {}.{:02}:1",
                actual.hundredths() / 100,
                actual.hundredths() % 100,
                minimum.hundredths() / 100,
                minimum.hundredths() % 100,
            ),
            Self::UnapprovedColorCombination => {
                formatter.write_str("the foreground/background combination is not approved")
            }
            Self::LogoRequiresHighEcc => formatter.write_str("the bundled logo requires ECC H"),
            Self::LogoRequiresOpaqueWhite => {
                formatter.write_str("the bundled logo requires an opaque white background")
            }
            Self::UnsafeLogoGeometry => formatter
                .write_str("the bundled logo cannot avoid protected modules at this QR version"),
            Self::RenderFailure => formatter.write_str("rendering failed"),
        }
    }
}

impl Error for RenderError {}
