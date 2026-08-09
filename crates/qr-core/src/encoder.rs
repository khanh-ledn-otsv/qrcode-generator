//! Focused whole-payload QR encoding interface.

use crate::Version;
use crate::codeword_stream::{
    CodewordStreamError, CodewordStreamRequest, construct as construct_codewords,
};
use crate::encoding::{EciAssignment, EncodeRequest, EncodingError, encode as encode_data};
use crate::matrix::{MaskId, ModuleMatrix};
use crate::selection::{SelectionError, select_mask};
use crate::tables::{DataMode, ErrorCorrection};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedQr {
    version: Version,
    ecc: ErrorCorrection,
    mode: DataMode,
    eci_assignment: Option<EciAssignment>,
    mask: MaskId,
    data_bits_used: u32,
    data_bits_capacity: u32,
    minimum_version_applied: bool,
    modules: ModuleMatrix,
}

impl EncodedQr {
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
    pub const fn mask(&self) -> MaskId {
        self.mask
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
    pub const fn minimum_version_applied(&self) -> bool {
        self.minimum_version_applied
    }

    #[must_use]
    pub const fn modules(&self) -> &ModuleMatrix {
        &self.modules
    }
}

pub fn encode(request: EncodeRequest<'_>) -> Result<EncodedQr, EncodeError> {
    let encoded = encode_data(request)?;
    let stream = construct_codewords(CodewordStreamRequest {
        version: encoded.version(),
        ecc: encoded.ecc(),
        data_codewords: encoded.data_codewords(),
    })?;
    let selected = select_mask(&stream)?;
    let mask = selected.mask();
    Ok(EncodedQr {
        version: encoded.version(),
        ecc: encoded.ecc(),
        mode: encoded.mode(),
        eci_assignment: encoded.eci_assignment(),
        mask,
        data_bits_used: encoded.data_bits_used(),
        data_bits_capacity: encoded.data_bits_capacity(),
        minimum_version_applied: encoded.minimum_version_applied(),
        modules: selected.into_matrix(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodeError {
    Payload(EncodingError),
    Codewords(CodewordStreamError),
    Selection(SelectionError),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Payload(error) => error.fmt(formatter),
            Self::Codewords(error) => error.fmt(formatter),
            Self::Selection(error) => error.fmt(formatter),
        }
    }
}

impl Error for EncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Payload(error) => Some(error),
            Self::Codewords(error) => Some(error),
            Self::Selection(error) => Some(error),
        }
    }
}

impl From<EncodingError> for EncodeError {
    fn from(error: EncodingError) -> Self {
        Self::Payload(error)
    }
}

impl From<CodewordStreamError> for EncodeError {
    fn from(error: CodewordStreamError) -> Self {
        Self::Codewords(error)
    }
}

impl From<SelectionError> for EncodeError {
    fn from(error: SelectionError) -> Self {
        Self::Selection(error)
    }
}
