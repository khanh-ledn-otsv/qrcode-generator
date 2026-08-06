//! Checked construction of classified QR module matrices.
//!
//! ISO/IEC 18004:2024 function-pattern placement and reservation; 2024
//! clause mapping pending audit. The placement rules are corroborated by the
//! pinned public encoders recorded by the `qr-classified-function-matrices`
//! fixture in `tests/fixtures/manifest.json`: Nayuki 1.8.0
//! `rust/src/lib.rs::{draw_function_patterns,draw_finder_pattern,
//! draw_alignment_pattern,set_function_module}` and python-qrcode 8.2
//! `qrcode/main.py::{setup_position_probe_pattern,
//! setup_position_adjust_pattern,setup_timing_pattern,setup_type_info,
//! setup_type_number}`. This evidence is `public-corroborated, non-normative`
//! until that audit is complete.

use crate::Version;
use crate::tables::{self, TableLookupError};
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
    size: u16,
    modules: Vec<Module>,
}

impl ModuleMatrix {
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
            size: self.size,
            modules,
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
