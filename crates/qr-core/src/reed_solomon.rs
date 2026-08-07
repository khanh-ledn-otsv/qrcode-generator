//! QR Reed–Solomon arithmetic over GF(256).
//!
//! ISO/IEC 18004:2024 error-correction codeword generation, exact clause
//! mapping pending audit. The field polynomial, supported QR generator degrees,
//! and algorithm vectors are public-corroborated, non-normative evidence from
//! Nayuki QR Code Generator 1.8.0 (`reed_solomon_*`) and python-qrcode 8.2
//! (`Polynomial`, `gexp`, and `glog`). See
//! `docs/research/qr-public-source-provenance.md` and the accepted
//! `qr-reed-solomon-vectors` entry in `tests/fixtures/manifest.json`.

use std::error::Error;
use std::fmt;

/// Primitive polynomial used by QR Code's GF(256), including the x^8 term.
pub const PRIMITIVE_POLYNOMIAL: u16 = 0x11D;

/// Error-correction codeword counts used by the 160 QR version/ECC table rows.
pub const SUPPORTED_ECC_CODEWORD_COUNTS: [ErrorCorrectionCodewordCount; 13] = [
    ErrorCorrectionCodewordCount(7),
    ErrorCorrectionCodewordCount(10),
    ErrorCorrectionCodewordCount(13),
    ErrorCorrectionCodewordCount(15),
    ErrorCorrectionCodewordCount(16),
    ErrorCorrectionCodewordCount(17),
    ErrorCorrectionCodewordCount(18),
    ErrorCorrectionCodewordCount(20),
    ErrorCorrectionCodewordCount(22),
    ErrorCorrectionCodewordCount(24),
    ErrorCorrectionCodewordCount(26),
    ErrorCorrectionCodewordCount(28),
    ErrorCorrectionCodewordCount(30),
];

const EXPONENTS: [u8; 510] = exponent_table();
const LOGARITHMS: [u8; 256] = logarithm_table();

/// Multiplies two field elements.
#[must_use]
pub fn multiply(left: u8, right: u8) -> u8 {
    if left == 0 || right == 0 {
        return 0;
    }
    let exponent =
        usize::from(LOGARITHMS[usize::from(left)]) + usize::from(LOGARITHMS[usize::from(right)]);
    EXPONENTS[exponent]
}

/// Divides two field elements.
pub fn divide(dividend: u8, divisor: u8) -> Result<u8, ReedSolomonError> {
    if divisor == 0 {
        return Err(ReedSolomonError::DivisionByZero);
    }
    if dividend == 0 {
        return Ok(0);
    }
    let dividend_log = u16::from(LOGARITHMS[usize::from(dividend)]);
    let divisor_log = u16::from(LOGARITHMS[usize::from(divisor)]);
    let exponent = usize::from((dividend_log + 255 - divisor_log) % 255);
    Ok(EXPONENTS[exponent])
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct ErrorCorrectionCodewordCount(u8);

impl ErrorCorrectionCodewordCount {
    pub fn new(requested: u8) -> Result<Self, ReedSolomonError> {
        let count = Self(requested);
        if SUPPORTED_ECC_CODEWORD_COUNTS.contains(&count) {
            Ok(count)
        } else {
            Err(ReedSolomonError::UnsupportedCodewordCount { requested })
        }
    }

    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for ErrorCorrectionCodewordCount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Builds the monic QR generator polynomial in descending coefficient order.
#[must_use]
pub fn generator_polynomial(degree: ErrorCorrectionCodewordCount) -> Vec<u8> {
    let mut polynomial = vec![1_u8];
    for root_power in 0..degree.number() {
        let root = EXPONENTS[usize::from(root_power)];
        let mut product = vec![0_u8; polynomial.len() + 1];
        for (index, coefficient) in polynomial.iter().copied().enumerate() {
            product[index] ^= coefficient;
            product[index + 1] ^= multiply(coefficient, root);
        }
        polynomial = product;
    }
    polynomial
}

/// Returns the error-correction codewords for one QR data block.
pub fn generate_error_correction(
    data: &[u8],
    codeword_count: ErrorCorrectionCodewordCount,
) -> Result<Vec<u8>, ReedSolomonError> {
    let generator = generator_polynomial(codeword_count);
    let count = codeword_count.number();
    let maximum_data_codewords = usize::from(u8::MAX - count);
    if data.len() > maximum_data_codewords {
        return Err(ReedSolomonError::BlockTooLong {
            data_codewords: data.len(),
            ecc_codewords: count,
            maximum_total_codewords: usize::from(u8::MAX),
        });
    }

    let mut remainder = vec![0_u8; usize::from(count)];
    for &data_codeword in data {
        let first = remainder
            .first()
            .copied()
            .ok_or(ReedSolomonError::UnsupportedCodewordCount { requested: count })?;
        let factor = data_codeword ^ first;
        remainder.rotate_left(1);
        if let Some(last) = remainder.last_mut() {
            *last = 0;
        }
        for (value, coefficient) in remainder.iter_mut().zip(generator.iter().copied().skip(1)) {
            *value ^= multiply(coefficient, factor);
        }
    }
    Ok(remainder)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReedSolomonError {
    DivisionByZero,
    UnsupportedCodewordCount {
        requested: u8,
    },
    BlockTooLong {
        data_codewords: usize,
        ecc_codewords: u8,
        maximum_total_codewords: usize,
    },
}

impl fmt::Display for ReedSolomonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(formatter, "cannot divide a GF(256) value by zero"),
            Self::UnsupportedCodewordCount { requested } => write!(
                formatter,
                "QR error-correction codeword count must be one of {SUPPORTED_ECC_CODEWORD_COUNTS:?}, got {requested}"
            ),
            Self::BlockTooLong {
                data_codewords,
                ecc_codewords,
                maximum_total_codewords,
            } => write!(
                formatter,
                "Reed–Solomon block has {data_codewords} data and {ecc_codewords} error-correction codewords, exceeding {maximum_total_codewords} total codewords"
            ),
        }
    }
}

impl Error for ReedSolomonError {}

const fn exponent_table() -> [u8; 510] {
    let mut exponents = [0_u8; 510];
    let mut value = 1_u16;
    let mut index = 0;
    while index < 255 {
        exponents[index] = value as u8;
        value <<= 1;
        if value & 0x100 != 0 {
            value ^= PRIMITIVE_POLYNOMIAL;
        }
        index += 1;
    }
    while index < exponents.len() {
        exponents[index] = exponents[index - 255];
        index += 1;
    }
    exponents
}

const fn logarithm_table() -> [u8; 256] {
    let exponents = exponent_table();
    let mut logarithms = [0_u8; 256];
    let mut exponent = 0;
    while exponent < 255 {
        logarithms[exponents[exponent] as usize] = exponent as u8;
        exponent += 1;
    }
    logarithms
}

#[cfg(test)]
mod tests {
    use super::{exponent_table, logarithm_table};

    #[test]
    fn generated_field_tables_obey_primitive_field_invariants() {
        let exponent_generator: fn() -> [u8; 510] = exponent_table;
        let logarithm_generator: fn() -> [u8; 256] = logarithm_table;
        let exponents = std::hint::black_box(exponent_generator)();
        let logarithms = std::hint::black_box(logarithm_generator)();
        assert_eq!(&exponents[..8], &[1, 2, 4, 8, 16, 32, 64, 128]);
        for exponent in 0..255 {
            assert_eq!(exponents[exponent + 255], exponents[exponent]);
            assert_eq!(
                usize::from(logarithms[usize::from(exponents[exponent])]),
                exponent
            );
        }
    }
}
