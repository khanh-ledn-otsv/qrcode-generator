//! Standards-conformant QR encoding primitives.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

pub mod bch;
pub mod bit_buffer;
pub mod codeword_stream;
mod encoder;
pub mod encoding;
pub mod matrix;
pub mod penalty;
pub mod reed_solomon;
pub mod selection;
pub mod tables;

pub use encoder::{EncodeError, EncodedQr, encode};
pub use encoding::{EciAssignment, EncodeRequest};

/// A QR Code Model 2 symbol version.
///
/// ISO/IEC 18004:2024, 5.3.2.1 defines versions 1 through 40 and their
/// corresponding symbol sizes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Version(u8);

impl Version {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 40;

    pub const fn new(number: u8) -> Result<Self, VersionError> {
        if number < Self::MIN || number > Self::MAX {
            return Err(VersionError { number });
        }
        Ok(Self(number))
    }

    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }

    /// Returns the width or height of the QR matrix in modules.
    ///
    /// ISO/IEC 18004:2024, 5.3.2.1 defines Version 1 as 21 modules square
    /// and each subsequent version as four modules wider per side.
    #[must_use]
    pub fn symbol_size(self) -> u16 {
        17 + u16::from(self.0) * 4
    }
}

impl TryFrom<u8> for Version {
    type Error = VersionError;

    fn try_from(number: u8) -> Result<Self, Self::Error> {
        Self::new(number)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionError {
    number: u8,
}

impl VersionError {
    #[must_use]
    pub const fn number(self) -> u8 {
        self.number
    }
}

impl fmt::Display for VersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "QR version must be between {} and {}, got {}",
            Version::MIN,
            Version::MAX,
            self.number
        )
    }
}

impl Error for VersionError {}
