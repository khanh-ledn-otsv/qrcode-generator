//! QR data-block construction and final codeword interleaving.
//!
//! ISO/IEC 18004:2024 final-message construction.
//! 2024 clause mapping pending audit.
//! Block splitting and interleaving are public-corroborated, non-normative
//! evidence from Nayuki QR Code Generator 1.8.0
//! (`add_ecc_and_interleave`) and python-qrcode 8.2 (`create_bytes`). See
//! `docs/research/qr-public-source-provenance.md` and the accepted
//! `qr-interleaved-codeword-vectors` entry in `tests/fixtures/manifest.json`.

use crate::Version;
use crate::reed_solomon::{
    ErrorCorrectionCodewordCount, ReedSolomonError, generate_error_correction,
};
use crate::tables::{ErrorCorrection, TableLookupError, lookup};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodewordStreamRequest<'a> {
    pub version: Version,
    pub ecc: ErrorCorrection,
    pub data_codewords: &'a [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterleavedCodewords {
    version: Version,
    ecc: ErrorCorrection,
    codewords: Vec<u8>,
    remainder_bit_count: u8,
}

impl InterleavedCodewords {
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    #[must_use]
    pub const fn error_correction(&self) -> ErrorCorrection {
        self.ecc
    }

    #[must_use]
    pub fn codewords(&self) -> &[u8] {
        &self.codewords
    }

    #[must_use]
    pub const fn remainder_bit_count(&self) -> u8 {
        self.remainder_bit_count
    }
}

pub fn construct(
    request: CodewordStreamRequest<'_>,
) -> Result<InterleavedCodewords, CodewordStreamError> {
    let row = lookup(request.version, request.ecc)?;
    let expected_data_codewords = usize::from(row.data_codewords());
    if request.data_codewords.len() != expected_data_codewords {
        return Err(CodewordStreamError::DataLengthMismatch {
            expected: expected_data_codewords,
            actual: request.data_codewords.len(),
        });
    }

    let ecc_count = ErrorCorrectionCodewordCount::new(row.ecc_codewords_per_block())?;
    let mut blocks = Vec::new();
    let mut offset = 0_usize;
    for group in row.block_groups() {
        let data_length = usize::from(group.data_codewords_per_block());
        for _ in 0..group.block_count() {
            let end = offset
                .checked_add(data_length)
                .ok_or(CodewordStreamError::LengthOverflow)?;
            let data = request.data_codewords.get(offset..end).ok_or(
                CodewordStreamError::InconsistentBlockLayout {
                    expected: expected_data_codewords,
                    consumed: offset,
                },
            )?;
            blocks.push(DataBlock {
                data,
                error_correction: generate_error_correction(data, ecc_count)?,
            });
            offset = end;
        }
    }
    if offset != expected_data_codewords {
        return Err(CodewordStreamError::InconsistentBlockLayout {
            expected: expected_data_codewords,
            consumed: offset,
        });
    }

    let mut codewords = Vec::with_capacity(usize::from(row.total_codewords()));
    let maximum_data_length = blocks.iter().map(|block| block.data.len()).max().ok_or(
        CodewordStreamError::InconsistentBlockLayout {
            expected: expected_data_codewords,
            consumed: 0,
        },
    )?;
    for index in 0..maximum_data_length {
        for block in &blocks {
            if let Some(&codeword) = block.data.get(index) {
                codewords.push(codeword);
            }
        }
    }
    for index in 0..usize::from(ecc_count.number()) {
        for block in &blocks {
            let codeword = block.error_correction.get(index).copied().ok_or(
                CodewordStreamError::InconsistentErrorCorrectionLength {
                    expected: usize::from(ecc_count.number()),
                    actual: block.error_correction.len(),
                },
            )?;
            codewords.push(codeword);
        }
    }

    let expected_total = usize::from(row.total_codewords());
    if codewords.len() != expected_total {
        return Err(CodewordStreamError::InconsistentStreamLength {
            expected: expected_total,
            actual: codewords.len(),
        });
    }
    Ok(InterleavedCodewords {
        version: request.version,
        ecc: request.ecc,
        codewords,
        remainder_bit_count: row.remainder_bits(),
    })
}

struct DataBlock<'a> {
    data: &'a [u8],
    error_correction: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodewordStreamError {
    Table(TableLookupError),
    ReedSolomon(ReedSolomonError),
    DataLengthMismatch { expected: usize, actual: usize },
    LengthOverflow,
    InconsistentBlockLayout { expected: usize, consumed: usize },
    InconsistentErrorCorrectionLength { expected: usize, actual: usize },
    InconsistentStreamLength { expected: usize, actual: usize },
}

impl fmt::Display for CodewordStreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table(error) => error.fmt(formatter),
            Self::ReedSolomon(error) => error.fmt(formatter),
            Self::DataLengthMismatch { expected, actual } => write!(
                formatter,
                "QR block construction requires {expected} data codewords, got {actual}"
            ),
            Self::LengthOverflow => write!(formatter, "QR block length arithmetic overflowed"),
            Self::InconsistentBlockLayout { expected, consumed } => write!(
                formatter,
                "QR block layout declares {expected} data codewords but consumes {consumed}"
            ),
            Self::InconsistentErrorCorrectionLength { expected, actual } => write!(
                formatter,
                "QR block requires {expected} error-correction codewords, got {actual}"
            ),
            Self::InconsistentStreamLength { expected, actual } => write!(
                formatter,
                "QR interleaved stream requires {expected} codewords, got {actual}"
            ),
        }
    }
}

impl Error for CodewordStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Table(error) => Some(error),
            Self::ReedSolomon(error) => Some(error),
            Self::DataLengthMismatch { .. }
            | Self::LengthOverflow
            | Self::InconsistentBlockLayout { .. }
            | Self::InconsistentErrorCorrectionLength { .. }
            | Self::InconsistentStreamLength { .. } => None,
        }
    }
}

impl From<TableLookupError> for CodewordStreamError {
    fn from(error: TableLookupError) -> Self {
        Self::Table(error)
    }
}

impl From<ReedSolomonError> for CodewordStreamError {
    fn from(error: ReedSolomonError) -> Self {
        Self::ReedSolomon(error)
    }
}
