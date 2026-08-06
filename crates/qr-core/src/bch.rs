//! QR format and version BCH information values.
//!
//! ISO/IEC 18004:2024 format/version information BCH construction; 2024
//! clause mapping pending audit. Corroborated by Nayuki 1.8.0
//! `rust/src/lib.rs::{draw_format_bits,draw_version}` and python-qrcode 8.2
//! `qrcode/util.py::{BCH_type_info,BCH_type_number}`. Evidence is
//! `public-corroborated, non-normative` pending a complete 2024 audit.

use crate::Version;
use crate::matrix::MaskId;
use crate::tables::ErrorCorrection;

const FORMAT_GENERATOR: u32 = 0x537;
const FORMAT_MASK: u32 = 0x5412;
const VERSION_GENERATOR: u32 = 0x1F25;

/// Returns the 15-bit, masked format information for an ECC/mask pair.
#[must_use]
pub fn format_bits(ecc: ErrorCorrection, mask: MaskId) -> u16 {
    let ecc_bits = match ecc {
        ErrorCorrection::Low => 1_u32,
        ErrorCorrection::Medium => 0,
        ErrorCorrection::Quartile => 3,
        ErrorCorrection::High => 2,
    };
    let data = (ecc_bits << 3) | u32::from(mask.number());
    let encoded = (data << 10) | polynomial_remainder(data << 10, FORMAT_GENERATOR);
    (encoded ^ FORMAT_MASK) as u16
}

/// Returns the 18-bit version information for Versions 7 through 40.
#[must_use]
pub fn version_bits(version: Version) -> Option<u32> {
    if version.number() < 7 {
        return None;
    }
    let data = u32::from(version.number());
    Some((data << 12) | polynomial_remainder(data << 12, VERSION_GENERATOR))
}

fn polynomial_remainder(mut value: u32, generator: u32) -> u32 {
    let generator_degree = u32::BITS - 1 - generator.leading_zeros();
    while value != 0 {
        let value_degree = u32::BITS - 1 - value.leading_zeros();
        if value_degree < generator_degree {
            break;
        }
        value ^= generator << (value_degree - generator_degree);
    }
    value
}
