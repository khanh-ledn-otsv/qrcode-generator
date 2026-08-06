//! Checked construction of classified QR module matrices.
//!
//! ISO/IEC 18004:2024 function-pattern placement and reservation; 2024
//! clause mapping pending audit. The placement rules are corroborated by the
//! pinned public encoders recorded by the `qr-classified-function-matrices`
//! fixture in `tests/fixtures/manifest.json`: Nayuki 1.8.0
//! `rust/src/lib.rs::{draw_function_patterns,draw_finder_pattern,
//! draw_alignment_pattern,set_function_module,draw_format_bits,draw_version}`
//! and python-qrcode 8.2
//! `qrcode/main.py::{setup_position_probe_pattern,
//! setup_position_adjust_pattern,setup_timing_pattern,setup_type_info,
//! setup_type_number}`. Data placement and explicit masking are corroborated
//! by Nayuki's `draw_codewords`/`apply_mask` and python-qrcode's
//! `map_data`/`mask_func`. This evidence is `public-corroborated,
//! non-normative` until that audit is complete.

use crate::Version;
use crate::bch::{format_bits, version_bits};
use crate::codeword_stream::InterleavedCodewords;
use crate::tables::{self, ErrorCorrection, TableLookupError};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleKind {
    Data,
    Remainder,
    Finder,
    Separator,
    Timing,
    Alignment,
    Format,
    Version,
    Dark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Module {
    dark: bool,
    kind: ModuleKind,
}

impl Module {
    #[must_use]
    pub const fn new(dark: bool, kind: ModuleKind) -> Self {
        Self { dark, kind }
    }

    #[must_use]
    pub const fn is_dark(self) -> bool {
        self.dark
    }

    #[must_use]
    pub const fn kind(self) -> ModuleKind {
        self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleMatrix {
    version: Version,
    size: u16,
    modules: Vec<Module>,
    data_placed: bool,
    information_finalized: bool,
}

impl ModuleMatrix {
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    #[must_use]
    pub const fn size(&self) -> u16 {
        self.size
    }

    #[must_use]
    pub fn module(&self, x: u16, y: u16) -> Option<Module> {
        checked_index(self.size, x, y).and_then(|index| self.modules.get(index).copied())
    }

    pub fn modules(&self) -> impl ExactSizeIterator<Item = Module> + '_ {
        self.modules.iter().copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatrixError {
    TableLookup(TableLookupError),
    DimensionOverflow { size: u16 },
    OutOfBounds { x: u16, y: u16, size: u16 },
    DoubleWrite { x: u16, y: u16 },
    InvalidReservation { x: u16, y: u16, kind: ModuleKind },
    Incomplete { unwritten: usize },
}

impl fmt::Display for MatrixError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TableLookup(error) => error.fmt(formatter),
            Self::DimensionOverflow { size } => {
                write!(
                    formatter,
                    "QR matrix dimension {size} cannot be represented"
                )
            }
            Self::OutOfBounds { x, y, size } => {
                write!(
                    formatter,
                    "matrix coordinate ({x}, {y}) is outside {size}×{size}"
                )
            }
            Self::DoubleWrite { x, y } => {
                write!(formatter, "matrix coordinate ({x}, {y}) was written twice")
            }
            Self::InvalidReservation { x, y, kind } => {
                write!(
                    formatter,
                    "module kind {kind:?} cannot be reserved at ({x}, {y})"
                )
            }
            Self::Incomplete { unwritten } => {
                write!(formatter, "matrix has {unwritten} unwritten modules")
            }
        }
    }
}

impl Error for MatrixError {}

impl From<TableLookupError> for MatrixError {
    fn from(error: TableLookupError) -> Self {
        Self::TableLookup(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatrixBuilder {
    version: Version,
    size: u16,
    modules: Vec<Option<Module>>,
}

impl MatrixBuilder {
    pub fn new(version: Version) -> Result<Self, MatrixError> {
        let size = version.symbol_size();
        let length = usize::from(size)
            .checked_mul(usize::from(size))
            .ok_or(MatrixError::DimensionOverflow { size })?;
        Ok(Self {
            version,
            size,
            modules: vec![None; length],
        })
    }

    #[must_use]
    pub const fn size(&self) -> u16 {
        self.size
    }

    pub fn write(&mut self, x: u16, y: u16, module: Module) -> Result<(), MatrixError> {
        if matches!(module.kind(), ModuleKind::Format | ModuleKind::Version) {
            return Err(MatrixError::InvalidReservation {
                x,
                y,
                kind: module.kind(),
            });
        }
        self.write_module(x, y, module)
    }

    fn write_module(&mut self, x: u16, y: u16, module: Module) -> Result<(), MatrixError> {
        let index = checked_index(self.size, x, y).ok_or(MatrixError::OutOfBounds {
            x,
            y,
            size: self.size,
        })?;
        let cell = self
            .modules
            .get_mut(index)
            .ok_or(MatrixError::OutOfBounds {
                x,
                y,
                size: self.size,
            })?;
        if cell.is_some() {
            return Err(MatrixError::DoubleWrite { x, y });
        }
        *cell = Some(module);
        Ok(())
    }

    pub fn reserve(&mut self, x: u16, y: u16, kind: ModuleKind) -> Result<(), MatrixError> {
        if !is_valid_reservation(self.size, x, y, kind) {
            return Err(MatrixError::InvalidReservation { x, y, kind });
        }
        self.write_module(x, y, Module::new(false, kind))
    }

    pub fn finish(self) -> Result<ModuleMatrix, MatrixError> {
        let unwritten = self
            .modules
            .iter()
            .filter(|module| module.is_none())
            .count();
        if unwritten != 0 {
            return Err(MatrixError::Incomplete { unwritten });
        }
        let modules = self.modules.into_iter().flatten().collect();
        Ok(ModuleMatrix {
            version: self.version,
            size: self.size,
            modules,
            data_placed: false,
            information_finalized: false,
        })
    }

    fn fill_unwritten(&mut self, module: Module) {
        for cell in &mut self.modules {
            if cell.is_none() {
                *cell = Some(module);
            }
        }
    }

    fn is_written(&self, x: u16, y: u16) -> Result<bool, MatrixError> {
        let index = checked_index(self.size, x, y).ok_or(MatrixError::OutOfBounds {
            x,
            y,
            size: self.size,
        })?;
        self.modules
            .get(index)
            .map(Option::is_some)
            .ok_or(MatrixError::OutOfBounds {
                x,
                y,
                size: self.size,
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaskId(u8);

impl MaskId {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 7;

    pub const fn new(number: u8) -> Result<Self, MaskError> {
        if number > Self::MAX {
            return Err(MaskError { number });
        }
        Ok(Self(number))
    }

    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }

    #[must_use]
    pub fn applies(self, x: u16, y: u16) -> bool {
        let x = u32::from(x);
        let y = u32::from(y);
        let product = x * y;
        match self.0 {
            0 => (x + y) % 2 == 0,
            1 => y % 2 == 0,
            2 => x % 3 == 0,
            3 => (x + y) % 3 == 0,
            4 => (y / 2 + x / 3) % 2 == 0,
            5 => product % 2 + product % 3 == 0,
            6 => (product % 2 + product % 3) % 2 == 0,
            7 => ((x + y) % 2 + product % 3) % 2 == 0,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaskError {
    number: u8,
}

impl MaskError {
    #[must_use]
    pub const fn number(self) -> u8 {
        self.number
    }
}

impl fmt::Display for MaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "QR mask must be between {} and {}, got {}",
            MaskId::MIN,
            MaskId::MAX,
            self.number
        )
    }
}

impl Error for MaskError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementError {
    Matrix(MatrixError),
    AlreadyPlaced,
    OwnershipMismatch {
        x: u16,
        y: u16,
        expected: Module,
        actual: Module,
    },
    LengthOverflow,
    StreamLengthMismatch {
        writable_modules: usize,
        data_bits: usize,
        remainder_bits: usize,
    },
    TraversalIncomplete {
        expected: usize,
        placed: usize,
    },
}

impl fmt::Display for PlacementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Matrix(error) => error.fmt(formatter),
            Self::AlreadyPlaced => formatter.write_str("QR matrix data has already been placed"),
            Self::OwnershipMismatch {
                x,
                y,
                expected,
                actual,
            } => write!(
                formatter,
                "matrix ownership mismatch at ({x}, {y}): expected {expected:?}, got {actual:?}"
            ),
            Self::LengthOverflow => formatter.write_str("QR placement bit length overflowed"),
            Self::StreamLengthMismatch {
                writable_modules,
                data_bits,
                remainder_bits,
            } => write!(
                formatter,
                "matrix has {writable_modules} writable modules but stream requires {data_bits} data bits and {remainder_bits} remainder bits"
            ),
            Self::TraversalIncomplete { expected, placed } => write!(
                formatter,
                "matrix traversal expected {expected} writable modules but placed {placed}"
            ),
        }
    }
}

impl Error for PlacementError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Matrix(error) => Some(error),
            Self::AlreadyPlaced
            | Self::OwnershipMismatch { .. }
            | Self::LengthOverflow
            | Self::StreamLengthMismatch { .. }
            | Self::TraversalIncomplete { .. } => None,
        }
    }
}

impl From<MatrixError> for PlacementError {
    fn from(error: MatrixError) -> Self {
        Self::Matrix(error)
    }
}

pub fn place_data(
    mut matrix: ModuleMatrix,
    stream: &InterleavedCodewords,
    mask: MaskId,
) -> Result<ModuleMatrix, PlacementError> {
    if matrix.data_placed {
        return Err(PlacementError::AlreadyPlaced);
    }
    let expected_matrix = build_function_matrix(matrix.version)?;
    for (index, (expected, actual)) in expected_matrix
        .modules
        .iter()
        .copied()
        .zip(matrix.modules.iter().copied())
        .enumerate()
    {
        if expected != actual {
            let size = usize::from(matrix.size);
            let x = u16::try_from(index % size).map_err(|_| PlacementError::LengthOverflow)?;
            let y = u16::try_from(index / size).map_err(|_| PlacementError::LengthOverflow)?;
            return Err(PlacementError::OwnershipMismatch {
                x,
                y,
                expected,
                actual,
            });
        }
    }

    let data_bits = stream
        .codewords()
        .len()
        .checked_mul(8)
        .ok_or(PlacementError::LengthOverflow)?;
    let remainder_bits = usize::from(stream.remainder_bit_count());
    let required_modules = data_bits
        .checked_add(remainder_bits)
        .ok_or(PlacementError::LengthOverflow)?;
    let writable_modules = matrix
        .modules
        .iter()
        .filter(|module| module.kind() == ModuleKind::Data)
        .count();
    if writable_modules != required_modules {
        return Err(PlacementError::StreamLengthMismatch {
            writable_modules,
            data_bits,
            remainder_bits,
        });
    }

    let mut placed = 0_usize;
    let size = matrix.size;
    let mut right = size - 1;
    let mut upward = true;
    loop {
        if right == 6 {
            right = 5;
        }
        for step in 0..size {
            let y = if upward { size - 1 - step } else { step };
            for x in [right, right - 1] {
                let index =
                    checked_index(size, x, y).ok_or(PlacementError::TraversalIncomplete {
                        expected: required_modules,
                        placed,
                    })?;
                let module =
                    matrix
                        .modules
                        .get_mut(index)
                        .ok_or(PlacementError::TraversalIncomplete {
                            expected: required_modules,
                            placed,
                        })?;
                if module.kind() != ModuleKind::Data {
                    continue;
                }
                let (dark, kind) = if placed < data_bits {
                    let byte = stream.codewords().get(placed / 8).copied().ok_or(
                        PlacementError::TraversalIncomplete {
                            expected: required_modules,
                            placed,
                        },
                    )?;
                    let bit = (byte >> (7 - placed % 8)) & 1 != 0;
                    (bit ^ mask.applies(x, y), ModuleKind::Data)
                } else {
                    (mask.applies(x, y), ModuleKind::Remainder)
                };
                *module = Module::new(dark, kind);
                placed = placed
                    .checked_add(1)
                    .ok_or(PlacementError::LengthOverflow)?;
            }
        }
        if right < 2 {
            break;
        }
        right -= 2;
        upward = !upward;
    }
    if placed != required_modules {
        return Err(PlacementError::TraversalIncomplete {
            expected: required_modules,
            placed,
        });
    }
    matrix.data_placed = true;
    Ok(matrix)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InformationError {
    DataNotPlaced,
    AlreadyFinalized,
    OutOfBounds {
        x: u16,
        y: u16,
    },
    OwnershipMismatch {
        x: u16,
        y: u16,
        expected: ModuleKind,
        actual: ModuleKind,
    },
}

impl fmt::Display for InformationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataNotPlaced => {
                formatter.write_str("QR information cannot be finalized before data placement")
            }
            Self::AlreadyFinalized => formatter.write_str("QR information is already finalized"),
            Self::OutOfBounds { x, y } => {
                write!(
                    formatter,
                    "QR information coordinate ({x}, {y}) is out of bounds"
                )
            }
            Self::OwnershipMismatch {
                x,
                y,
                expected,
                actual,
            } => write!(
                formatter,
                "QR information coordinate ({x}, {y}) requires {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl Error for InformationError {}

pub fn finalize_information(
    mut matrix: ModuleMatrix,
    ecc: ErrorCorrection,
    mask: MaskId,
) -> Result<ModuleMatrix, InformationError> {
    if !matrix.data_placed {
        return Err(InformationError::DataNotPlaced);
    }
    if matrix.information_finalized {
        return Err(InformationError::AlreadyFinalized);
    }

    let size = matrix.size;
    let format = format_bits(ecc, mask);
    for bit in 0..15_u16 {
        let dark = ((format >> bit) & 1) != 0;
        let primary = match bit {
            0..=5 => (8, bit),
            6 => (8, 7),
            7 => (8, 8),
            8 => (7, 8),
            9..=14 => (14 - bit, 8),
            _ => continue,
        };
        let secondary = if bit <= 7 {
            (size - 1 - bit, 8)
        } else {
            (8, size - 15 + bit)
        };
        write_information_module(&mut matrix, primary.0, primary.1, dark, ModuleKind::Format)?;
        write_information_module(
            &mut matrix,
            secondary.0,
            secondary.1,
            dark,
            ModuleKind::Format,
        )?;
    }

    if let Some(version) = version_bits(matrix.version) {
        let start = size - 11;
        for bit in 0..18_u16 {
            let a = start + bit % 3;
            let b = bit / 3;
            let dark = ((version >> bit) & 1) != 0;
            write_information_module(&mut matrix, a, b, dark, ModuleKind::Version)?;
            write_information_module(&mut matrix, b, a, dark, ModuleKind::Version)?;
        }
    }
    matrix.information_finalized = true;
    Ok(matrix)
}

fn write_information_module(
    matrix: &mut ModuleMatrix,
    x: u16,
    y: u16,
    dark: bool,
    kind: ModuleKind,
) -> Result<(), InformationError> {
    let index = checked_index(matrix.size, x, y).ok_or(InformationError::OutOfBounds { x, y })?;
    let module = matrix
        .modules
        .get_mut(index)
        .ok_or(InformationError::OutOfBounds { x, y })?;
    if module.kind() != kind {
        return Err(InformationError::OwnershipMismatch {
            x,
            y,
            expected: kind,
            actual: module.kind(),
        });
    }
    *module = Module::new(dark, kind);
    Ok(())
}

pub fn build_function_matrix(version: Version) -> Result<ModuleMatrix, MatrixError> {
    let mut builder = MatrixBuilder::new(version)?;
    let size = builder.size();

    write_finder(&mut builder, 0, 0)?;
    write_finder(&mut builder, size - 7, 0)?;
    write_finder(&mut builder, 0, size - 7)?;
    write_separators(&mut builder)?;
    let version_info = tables::version_info(version)?;
    for (center_x, center_y) in version_info.alignment_pattern_positions() {
        write_alignment(&mut builder, u16::from(center_x), u16::from(center_y))?;
    }
    write_timing_patterns(&mut builder)?;
    reserve_format_regions(&mut builder)?;
    if version.number() >= 7 {
        reserve_version_regions(&mut builder)?;
    }
    builder.write(8, size - 8, Module::new(true, ModuleKind::Dark))?;
    builder.fill_unwritten(Module::new(false, ModuleKind::Data));
    builder.finish()
}

fn checked_index(size: u16, x: u16, y: u16) -> Option<usize> {
    if x >= size || y >= size {
        return None;
    }
    usize::from(y)
        .checked_mul(usize::from(size))
        .and_then(|row| row.checked_add(usize::from(x)))
}

fn is_valid_reservation(size: u16, x: u16, y: u16, kind: ModuleKind) -> bool {
    match kind {
        ModuleKind::Format => {
            (x == 8 && (y <= 5 || y == 7 || y == 8 || y >= size - 7))
                || (y == 8 && (x <= 5 || x == 7 || x >= size - 8))
        }
        ModuleKind::Version if size >= 45 => {
            let start = size - 11;
            ((start..=start + 2).contains(&x) && y <= 5)
                || ((start..=start + 2).contains(&y) && x <= 5)
        }
        ModuleKind::Data
        | ModuleKind::Remainder
        | ModuleKind::Finder
        | ModuleKind::Separator
        | ModuleKind::Timing
        | ModuleKind::Alignment
        | ModuleKind::Version
        | ModuleKind::Dark => false,
    }
}

fn write_finder(
    builder: &mut MatrixBuilder,
    origin_x: u16,
    origin_y: u16,
) -> Result<(), MatrixError> {
    for offset_y in 0..7 {
        for offset_x in 0..7 {
            let dark = offset_x == 0
                || offset_x == 6
                || offset_y == 0
                || offset_y == 6
                || ((2..=4).contains(&offset_x) && (2..=4).contains(&offset_y));
            builder.write(
                origin_x + offset_x,
                origin_y + offset_y,
                Module::new(dark, ModuleKind::Finder),
            )?;
        }
    }
    Ok(())
}

fn write_separators(builder: &mut MatrixBuilder) -> Result<(), MatrixError> {
    let size = builder.size();
    for offset in 0..8 {
        builder.write(7, offset, Module::new(false, ModuleKind::Separator))?;
        builder.write(size - 8, offset, Module::new(false, ModuleKind::Separator))?;
        builder.write(
            7,
            size - 1 - offset,
            Module::new(false, ModuleKind::Separator),
        )?;
    }
    for offset in 0..7 {
        builder.write(offset, 7, Module::new(false, ModuleKind::Separator))?;
        builder.write(
            size - 1 - offset,
            7,
            Module::new(false, ModuleKind::Separator),
        )?;
        builder.write(offset, size - 8, Module::new(false, ModuleKind::Separator))?;
    }
    Ok(())
}

fn write_timing_patterns(builder: &mut MatrixBuilder) -> Result<(), MatrixError> {
    for coordinate in 8..builder.size() - 8 {
        let module = Module::new(coordinate % 2 == 0, ModuleKind::Timing);
        if !builder.is_written(coordinate, 6)? {
            builder.write(coordinate, 6, module)?;
        }
        if !builder.is_written(6, coordinate)? {
            builder.write(6, coordinate, module)?;
        }
    }
    Ok(())
}

fn write_alignment(
    builder: &mut MatrixBuilder,
    center_x: u16,
    center_y: u16,
) -> Result<(), MatrixError> {
    let origin_x = center_x - 2;
    let origin_y = center_y - 2;
    for offset_y in 0..5 {
        for offset_x in 0..5 {
            let dark = offset_x == 0
                || offset_x == 4
                || offset_y == 0
                || offset_y == 4
                || (offset_x == 2 && offset_y == 2);
            builder.write(
                origin_x + offset_x,
                origin_y + offset_y,
                Module::new(dark, ModuleKind::Alignment),
            )?;
        }
    }
    Ok(())
}

fn reserve_format_regions(builder: &mut MatrixBuilder) -> Result<(), MatrixError> {
    let size = builder.size();
    for coordinate in 0..=5 {
        builder.reserve(8, coordinate, ModuleKind::Format)?;
        builder.reserve(coordinate, 8, ModuleKind::Format)?;
    }
    builder.reserve(8, 7, ModuleKind::Format)?;
    builder.reserve(8, 8, ModuleKind::Format)?;
    builder.reserve(7, 8, ModuleKind::Format)?;
    for offset in 0..8 {
        builder.reserve(size - 1 - offset, 8, ModuleKind::Format)?;
    }
    for offset in 0..7 {
        builder.reserve(8, size - 1 - offset, ModuleKind::Format)?;
    }
    Ok(())
}

fn reserve_version_regions(builder: &mut MatrixBuilder) -> Result<(), MatrixError> {
    let start = builder.size() - 11;
    for offset in 0..6 {
        for band in 0..3 {
            builder.reserve(start + band, offset, ModuleKind::Version)?;
            builder.reserve(offset, start + band, ModuleKind::Version)?;
        }
    }
    Ok(())
}
