//! Checked MSB-first bit-buffer operations used by QR segment encoding.

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BitBuffer {
    bytes: Vec<u8>,
    bit_length: usize,
}

impl BitBuffer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_length: 0,
        }
    }

    #[must_use]
    pub const fn bit_length(&self) -> usize {
        self.bit_length
    }

    pub fn append_bits(&mut self, value: u32, width: u8) -> Result<(), BitBufferError> {
        if width > 32 {
            return Err(BitBufferError::InvalidWidth { width });
        }
        if width < 32 && value >= (1_u32 << width) {
            return Err(BitBufferError::ValueDoesNotFit { value, width });
        }
        let new_length = self
            .bit_length
            .checked_add(usize::from(width))
            .ok_or(BitBufferError::LengthOverflow)?;
        let required_bytes = new_length
            .checked_add(7)
            .ok_or(BitBufferError::LengthOverflow)?
            / 8;
        self.bytes.resize(required_bytes, 0);
        for offset in (0..width).rev() {
            if ((value >> offset) & 1) != 0 {
                let byte_index = self.bit_length / 8;
                let bit_index = self.bit_length % 8;
                let byte = self
                    .bytes
                    .get_mut(byte_index)
                    .ok_or(BitBufferError::LengthOverflow)?;
                *byte |= 0x80_u8 >> bit_index;
            }
            self.bit_length = self
                .bit_length
                .checked_add(1)
                .ok_or(BitBufferError::LengthOverflow)?;
        }
        Ok(())
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, BitBufferError> {
        if !self.bit_length.is_multiple_of(8) {
            return Err(BitBufferError::NotByteAligned {
                bit_length: self.bit_length,
            });
        }
        Ok(self.bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BitBufferError {
    InvalidWidth { width: u8 },
    ValueDoesNotFit { value: u32, width: u8 },
    LengthOverflow,
    NotByteAligned { bit_length: usize },
}

impl fmt::Display for BitBufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWidth { width } => write!(formatter, "bit width {width} exceeds 32"),
            Self::ValueDoesNotFit { value, width } => {
                write!(formatter, "value {value} does not fit in {width} bits")
            }
            Self::LengthOverflow => formatter.write_str("bit-buffer length overflow"),
            Self::NotByteAligned { bit_length } => {
                write!(formatter, "bit length {bit_length} is not byte-aligned")
            }
        }
    }
}

impl Error for BitBufferError {}
