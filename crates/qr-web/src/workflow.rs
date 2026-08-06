//! Plain-Rust state transitions for the interactive QR workflow.

use qr_core::encoding::EncodingError;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeError, EncodeRequest, Version, encode};
use qr_render::{
    OutputProfile, ProfileId, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_svg,
};

const CONTROL_CHARACTER_CAUTION: &str =
    "This payload contains control characters. Confirm that they are intentional.";
const INTERNAL_FAILURE_MESSAGE: &str =
    "QR generation failed unexpectedly. Change the input and try again.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Revision(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRequest {
    revision: Revision,
    payload: String,
    profile_id: ProfileId,
    logo_enabled: bool,
}

impl PreviewRequest {
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    #[must_use]
    pub const fn ecc(&self) -> ErrorCorrection {
        if self.logo_enabled {
            ErrorCorrection::High
        } else {
            ErrorCorrection::Medium
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostics {
    ecc: ErrorCorrection,
    maximum_version: Version,
    selected_version: Version,
    used_data_bits: u32,
    available_data_bits: u32,
    data_codewords: u16,
    matrix_modules: u16,
    svg_side_pixels: u32,
    png_side_pixels: u32,
    print_guidance: &'static str,
}

impl Diagnostics {
    #[must_use]
    pub const fn ecc(self) -> ErrorCorrection {
        self.ecc
    }

    #[must_use]
    pub const fn maximum_version(self) -> Version {
        self.maximum_version
    }

    #[must_use]
    pub const fn selected_version(self) -> Version {
        self.selected_version
    }

    #[must_use]
    pub const fn used_data_bits(self) -> u32 {
        self.used_data_bits
    }

    #[must_use]
    pub const fn available_data_bits(self) -> u32 {
        self.available_data_bits
    }

    #[must_use]
    pub const fn data_codewords(self) -> u16 {
        self.data_codewords
    }

    #[must_use]
    pub const fn matrix_modules(self) -> u16 {
        self.matrix_modules
    }

    #[must_use]
    pub const fn svg_side_pixels(self) -> u32 {
        self.svg_side_pixels
    }

    #[must_use]
    pub const fn png_side_pixels(self) -> u32 {
        self.png_side_pixels
    }

    #[must_use]
    pub const fn print_guidance(self) -> &'static str {
        self.print_guidance
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    svg: String,
    diagnostics: Diagnostics,
}

impl Preview {
    #[must_use]
    pub fn svg(&self) -> &str {
        &self.svg
    }

    #[must_use]
    pub const fn diagnostics(&self) -> Diagnostics {
        self.diagnostics
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkflowFailure {
    EmptyPayload,
    InputLimitExceeded { byte_length: usize, maximum: usize },
    OverCapacity { maximum_version: Version },
    Internal,
}

impl WorkflowFailure {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::EmptyPayload => "Enter text to generate a QR code.".to_owned(),
            Self::InputLimitExceeded {
                byte_length,
                maximum,
            } => format!("The payload is {byte_length} bytes; the input limit is {maximum} bytes."),
            Self::OverCapacity { maximum_version } => format!(
                "The payload does not fit this profile's maximum QR version {}.",
                maximum_version.number()
            ),
            Self::Internal => INTERNAL_FAILURE_MESSAGE.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PreviewState {
    Pending,
    Ready(Preview),
    Invalid(WorkflowFailure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowState {
    payload: String,
    profile_id: ProfileId,
    logo_enabled: bool,
    revision: Revision,
    preview_state: PreviewState,
}

impl WorkflowState {
    #[must_use]
    pub fn new(profile_id: ProfileId) -> Self {
        Self {
            payload: String::new(),
            profile_id,
            logo_enabled: false,
            revision: Revision(0),
            preview_state: PreviewState::Invalid(WorkflowFailure::EmptyPayload),
        }
    }

    pub fn set_payload(&mut self, payload: String) -> PreviewRequest {
        self.payload = payload;
        self.begin_preview()
    }

    pub fn select_profile(&mut self, profile_id: ProfileId) -> PreviewRequest {
        self.profile_id = profile_id;
        self.begin_preview()
    }

    pub fn set_logo_enabled(&mut self, enabled: bool) -> PreviewRequest {
        self.logo_enabled = enabled;
        self.begin_preview()
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    #[must_use]
    pub fn character_count(&self) -> usize {
        self.payload.chars().count()
    }

    #[must_use]
    pub fn byte_count(&self) -> usize {
        self.payload.len()
    }

    #[must_use]
    pub const fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub fn preview(&self) -> Option<&Preview> {
        match &self.preview_state {
            PreviewState::Ready(preview) => Some(preview),
            PreviewState::Pending | PreviewState::Invalid(_) => None,
        }
    }

    #[must_use]
    pub const fn exports_enabled(&self) -> bool {
        matches!(self.preview_state, PreviewState::Ready(_))
    }

    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self.preview_state, PreviewState::Pending)
    }

    #[must_use]
    pub fn validation_message(&self) -> Option<String> {
        match &self.preview_state {
            PreviewState::Invalid(failure) => Some(failure.message()),
            PreviewState::Pending | PreviewState::Ready(_) => None,
        }
    }

    #[must_use]
    pub fn caution(&self) -> Option<&'static str> {
        control_character_caution(&self.payload)
    }

    pub fn complete_preview(
        &mut self,
        revision: Revision,
        result: Result<Preview, WorkflowFailure>,
    ) -> bool {
        if revision != self.revision {
            return false;
        }
        self.preview_state = match result {
            Ok(preview) => PreviewState::Ready(preview),
            Err(failure) => PreviewState::Invalid(failure),
        };
        true
    }

    fn begin_preview(&mut self) -> PreviewRequest {
        self.revision.0 = self.revision.0.wrapping_add(1);
        self.preview_state = PreviewState::Pending;
        PreviewRequest {
            revision: self.revision,
            payload: self.payload.clone(),
            profile_id: self.profile_id,
            logo_enabled: self.logo_enabled,
        }
    }
}

pub fn evaluate_preview(request: &PreviewRequest) -> Result<Preview, WorkflowFailure> {
    let profile = supported_profile(request.profile_id).ok_or(WorkflowFailure::Internal)?;
    let encoded = encode(EncodeRequest {
        text: request.payload(),
        ecc: request.ecc(),
        max_version: profile.maximum_version(),
    })
    .map_err(|error| classify_encode_error(error, profile.maximum_version()))?;
    let options = RenderOptions::safe(profile).map_err(|_| WorkflowFailure::Internal)?;
    let model = RenderModel::new(&encoded, options).map_err(|_| WorkflowFailure::Internal)?;
    let svg = render_svg(&model).map_err(|_| WorkflowFailure::Internal)?;
    let data_codewords =
        u16::try_from(encoded.data_bits_capacity() / 8).map_err(|_| WorkflowFailure::Internal)?;
    Ok(Preview {
        svg,
        diagnostics: Diagnostics {
            ecc: encoded.ecc(),
            maximum_version: profile.maximum_version(),
            selected_version: encoded.version(),
            used_data_bits: encoded.data_bits_used(),
            available_data_bits: encoded.data_bits_capacity(),
            data_codewords,
            matrix_modules: encoded.version().symbol_size(),
            svg_side_pixels: profile.svg_dimensions().width().get(),
            png_side_pixels: profile.png_dimensions().width().get(),
            print_guidance: print_guidance(profile.id()),
        },
    })
}

fn supported_profile(profile_id: ProfileId) -> Option<OutputProfile> {
    SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == profile_id)
}

fn classify_encode_error(error: EncodeError, maximum_version: Version) -> WorkflowFailure {
    match error {
        EncodeError::Payload(EncodingError::EmptyPayload) => WorkflowFailure::EmptyPayload,
        EncodeError::Payload(EncodingError::InputLimitExceeded {
            byte_length,
            maximum,
        }) => WorkflowFailure::InputLimitExceeded {
            byte_length,
            maximum,
        },
        EncodeError::Payload(
            EncodingError::PayloadTooLargeForProfile { .. } | EncodingError::PayloadTooLargeForQr,
        ) => WorkflowFailure::OverCapacity { maximum_version },
        _ => WorkflowFailure::Internal,
    }
}

const fn print_guidance(profile_id: ProfileId) -> &'static str {
    match profile_id {
        ProfileId::Print => "Place at 25–30 mm or larger; validate for the actual environment.",
        ProfileId::Inline | ProfileId::Content | ProfileId::Landing => {
            "Use SVG when resizing and validate the QR code in its final environment."
        }
    }
}

#[must_use]
fn control_character_caution(payload: &str) -> Option<&'static str> {
    payload
        .chars()
        .any(char::is_control)
        .then_some(CONTROL_CHARACTER_CAUTION)
}
