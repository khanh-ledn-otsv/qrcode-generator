//! QR Code Model 2 capacity and version tables.
//!
//! ISO/IEC 18004:2024 defines the corresponding behavior in 5.3.2 (versions
//! and sizes), 7.4 (data encoding), 7.5.1 (error-correction capacity), 7.6
//! (final message construction), and Annex E (alignment positions). The values
//! are cross-validated by `tests/fixtures/qr_tables.csv`, generated from
//! independently maintained `qrcodegen` 1.8.0 and `python-qrcode` 8.2
//! development oracles.

use crate::{Version, VersionError};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

impl ErrorCorrection {
    const fn table_index(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::Quartile => 2,
            Self::High => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataMode {
    Numeric,
    Alphanumeric,
    Byte,
    Kanji,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockGroup {
    block_count: u8,
    data_codewords_per_block: u16,
}

impl BlockGroup {
    #[must_use]
    pub const fn block_count(self) -> u8 {
        self.block_count
    }

    #[must_use]
    pub const fn data_codewords_per_block(self) -> u16 {
        self.data_codewords_per_block
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QrTableRow {
    version: Version,
    error_correction: ErrorCorrection,
    total_codewords: u16,
    data_codewords: u16,
    ecc_codewords_per_block: u8,
    groups: [Option<BlockGroup>; 2],
    remainder_bits: u8,
}

impl QrTableRow {
    #[must_use]
    pub const fn version(self) -> Version {
        self.version
    }

    #[must_use]
    pub const fn error_correction(self) -> ErrorCorrection {
        self.error_correction
    }

    #[must_use]
    pub const fn total_codewords(self) -> u16 {
        self.total_codewords
    }

    #[must_use]
    pub const fn data_codewords(self) -> u16 {
        self.data_codewords
    }

    #[must_use]
    pub const fn ecc_codewords(self) -> u16 {
        self.total_codewords - self.data_codewords
    }

    #[must_use]
    pub const fn ecc_codewords_per_block(self) -> u8 {
        self.ecc_codewords_per_block
    }

    pub fn block_groups(self) -> impl Iterator<Item = BlockGroup> {
        self.groups.into_iter().flatten()
    }

    #[must_use]
    pub const fn remainder_bits(self) -> u8 {
        self.remainder_bits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionInfo {
    version: Version,
    total_codewords: u16,
    remainder_bits: u8,
    alignment_pattern_centers: &'static [u8],
}

impl VersionInfo {
    #[must_use]
    pub const fn version(self) -> Version {
        self.version
    }

    #[must_use]
    pub fn symbol_size(self) -> u16 {
        self.version.symbol_size()
    }

    #[must_use]
    pub const fn total_codewords(self) -> u16 {
        self.total_codewords
    }

    #[must_use]
    pub const fn remainder_bits(self) -> u8 {
        self.remainder_bits
    }

    #[must_use]
    pub const fn alignment_pattern_centers(self) -> &'static [u8] {
        self.alignment_pattern_centers
    }

    /// Returns alignment-pattern centers that do not overlap a finder pattern.
    pub fn alignment_pattern_positions(self) -> impl Iterator<Item = (u8, u8)> {
        let centers = self.alignment_pattern_centers;
        let final_center = centers.last().copied();
        centers
            .iter()
            .copied()
            .flat_map(move |y| centers.iter().copied().map(move |x| (x, y)))
            .filter(move |&(x, y)| {
                final_center.is_some_and(|last| {
                    !((x == 6 && (y == 6 || y == last)) || (x == last && y == 6))
                })
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableLookupError {
    InvalidVersion(VersionError),
    MissingRow {
        version: Version,
        error_correction: ErrorCorrection,
    },
    MissingVersionData {
        version: Version,
    },
    InconsistentRow {
        version: Version,
        error_correction: ErrorCorrection,
    },
}

impl fmt::Display for TableLookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion(error) => error.fmt(formatter),
            Self::MissingRow {
                version,
                error_correction,
            } => write!(
                formatter,
                "missing QR table row for version {} and {error_correction:?}",
                version.number()
            ),
            Self::MissingVersionData { version } => {
                write!(
                    formatter,
                    "missing QR version data for {}",
                    version.number()
                )
            }
            Self::InconsistentRow {
                version,
                error_correction,
            } => write!(
                formatter,
                "inconsistent QR table row for version {} and {error_correction:?}",
                version.number()
            ),
        }
    }
}

impl Error for TableLookupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidVersion(error) => Some(error),
            Self::MissingRow { .. }
            | Self::MissingVersionData { .. }
            | Self::InconsistentRow { .. } => None,
        }
    }
}

impl From<VersionError> for TableLookupError {
    fn from(error: VersionError) -> Self {
        Self::InvalidVersion(error)
    }
}

pub fn lookup(
    version: Version,
    error_correction: ErrorCorrection,
) -> Result<QrTableRow, TableLookupError> {
    let info = version_info_for(version)?;
    let index = usize::from(version.number() - 1);
    let ecc_per_block = table_value(&ECC_CODEWORDS_PER_BLOCK, index, version, error_correction)?;
    let block_count = table_value(&ERROR_CORRECTION_BLOCKS, index, version, error_correction)?;
    expand_row(version, error_correction, info, ecc_per_block, block_count)
}

pub fn lookup_version_number(
    version_number: u8,
    error_correction: ErrorCorrection,
) -> Result<QrTableRow, TableLookupError> {
    lookup(Version::new(version_number)?, error_correction)
}

pub fn version_info(version: Version) -> Result<VersionInfo, TableLookupError> {
    version_info_for(version)
}

pub fn version_info_for_number(version_number: u8) -> Result<VersionInfo, TableLookupError> {
    version_info(Version::new(version_number)?)
}

#[must_use]
pub fn character_count_bits(version: Version, mode: DataMode) -> u8 {
    let version = version.number();
    let band = if version <= 9 {
        0
    } else if version <= 26 {
        1
    } else {
        2
    };
    match (mode, band) {
        (DataMode::Numeric, 0) => 10,
        (DataMode::Numeric, 1) => 12,
        (DataMode::Numeric, _) => 14,
        (DataMode::Alphanumeric, 0) => 9,
        (DataMode::Alphanumeric, 1) => 11,
        (DataMode::Alphanumeric, _) => 13,
        (DataMode::Byte, 0) => 8,
        (DataMode::Byte, _) => 16,
        (DataMode::Kanji, 0) => 8,
        (DataMode::Kanji, 1) => 10,
        (DataMode::Kanji, _) => 12,
    }
}

pub fn character_count_bits_for_version_number(
    version_number: u8,
    mode: DataMode,
) -> Result<u8, TableLookupError> {
    Ok(character_count_bits(Version::new(version_number)?, mode))
}

fn version_info_for(version: Version) -> Result<VersionInfo, TableLookupError> {
    let index = usize::from(version.number() - 1);
    let total_codewords = *TOTAL_CODEWORDS
        .get(index)
        .ok_or(TableLookupError::MissingVersionData { version })?;
    let remainder_bits = *REMAINDER_BITS
        .get(index)
        .ok_or(TableLookupError::MissingVersionData { version })?;
    let alignment_pattern_centers = *ALIGNMENT_PATTERN_CENTERS
        .get(index)
        .ok_or(TableLookupError::MissingVersionData { version })?;
    Ok(VersionInfo {
        version,
        total_codewords,
        remainder_bits,
        alignment_pattern_centers,
    })
}

fn table_value(
    table: &[[u8; 40]; 4],
    version_index: usize,
    version: Version,
    error_correction: ErrorCorrection,
) -> Result<u8, TableLookupError> {
    table
        .get(error_correction.table_index())
        .and_then(|row| row.get(version_index))
        .copied()
        .ok_or(TableLookupError::MissingRow {
            version,
            error_correction,
        })
}

fn expand_row(
    version: Version,
    error_correction: ErrorCorrection,
    info: VersionInfo,
    ecc_per_block: u8,
    block_count: u8,
) -> Result<QrTableRow, TableLookupError> {
    let inconsistent = || TableLookupError::InconsistentRow {
        version,
        error_correction,
    };
    if block_count == 0 {
        return Err(inconsistent());
    }
    let block_count_u16 = u16::from(block_count);
    let ecc_codewords = u16::from(ecc_per_block)
        .checked_mul(block_count_u16)
        .ok_or_else(inconsistent)?;
    let data_codewords = info
        .total_codewords
        .checked_sub(ecc_codewords)
        .ok_or_else(inconsistent)?;
    let short_total = info.total_codewords / block_count_u16;
    let long_count =
        u8::try_from(info.total_codewords % block_count_u16).map_err(|_| inconsistent())?;
    let short_count = block_count
        .checked_sub(long_count)
        .ok_or_else(inconsistent)?;
    let short_data = short_total
        .checked_sub(u16::from(ecc_per_block))
        .ok_or_else(inconsistent)?;
    let first = BlockGroup {
        block_count: short_count,
        data_codewords_per_block: short_data,
    };
    let second = if long_count == 0 {
        None
    } else {
        Some(BlockGroup {
            block_count: long_count,
            data_codewords_per_block: short_data.checked_add(1).ok_or_else(inconsistent)?,
        })
    };
    Ok(QrTableRow {
        version,
        error_correction,
        total_codewords: info.total_codewords,
        data_codewords,
        ecc_codewords_per_block: ecc_per_block,
        groups: [Some(first), second],
        remainder_bits: info.remainder_bits,
    })
}

// Compact production data validated exhaustively against the committed
// dual-oracle fixture; group sizes are derived with checked arithmetic.
const TOTAL_CODEWORDS: [u16; 40] = [
    26, 44, 70, 100, 134, 172, 196, 242, 292, 346, 404, 466, 532, 581, 655, 733, 815, 901, 991,
    1085, 1156, 1258, 1364, 1474, 1588, 1706, 1828, 1921, 2051, 2185, 2323, 2465, 2611, 2761, 2876,
    3034, 3196, 3362, 3532, 3706,
];

const REMAINDER_BITS: [u8; 40] = [
    0, 7, 7, 7, 7, 7, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3,
    3, 3, 0, 0, 0, 0, 0, 0,
];

const ALIGNMENT_PATTERN_CENTERS: [&[u8]; 40] = [
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
    &[6, 30, 54],
    &[6, 32, 58],
    &[6, 34, 62],
    &[6, 26, 46, 66],
    &[6, 26, 48, 70],
    &[6, 26, 50, 74],
    &[6, 30, 54, 78],
    &[6, 30, 56, 82],
    &[6, 30, 58, 86],
    &[6, 34, 62, 90],
    &[6, 28, 50, 72, 94],
    &[6, 26, 50, 74, 98],
    &[6, 30, 54, 78, 102],
    &[6, 28, 54, 80, 106],
    &[6, 32, 58, 84, 110],
    &[6, 30, 58, 86, 114],
    &[6, 34, 62, 90, 118],
    &[6, 26, 50, 74, 98, 122],
    &[6, 30, 54, 78, 102, 126],
    &[6, 26, 52, 78, 104, 130],
    &[6, 30, 56, 82, 108, 134],
    &[6, 34, 60, 86, 112, 138],
    &[6, 30, 58, 86, 114, 142],
    &[6, 34, 62, 90, 118, 146],
    &[6, 30, 54, 78, 102, 126, 150],
    &[6, 24, 50, 76, 102, 128, 154],
    &[6, 28, 54, 80, 106, 132, 158],
    &[6, 32, 58, 84, 110, 136, 162],
    &[6, 26, 54, 82, 110, 138, 166],
    &[6, 30, 58, 86, 114, 142, 170],
];

const ECC_CODEWORDS_PER_BLOCK: [[u8; 40]; 4] = [
    [
        7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30,
        30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
    [
        10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    ],
    [
        13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30,
        30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
    [
        17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30,
        30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
];

const ERROR_CORRECTION_BLOCKS: [[u8; 40]; 4] = [
    [
        1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13,
        14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25,
    ],
    [
        1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23,
        25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49,
    ],
    [
        1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29,
        34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68,
    ],
    [
        1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35,
        37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81,
    ],
];
