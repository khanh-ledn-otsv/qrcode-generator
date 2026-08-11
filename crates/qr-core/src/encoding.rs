//! Deterministic, version-aware QR data segmentation and encoding.
//!
//! ISO/IEC 18004:2024 defines ECI and mode encoding in 7.4.3 through 7.4.7,
//! terminator handling in 7.4.10, and bit-stream-to-codeword conversion in
//! 7.4.11.
//! 2024 clause mapping pending audit.
//! The UTF-8 ECI 26 choice is the project's explicit release policy.
//! Equal-bit segment plans prefer fewer segments, then Numeric, Alphanumeric,
//! and Byte in that order, then a longer first segment; the canonical suffix
//! applies the same rule recursively.

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingMode {
    Single(DataMode),
    Mixed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodedSegment {
    mode: DataMode,
    character_count: u32,
    byte_count: u32,
}

impl EncodedSegment {
    #[must_use]
    pub const fn mode(self) -> DataMode {
        self.mode
    }

    #[must_use]
    pub const fn character_count(self) -> u32 {
        self.character_count
    }

    #[must_use]
    pub const fn byte_count(self) -> u32 {
        self.byte_count
    }
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
    text: &'a str,
    ecc: ErrorCorrection,
    min_version: Version,
    max_version: Version,
}

impl<'a> EncodeRequest<'a> {
    #[must_use]
    pub const fn first_fit(text: &'a str, ecc: ErrorCorrection, max_version: Version) -> Self {
        Self::with_version_range(text, ecc, Version::MINIMUM, max_version)
    }

    #[must_use]
    pub const fn with_version_range(
        text: &'a str,
        ecc: ErrorCorrection,
        min_version: Version,
        max_version: Version,
    ) -> Self {
        Self {
            text,
            ecc,
            min_version,
            max_version,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedData {
    version: Version,
    ecc: ErrorCorrection,
    mode: EncodingMode,
    segments: Vec<EncodedSegment>,
    eci_assignment: Option<EciAssignment>,
    data_bits_used: u32,
    data_bits_capacity: u32,
    minimum_version_increased_selection: bool,
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
    pub const fn mode(&self) -> EncodingMode {
        self.mode
    }

    #[must_use]
    pub fn segments(&self) -> &[EncodedSegment] {
        &self.segments
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
    pub const fn minimum_version_increased_selection(&self) -> bool {
        self.minimum_version_increased_selection
    }

    #[must_use]
    pub fn data_codewords(&self) -> &[u8] {
        &self.data_codewords
    }
}

pub fn encode(request: EncodeRequest<'_>) -> Result<EncodedData, EncodingError> {
    if request.min_version > request.max_version {
        return Err(EncodingError::InvalidVersionRange {
            minimum: request.min_version,
            maximum: request.max_version,
        });
    }
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

    let first_fit = first_fitting_version(payload, request.text.is_ascii(), request.ecc)?;
    let selected_version = first_fit.max(request.min_version);
    if selected_version > request.max_version {
        return Err(EncodingError::PayloadTooLargeForProfile {
            required: first_fit,
            maximum: request.max_version,
        });
    }

    let prepared = PreparedEncoding::new(payload, request.text.is_ascii(), selected_version)?;
    encode_at_version(
        &prepared,
        request.ecc,
        selected_version,
        selected_version > first_fit,
    )
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
}

impl<'a> PreparedSegment<'a> {
    fn new(payload: &'a [u8], mode: DataMode) -> Result<Self, EncodingError> {
        let character_count =
            u32::try_from(payload.len()).map_err(|_| EncodingError::LengthOverflow)?;
        if mode == DataMode::Numeric {
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
            })
        } else if mode == DataMode::Alphanumeric {
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
            })
        } else if mode == DataMode::Byte {
            Ok(Self {
                mode: DataMode::Byte,
                mode_indicator: 0b0100,
                payload: PayloadEncoding::Bytes(payload),
                character_count,
                payload_bits: character_count
                    .checked_mul(8)
                    .ok_or(EncodingError::LengthOverflow)?,
            })
        } else {
            Err(EncodingError::MalformedPayload { mode })
        }
    }

    fn required_bits(self, version: Version) -> Result<Option<u32>, EncodingError> {
        let count_width = character_count_bits(version, self.mode);
        if self.character_count >= (1_u32 << count_width) {
            return Ok(None);
        }
        4_u32
            .checked_add(u32::from(count_width))
            .and_then(|bits| bits.checked_add(self.payload_bits))
            .map(Some)
            .ok_or(EncodingError::LengthOverflow)
    }

    fn write(self, version: Version, buffer: &mut BitBuffer) -> Result<(), EncodingError> {
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedEncoding<'a> {
    segments: Vec<PreparedSegment<'a>>,
    eci_assignment: Option<EciAssignment>,
    required_bits: u32,
}

impl<'a> PreparedEncoding<'a> {
    fn new(payload: &'a [u8], is_ascii: bool, version: Version) -> Result<Self, EncodingError> {
        if payload.iter().all(u8::is_ascii_digit) {
            return Self::single_mode(payload, is_ascii, version, DataMode::Numeric);
        }
        if payload
            .iter()
            .all(|byte| alphanumeric_value(*byte).is_some())
            && !payload.iter().any(u8::is_ascii_digit)
        {
            return Self::single_mode(payload, is_ascii, version, DataMode::Alphanumeric);
        }
        if payload
            .iter()
            .all(|byte| alphanumeric_value(*byte).is_none())
        {
            return Self::single_mode(payload, is_ascii, version, DataMode::Byte);
        }

        let boundaries = utf8_boundaries(payload)?;
        let mut best = vec![None; payload.len() + 1];
        *best
            .get_mut(payload.len())
            .ok_or(EncodingError::LengthOverflow)? = Some(BestSuffix {
            bits: 0,
            segment_count: 0,
            choice: None,
        });

        for &start in boundaries.iter().rev().skip(1) {
            let mut numeric = true;
            let mut alphanumeric = true;
            let mut previous_end = start;
            for &end in boundaries.iter().filter(|&&end| end > start) {
                let slice = payload
                    .get(start..end)
                    .ok_or(EncodingError::LengthOverflow)?;
                let last_character = payload
                    .get(previous_end..end)
                    .ok_or(EncodingError::LengthOverflow)?;
                previous_end = end;
                numeric &= last_character.len() == 1
                    && last_character.first().is_some_and(u8::is_ascii_digit);
                alphanumeric &= last_character.len() == 1
                    && last_character
                        .first()
                        .and_then(|byte| alphanumeric_value(*byte))
                        .is_some();

                for mode in [DataMode::Numeric, DataMode::Alphanumeric, DataMode::Byte] {
                    if (mode == DataMode::Numeric && !numeric)
                        || (mode == DataMode::Alphanumeric && !alphanumeric)
                    {
                        continue;
                    }
                    let segment = PreparedSegment::new(slice, mode)?;
                    let Some(segment_bits) = segment.required_bits(version)? else {
                        continue;
                    };
                    let suffix = best
                        .get(end)
                        .and_then(Option::as_ref)
                        .ok_or(EncodingError::LengthOverflow)?;
                    let candidate = BestSuffix {
                        bits: segment_bits
                            .checked_add(suffix.bits)
                            .ok_or(EncodingError::LengthOverflow)?,
                        segment_count: suffix
                            .segment_count
                            .checked_add(1)
                            .ok_or(EncodingError::LengthOverflow)?,
                        choice: Some(SegmentChoice { mode, end }),
                    };
                    let replace = best
                        .get(start)
                        .and_then(Option::as_ref)
                        .is_none_or(|current| candidate.is_better_than(current, start));
                    if replace {
                        *best.get_mut(start).ok_or(EncodingError::LengthOverflow)? =
                            Some(candidate);
                    }
                }
            }
        }

        let mut segments = Vec::new();
        let mut start = 0;
        while start < payload.len() {
            let choice = best
                .get(start)
                .and_then(Option::as_ref)
                .and_then(|suffix| suffix.choice)
                .ok_or(EncodingError::LengthOverflow)?;
            segments.push(PreparedSegment::new(
                payload
                    .get(start..choice.end)
                    .ok_or(EncodingError::LengthOverflow)?,
                choice.mode,
            )?);
            start = choice.end;
        }
        let eci_assignment = (!is_ascii).then_some(EciAssignment::Utf8);
        let control_bits = if eci_assignment.is_some() { 12 } else { 0 };
        let required_bits = best
            .first()
            .and_then(Option::as_ref)
            .ok_or(EncodingError::LengthOverflow)?
            .bits
            .checked_add(control_bits)
            .ok_or(EncodingError::LengthOverflow)?;
        Ok(Self {
            segments,
            eci_assignment,
            required_bits,
        })
    }

    fn single_mode(
        payload: &'a [u8],
        is_ascii: bool,
        version: Version,
        mode: DataMode,
    ) -> Result<Self, EncodingError> {
        let count_width = character_count_bits(version, mode);
        let count_limit = (1_usize << count_width) - 1;
        let grouping = match mode {
            DataMode::Numeric => 3,
            DataMode::Alphanumeric => 2,
            DataMode::Byte => 1,
            DataMode::Kanji => return Err(EncodingError::MalformedPayload { mode }),
        };
        let grouped_limit = count_limit - count_limit % grouping;
        let mut segments = Vec::new();
        let mut start = 0;
        let mut required_bits = 0_u32;
        while start < payload.len() {
            let remaining = payload.len() - start;
            let requested_length = if remaining <= count_limit {
                remaining
            } else {
                grouped_limit
            };
            let end = if mode == DataMode::Byte {
                largest_utf8_boundary(payload, start, requested_length)?
            } else {
                start
                    .checked_add(requested_length)
                    .ok_or(EncodingError::LengthOverflow)?
            };
            if end <= start {
                return Err(EncodingError::LengthOverflow);
            }
            let segment = PreparedSegment::new(
                payload
                    .get(start..end)
                    .ok_or(EncodingError::LengthOverflow)?,
                mode,
            )?;
            required_bits = required_bits
                .checked_add(
                    segment
                        .required_bits(version)?
                        .ok_or(EncodingError::LengthOverflow)?,
                )
                .ok_or(EncodingError::LengthOverflow)?;
            segments.push(segment);
            start = end;
        }
        let eci_assignment = (!is_ascii).then_some(EciAssignment::Utf8);
        required_bits = required_bits
            .checked_add(if eci_assignment.is_some() { 12 } else { 0 })
            .ok_or(EncodingError::LengthOverflow)?;
        Ok(Self {
            segments,
            eci_assignment,
            required_bits,
        })
    }
}

fn largest_utf8_boundary(
    payload: &[u8],
    start: usize,
    maximum_length: usize,
) -> Result<usize, EncodingError> {
    let maximum_end = start
        .checked_add(maximum_length)
        .ok_or(EncodingError::LengthOverflow)?
        .min(payload.len());
    let text = std::str::from_utf8(payload).map_err(|_| EncodingError::LengthOverflow)?;
    if text.is_char_boundary(maximum_end) {
        return Ok(maximum_end);
    }
    let first_candidate = start.checked_add(1).ok_or(EncodingError::LengthOverflow)?;
    (first_candidate..maximum_end)
        .rev()
        .find(|&end| text.is_char_boundary(end))
        .ok_or(EncodingError::LengthOverflow)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SegmentChoice {
    mode: DataMode,
    end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BestSuffix {
    bits: u32,
    segment_count: u32,
    choice: Option<SegmentChoice>,
}

impl BestSuffix {
    fn is_better_than(&self, other: &Self, start: usize) -> bool {
        let (Some(self_choice), Some(other_choice)) = (self.choice, other.choice) else {
            return false;
        };
        (
            self.bits,
            self.segment_count,
            mode_tie_rank(self_choice.mode),
            usize::MAX - (self_choice.end - start),
        ) < (
            other.bits,
            other.segment_count,
            mode_tie_rank(other_choice.mode),
            usize::MAX - (other_choice.end - start),
        )
    }
}

const fn mode_tie_rank(mode: DataMode) -> u8 {
    match mode {
        DataMode::Numeric => 0,
        DataMode::Alphanumeric => 1,
        DataMode::Byte => 2,
        DataMode::Kanji => 3,
    }
}

fn utf8_boundaries(payload: &[u8]) -> Result<Vec<usize>, EncodingError> {
    let text = std::str::from_utf8(payload).map_err(|_| EncodingError::LengthOverflow)?;
    let mut boundaries = text
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(payload.len());
    Ok(boundaries)
}

fn first_fitting_version(
    payload: &[u8],
    is_ascii: bool,
    error_correction: ErrorCorrection,
) -> Result<Version, EncodingError> {
    let mut prepared_by_band: [Option<PreparedEncoding<'_>>; 3] = [None, None, None];
    for number in Version::MIN..=Version::MAX {
        let version = Version::new(number)?;
        let band = version_band(version);
        let slot = prepared_by_band
            .get_mut(band)
            .ok_or(EncodingError::LengthOverflow)?;
        if slot.is_none() {
            *slot = Some(PreparedEncoding::new(payload, is_ascii, version)?);
        }
        let used_bits = prepared_by_band
            .get(band)
            .ok_or(EncodingError::LengthOverflow)?
            .as_ref()
            .ok_or(EncodingError::LengthOverflow)?
            .required_bits;
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
    prepared: &PreparedEncoding<'_>,
    error_correction: ErrorCorrection,
    version: Version,
    minimum_version_increased_selection: bool,
) -> Result<EncodedData, EncodingError> {
    let row = lookup(version, error_correction)?;
    let capacity_bits = u32::from(row.data_codewords())
        .checked_mul(8)
        .ok_or(EncodingError::LengthOverflow)?;
    let mut buffer = BitBuffer::new();
    if let Some(assignment) = prepared.eci_assignment {
        buffer.append_bits(0b0111, 4)?;
        buffer.append_bits(assignment.number(), 8)?;
    }
    for segment in &prepared.segments {
        segment.write(version, &mut buffer)?;
    }
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
    let first_mode = prepared
        .segments
        .first()
        .map(|segment| segment.mode)
        .ok_or(EncodingError::LengthOverflow)?;
    let encoding_mode = if prepared
        .segments
        .iter()
        .all(|segment| segment.mode == first_mode)
    {
        EncodingMode::Single(first_mode)
    } else {
        EncodingMode::Mixed
    };
    Ok(EncodedData {
        version,
        ecc: error_correction,
        mode: encoding_mode,
        segments: prepared
            .segments
            .iter()
            .map(|segment| EncodedSegment {
                mode: segment.mode,
                character_count: segment.character_count,
                byte_count: segment.character_count,
            })
            .collect(),
        eci_assignment: prepared.eci_assignment,
        data_bits_used,
        data_bits_capacity: capacity_bits,
        minimum_version_increased_selection,
        data_codewords,
    })
}

const fn version_band(version: Version) -> usize {
    match version.number() {
        1..=9 => 0,
        10..=26 => 1,
        _ => 2,
    }
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
    // 2024 clause mapping pending audit.
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
    InvalidVersionRange { minimum: Version, maximum: Version },
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
            Self::InvalidVersionRange { minimum, maximum } => write!(
                formatter,
                "minimum QR version {} exceeds maximum version {}",
                minimum.number(),
                maximum.number()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn suffix(bits: u32, segment_count: u32, mode: DataMode, end: usize) -> BestSuffix {
        BestSuffix {
            bits,
            segment_count,
            choice: Some(SegmentChoice { mode, end }),
        }
    }

    #[test]
    fn equal_cost_suffix_ties_follow_the_documented_total_order() {
        let reference = suffix(20, 2, DataMode::Alphanumeric, 8);

        assert!(suffix(19, 3, DataMode::Byte, 4).is_better_than(&reference, 3));
        assert!(!suffix(21, 1, DataMode::Numeric, 12).is_better_than(&reference, 3));
        assert!(suffix(20, 1, DataMode::Byte, 4).is_better_than(&reference, 3));
        assert!(!suffix(20, 3, DataMode::Numeric, 12).is_better_than(&reference, 3));
        assert!(suffix(20, 2, DataMode::Numeric, 4).is_better_than(&reference, 3));
        assert!(!suffix(20, 2, DataMode::Byte, 12).is_better_than(&reference, 3));
        assert!(suffix(20, 2, DataMode::Alphanumeric, 9).is_better_than(&reference, 3));
        assert!(!suffix(20, 2, DataMode::Alphanumeric, 7).is_better_than(&reference, 3));
        assert!(!reference.is_better_than(&reference, 3));
        assert!(
            !BestSuffix {
                bits: 0,
                segment_count: 0,
                choice: None
            }
            .is_better_than(&reference, 3)
        );
        assert_eq!(
            [
                mode_tie_rank(DataMode::Numeric),
                mode_tie_rank(DataMode::Alphanumeric),
                mode_tie_rank(DataMode::Byte),
                mode_tie_rank(DataMode::Kanji),
            ],
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn version_bands_match_all_character_count_width_boundaries() {
        for (number, expected) in [(1, 0), (9, 0), (10, 1), (26, 1), (27, 2), (40, 2)] {
            assert_eq!(
                version_band(Version::new(number).expect("band endpoint is a QR version")),
                expected
            );
        }
    }

    #[test]
    fn same_mode_splitting_respects_grouping_and_utf8_boundaries() {
        let version_one = Version::MINIMUM;
        let alphanumeric = vec![b'A'; 512];
        let prepared =
            PreparedEncoding::single_mode(&alphanumeric, true, version_one, DataMode::Alphanumeric)
                .expect("oversized alphanumeric data splits on a complete pair");
        assert_eq!(
            prepared
                .segments
                .iter()
                .map(|segment| segment.character_count)
                .collect::<Vec<_>>(),
            [510, 2]
        );

        let utf8 = "é".repeat(128);
        let prepared =
            PreparedEncoding::single_mode(utf8.as_bytes(), false, version_one, DataMode::Byte)
                .expect("oversized UTF-8 data splits at a code-point boundary");
        assert_eq!(
            prepared
                .segments
                .iter()
                .map(|segment| segment.character_count)
                .collect::<Vec<_>>(),
            [254, 2]
        );
    }
}
