//! Whole-payload QR data encoding for the release-one mode policy.
//!
//! ISO/IEC 18004:2024 defines ECI and mode encoding in 7.4.3 through 7.4.7,
//! terminator handling in 7.4.10, and bit-stream-to-codeword conversion in
//! 7.4.11. The UTF-8 ECI 26 choice is the project's explicit release policy.

use crate::Version;
use crate::bit_buffer::{BitBuffer, BitBufferError};
use crate::tables::{DataMode, ErrorCorrection, TableLookupError, character_count_bits, lookup};
use std::error::Error;
use std::fmt;

const MAX_INPUT_BYTES: usize = 4 * 1024;
const UTF8_ECI_ASSIGNMENT: u32 = 26;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EciAssignment {
    Utf8,
}

impl EciAssignment {
    #[must_use]
    pub const fn number(self) -> u32 {
        match self {
            Self::Utf8 => UTF8_ECI_ASSIGNMENT,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodeRequest<'a> {
    pub text: &'a str,
    pub ecc: ErrorCorrection,
    pub max_version: Version,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedData {
    version: Version,
    ecc: ErrorCorrection,
    mode: DataMode,
    eci_assignment: Option<EciAssignment>,
    data_bits_used: u32,
    data_bits_capacity: u32,
    data_codewords: Vec<u8>,
}

impl EncodedData {
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    #[must_use]
    pub const fn ecc(&self) -> ErrorCorrection {
        self.ecc
    }

    #[must_use]
    pub const fn mode(&self) -> DataMode {
        self.mode
    }

    #[must_use]
    pub const fn eci_assignment(&self) -> Option<EciAssignment> {
        self.eci_assignment
    }

    #[must_use]
    pub const fn data_bits_used(&self) -> u32 {
        self.data_bits_used
    }

    #[must_use]
    pub const fn data_bits_capacity(&self) -> u32 {
        self.data_bits_capacity
    }

    #[must_use]
    pub fn data_codewords(&self) -> &[u8] {
        &self.data_codewords
    }
}

pub fn encode(request: EncodeRequest<'_>) -> Result<EncodedData, EncodingError> {
    let payload = request.text.as_bytes();
    if payload.is_empty() {
        return Err(EncodingError::EmptyPayload);
    }
    if payload.len() > MAX_INPUT_BYTES {
        return Err(EncodingError::InputLimitExceeded {
            byte_length: payload.len(),
            maximum: MAX_INPUT_BYTES,
        });
    }

    let segment = PreparedSegment::new(payload, request.text.is_ascii())?;
    let first_fit = first_fitting_version(&segment, request.ecc)?;
    if first_fit > request.max_version {
        return Err(EncodingError::PayloadTooLargeForProfile {
            required: first_fit,
            maximum: request.max_version,
        });
    }

    encode_at_version(&segment, request.ecc, first_fit)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PayloadEncoding<'a> {
    Numeric(&'a [u8]),
    Alphanumeric(&'a [u8]),
    Bytes(&'a [u8]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedSegment<'a> {
    mode: DataMode,
    mode_indicator: u32,
    payload: PayloadEncoding<'a>,
    character_count: u32,
    payload_bits: u32,
    eci_assignment: Option<EciAssignment>,
}

impl<'a> PreparedSegment<'a> {
    fn new(payload: &'a [u8], is_ascii: bool) -> Result<Self, EncodingError> {
        let character_count =
            u32::try_from(payload.len()).map_err(|_| EncodingError::LengthOverflow)?;
        if payload.iter().all(u8::is_ascii_digit) {
            let groups = character_count / 3;
            let tail = match character_count % 3 {
                0 => 0,
                1 => 4,
                _ => 7,
            };
            let payload_bits = groups
                .checked_mul(10)
                .and_then(|bits| bits.checked_add(tail))
                .ok_or(EncodingError::LengthOverflow)?;
            Ok(Self {
                mode: DataMode::Numeric,
                mode_indicator: 0b0001,
                payload: PayloadEncoding::Numeric(payload),
                character_count,
                payload_bits,
                eci_assignment: None,
            })
        } else if payload
            .iter()
            .all(|byte| alphanumeric_value(*byte).is_some())
        {
            let payload_bits = (character_count / 2)
                .checked_mul(11)
                .and_then(|bits| bits.checked_add(if character_count % 2 == 0 { 0 } else { 6 }))
                .ok_or(EncodingError::LengthOverflow)?;
            Ok(Self {
                mode: DataMode::Alphanumeric,
                mode_indicator: 0b0010,
                payload: PayloadEncoding::Alphanumeric(payload),
                character_count,
                payload_bits,
                eci_assignment: None,
            })
        } else {
            Ok(Self {
                mode: DataMode::Byte,
                mode_indicator: 0b0100,
                payload: PayloadEncoding::Bytes(payload),
                character_count,
                payload_bits: character_count
                    .checked_mul(8)
                    .ok_or(EncodingError::LengthOverflow)?,
                eci_assignment: (!is_ascii).then_some(EciAssignment::Utf8),
            })
        }
    }

    fn required_bits(self, version: Version) -> Result<Option<u32>, EncodingError> {
        let count_width = character_count_bits(version, self.mode);
        if self.character_count >= (1_u32 << count_width) {
            return Ok(None);
        }
        let control_bits: u32 = if self.eci_assignment.is_some() { 12 } else { 0 };
        control_bits
            .checked_add(4)
            .and_then(|bits| bits.checked_add(u32::from(count_width)))
            .and_then(|bits| bits.checked_add(self.payload_bits))
            .map(Some)
            .ok_or(EncodingError::LengthOverflow)
    }

    fn write(self, version: Version, buffer: &mut BitBuffer) -> Result<(), EncodingError> {
        if let Some(assignment) = self.eci_assignment {
            buffer.append_bits(0b0111, 4)?;
            buffer.append_bits(assignment.number(), 8)?;
        }
        buffer.append_bits(self.mode_indicator, 4)?;
        buffer.append_bits(
            self.character_count,
            character_count_bits(version, self.mode),
        )?;
        match self.payload {
            PayloadEncoding::Numeric(payload) => append_numeric(buffer, payload),
            PayloadEncoding::Alphanumeric(payload) => append_alphanumeric(buffer, payload),
            PayloadEncoding::Bytes(payload) => append_bytes(buffer, payload),
        }
    }
}

fn first_fitting_version(
    segment: &PreparedSegment<'_>,
    error_correction: ErrorCorrection,
) -> Result<Version, EncodingError> {
    for number in Version::MIN..=Version::MAX {
        let version = Version::new(number)?;
        let Some(used_bits) = segment.required_bits(version)? else {
            continue;
        };
        let capacity_bits = u32::from(lookup(version, error_correction)?.data_codewords())
            .checked_mul(8)
            .ok_or(EncodingError::LengthOverflow)?;
        if used_bits <= capacity_bits {
            return Ok(version);
        }
    }
    Err(EncodingError::PayloadTooLargeForQr)
}

fn encode_at_version(
    segment: &PreparedSegment<'_>,
    error_correction: ErrorCorrection,
    version: Version,
) -> Result<EncodedData, EncodingError> {
    let row = lookup(version, error_correction)?;
    let capacity_bits = u32::from(row.data_codewords())
        .checked_mul(8)
        .ok_or(EncodingError::LengthOverflow)?;
    let mut buffer = BitBuffer::new();
    segment.write(version, &mut buffer)?;
    let data_bits_used =
        u32::try_from(buffer.bit_length()).map_err(|_| EncodingError::LengthOverflow)?;

    let remaining = usize::try_from(capacity_bits)
        .map_err(|_| EncodingError::LengthOverflow)?
        .checked_sub(buffer.bit_length())
        .ok_or(EncodingError::LengthOverflow)?;
    buffer.append_bits(
        0,
        u8::try_from(remaining.min(4)).map_err(|_| EncodingError::LengthOverflow)?,
    )?;
    let alignment_bits = (8 - buffer.bit_length() % 8) % 8;
    buffer.append_bits(
        0,
        u8::try_from(alignment_bits).map_err(|_| EncodingError::LengthOverflow)?,
    )?;
    let mut data_codewords = buffer.into_bytes()?;
    let capacity_codewords = usize::from(row.data_codewords());
    let mut use_first_pad = true;
    while data_codewords.len() < capacity_codewords {
        data_codewords.push(if use_first_pad { 0xEC } else { 0x11 });
        use_first_pad = !use_first_pad;
    }
    if data_codewords.len() != capacity_codewords {
        return Err(EncodingError::LengthOverflow);
    }
    Ok(EncodedData {
        version,
        ecc: error_correction,
        mode: segment.mode,
        eci_assignment: segment.eci_assignment,
        data_bits_used,
        data_bits_capacity: capacity_bits,
        data_codewords,
    })
}

fn append_numeric(buffer: &mut BitBuffer, payload: &[u8]) -> Result<(), EncodingError> {
    for chunk in payload.chunks(3) {
        let value = chunk.iter().try_fold(0_u32, |value, byte| {
            byte.checked_sub(b'0')
                .filter(|digit| *digit <= 9)
                .map(u32::from)
                .and_then(|digit| value.checked_mul(10)?.checked_add(digit))
        });
        let value = value.ok_or(EncodingError::MalformedPayload {
            mode: DataMode::Numeric,
        })?;
        let width = match chunk.len() {
            1 => 4,
            2 => 7,
            3 => 10,
            _ => {
                return Err(EncodingError::MalformedPayload {
                    mode: DataMode::Numeric,
                });
            }
        };
        buffer.append_bits(value, width)?;
    }
    Ok(())
}

fn append_alphanumeric(buffer: &mut BitBuffer, payload: &[u8]) -> Result<(), EncodingError> {
    for chunk in payload.chunks(2) {
        let first = chunk
            .first()
            .and_then(|byte| alphanumeric_value(*byte))
            .ok_or(EncodingError::MalformedPayload {
                mode: DataMode::Alphanumeric,
            })?;
        match chunk.get(1) {
            Some(byte) => {
                let second = alphanumeric_value(*byte).ok_or(EncodingError::MalformedPayload {
                    mode: DataMode::Alphanumeric,
                })?;
                buffer.append_bits(u32::from(first) * 45 + u32::from(second), 11)?;
            }
            None => buffer.append_bits(u32::from(first), 6)?,
        }
    }
    Ok(())
}

fn append_bytes(buffer: &mut BitBuffer, payload: &[u8]) -> Result<(), EncodingError> {
    for byte in payload {
        buffer.append_bits(u32::from(*byte), 8)?;
    }
    Ok(())
}

fn alphanumeric_value(byte: u8) -> Option<u8> {
    // ISO/IEC 18004:2024, 7.4.5 defines this 45-character value mapping.
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'Z' => Some(byte - b'A' + 10),
        b' ' => Some(36),
        b'$' => Some(37),
        b'%' => Some(38),
        b'*' => Some(39),
        b'+' => Some(40),
        b'-' => Some(41),
        b'.' => Some(42),
        b'/' => Some(43),
        b':' => Some(44),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodingError {
    EmptyPayload,
    InputLimitExceeded { byte_length: usize, maximum: usize },
    PayloadTooLargeForProfile { required: Version, maximum: Version },
    PayloadTooLargeForQr,
    MalformedPayload { mode: DataMode },
    LengthOverflow,
    BitBuffer(BitBufferError),
    Table(TableLookupError),
    InvalidVersion(crate::VersionError),
}

impl fmt::Display for EncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPayload => formatter.write_str("QR payload must not be empty"),
            Self::InputLimitExceeded {
                byte_length,
                maximum,
            } => write!(
                formatter,
                "QR payload is {byte_length} bytes; the input limit is {maximum} bytes"
            ),
            Self::PayloadTooLargeForProfile { required, maximum } => write!(
                formatter,
                "QR payload requires version {}, above profile maximum {}",
                required.number(),
                maximum.number()
            ),
            Self::PayloadTooLargeForQr => {
                formatter.write_str("QR payload does not fit in Version 40")
            }
            Self::MalformedPayload { mode } => {
                write!(formatter, "payload is malformed for QR mode {mode:?}")
            }
            Self::LengthOverflow => formatter.write_str("QR encoded length overflow"),
            Self::BitBuffer(error) => error.fmt(formatter),
            Self::Table(error) => error.fmt(formatter),
            Self::InvalidVersion(error) => error.fmt(formatter),
        }
    }
}

impl Error for EncodingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BitBuffer(error) => Some(error),
            Self::Table(error) => Some(error),
            Self::InvalidVersion(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BitBufferError> for EncodingError {
    fn from(error: BitBufferError) -> Self {
        Self::BitBuffer(error)
    }
}

impl From<TableLookupError> for EncodingError {
    fn from(error: TableLookupError) -> Self {
        Self::Table(error)
    }
}

impl From<crate::VersionError> for EncodingError {
    fn from(error: crate::VersionError) -> Self {
        Self::InvalidVersion(error)
    }
}
