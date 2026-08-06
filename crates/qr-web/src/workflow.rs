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
const SAFE_OUTPUT_GUIDANCE: &str =
    "Use SVG when resizing and validate the QR code in its final environment.";
const PRINT_OUTPUT_GUIDANCE: &str =
    "Place at 25–30 mm or larger; validate for the actual environment.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfilePresentation {
    name: &'static str,
    value: &'static str,
    guidance: &'static str,
}

impl ProfilePresentation {
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    #[must_use]
    pub const fn value(self) -> &'static str {
        self.value
    }

    #[must_use]
    pub const fn guidance(self) -> &'static str {
        self.guidance
    }
}

#[must_use]
pub const fn profile_presentation(profile_id: ProfileId) -> ProfilePresentation {
    match profile_id {
        ProfileId::Inline => ProfilePresentation {
            name: "Inline",
            value: "inline",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::Content => ProfilePresentation {
            name: "Content",
            value: "content",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::Landing => ProfilePresentation {
            name: "Landing",
            value: "landing",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::Print => ProfilePresentation {
            name: "Print",
            value: "print",
            guidance: PRINT_OUTPUT_GUIDANCE,
        },
    }
}

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

    pub fn set_payload(&mut self, payload: String) -> Result<PreviewRequest, WorkflowFailure> {
        self.payload = payload;
        self.begin_preview()
    }

    pub fn set_display_payload(
        &mut self,
        display_payload: String,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        let previous_display = self.textarea_value();
        let prefix_bytes = common_prefix_bytes(&previous_display, &display_payload);
        let (previous_suffix_bytes, display_suffix_bytes) =
            common_suffix_bytes(&previous_display, &display_payload, prefix_bytes);
        let replaced_end = previous_display
            .len()
            .checked_sub(previous_suffix_bytes)
            .ok_or(WorkflowFailure::Internal)?;
        let inserted_end = display_payload
            .len()
            .checked_sub(display_suffix_bytes)
            .ok_or(WorkflowFailure::Internal)?;
        let start_utf16 = utf16_length(&previous_display[..prefix_bytes])?;
        let end_utf16 = utf16_length(&previous_display[..replaced_end])?;
        self.replace_display_range(
            start_utf16,
            end_utf16,
            &display_payload[prefix_bytes..inserted_end],
        )
    }

    pub fn replace_display_range(
        &mut self,
        start_utf16: u32,
        end_utf16: u32,
        inserted_raw: &str,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        if start_utf16 > end_utf16 {
            return Err(WorkflowFailure::Internal);
        }
        let start_byte = raw_byte_at_display_utf16(&self.payload, start_utf16)
            .ok_or(WorkflowFailure::Internal)?;
        let end_byte =
            raw_byte_at_display_utf16(&self.payload, end_utf16).ok_or(WorkflowFailure::Internal)?;
        let capacity = start_byte
            .checked_add(inserted_raw.len())
            .and_then(|length| length.checked_add(self.payload.len().checked_sub(end_byte)?))
            .ok_or(WorkflowFailure::Internal)?;
        let mut updated = String::new();
        updated
            .try_reserve(capacity)
            .map_err(|_| WorkflowFailure::Internal)?;
        updated.push_str(&self.payload[..start_byte]);
        updated.push_str(inserted_raw);
        updated.push_str(&self.payload[end_byte..]);
        self.payload = updated;
        self.begin_preview()
    }

    pub fn select_profile(
        &mut self,
        profile_id: ProfileId,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        self.profile_id = profile_id;
        self.begin_preview()
    }

    pub fn set_logo_enabled(&mut self, enabled: bool) -> Result<PreviewRequest, WorkflowFailure> {
        self.logo_enabled = enabled;
        self.begin_preview()
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    #[must_use]
    pub fn textarea_value(&self) -> String {
        normalize_textarea_line_endings(&self.payload)
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

    fn begin_preview(&mut self) -> Result<PreviewRequest, WorkflowFailure> {
        let Some(next_revision) = self.revision.0.checked_add(1) else {
            self.preview_state = PreviewState::Invalid(WorkflowFailure::Internal);
            return Err(WorkflowFailure::Internal);
        };
        self.revision.0 = next_revision;
        self.preview_state = PreviewState::Pending;
        Ok(PreviewRequest {
            revision: self.revision,
            payload: self.payload.clone(),
            profile_id: self.profile_id,
            logo_enabled: self.logo_enabled,
        })
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
            print_guidance: profile_presentation(profile.id()).guidance(),
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

fn normalize_textarea_line_endings(payload: &str) -> String {
    let mut normalized = String::with_capacity(payload.len());
    let mut characters = payload.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                _ = characters.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    normalized
}

fn raw_byte_at_display_utf16(payload: &str, requested: u32) -> Option<usize> {
    let mut raw_byte = 0_usize;
    let mut display_utf16 = 0_u32;
    while raw_byte < payload.len() {
        if display_utf16 == requested {
            return Some(raw_byte);
        }
        let remainder = payload.get(raw_byte..)?;
        let character = remainder.chars().next()?;
        let (raw_width, display_width) = if character == '\r' {
            (if remainder.starts_with("\r\n") { 2 } else { 1 }, 1)
        } else {
            (
                character.len_utf8(),
                u32::try_from(character.len_utf16()).ok()?,
            )
        };
        raw_byte = raw_byte.checked_add(raw_width)?;
        display_utf16 = display_utf16.checked_add(display_width)?;
        if display_utf16 > requested {
            return None;
        }
    }
    (display_utf16 == requested).then_some(raw_byte)
}

fn common_prefix_bytes(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

fn common_suffix_bytes(left: &str, right: &str, prefix_bytes: usize) -> (usize, usize) {
    let left_limit = left.len().saturating_sub(prefix_bytes);
    let right_limit = right.len().saturating_sub(prefix_bytes);
    let mut left_bytes = 0_usize;
    let mut right_bytes = 0_usize;
    for (left_character, right_character) in left.chars().rev().zip(right.chars().rev()) {
        let Some(next_left_bytes) = left_bytes.checked_add(left_character.len_utf8()) else {
            break;
        };
        let Some(next_right_bytes) = right_bytes.checked_add(right_character.len_utf8()) else {
            break;
        };
        if left_character != right_character
            || next_left_bytes > left_limit
            || next_right_bytes > right_limit
        {
            break;
        }
        left_bytes = next_left_bytes;
        right_bytes = next_right_bytes;
    }
    (left_bytes, right_bytes)
}

fn utf16_length(value: &str) -> Result<u32, WorkflowFailure> {
    u32::try_from(value.encode_utf16().count()).map_err(|_| WorkflowFailure::Internal)
}

#[must_use]
fn control_character_caution(payload: &str) -> Option<&'static str> {
    payload
        .chars()
        .any(char::is_control)
        .then_some(CONTROL_CHARACTER_CAUTION)
}
