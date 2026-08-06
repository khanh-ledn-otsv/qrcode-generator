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
pub const SUPPORTED_ECC_CODEWORD_COUNTS: [u8; 13] =
    [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30];

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

/// Builds the monic QR generator polynomial in descending coefficient order.
pub fn generator_polynomial(degree: u8) -> Result<Vec<u8>, ReedSolomonError> {
    validate_degree(degree)?;
    let mut polynomial = vec![1_u8];
    for root_power in 0..degree {
        let root = EXPONENTS[usize::from(root_power)];
        let mut product = vec![0_u8; polynomial.len() + 1];
        for (index, coefficient) in polynomial.iter().copied().enumerate() {
            product[index] ^= coefficient;
            product[index + 1] ^= multiply(coefficient, root);
        }
        polynomial = product;
    }
    Ok(polynomial)
}

/// Returns the error-correction codewords for one QR data block.
pub fn generate_error_correction(
    data: &[u8],
    codeword_count: u8,
) -> Result<Vec<u8>, ReedSolomonError> {
    let generator = generator_polynomial(codeword_count)?;
    let maximum_data_codewords = usize::from(u8::MAX - codeword_count);
    if data.len() > maximum_data_codewords {
        return Err(ReedSolomonError::BlockTooLong {
            data_codewords: data.len(),
            ecc_codewords: codeword_count,
            maximum_total_codewords: usize::from(u8::MAX),
        });
    }

    let mut remainder = vec![0_u8; usize::from(codeword_count)];
    for &data_codeword in data {
        let first =
            remainder
                .first()
                .copied()
                .ok_or(ReedSolomonError::UnsupportedCodewordCount {
                    requested: codeword_count,
                })?;
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

fn validate_degree(degree: u8) -> Result<(), ReedSolomonError> {
    if SUPPORTED_ECC_CODEWORD_COUNTS.contains(&degree) {
        Ok(())
    } else {
        Err(ReedSolomonError::UnsupportedCodewordCount { requested: degree })
    }
}

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
