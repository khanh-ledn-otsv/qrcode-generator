//! JSON messages exchanged with the browser preview worker.

use qr_core::encoding::EciAssignment;
use qr_core::matrix::MaskId;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_core::{EncodingMode, Version};
use qr_render::{ContrastRatio, LogoStyle, OutputSafety, ProfileId};
use serde::{Deserialize, Serialize};

use super::{
    Diagnostics, LogoDiagnostics, Preview, PreviewRequest, Revision, WorkflowFailure,
    evaluate_preview, profile_presentation,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerRequest {
    revision: u64,
    payload: String,
    profile: u8,
    logo_enabled: bool,
}

impl WorkerRequest {
    #[must_use]
    pub fn from_preview(request: &PreviewRequest) -> Self {
        Self {
            revision: request.revision.0,
            payload: request.payload.clone(),
            profile: profile_number(request.profile_id),
            logo_enabled: request.logo_enabled,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    fn into_preview_request(self) -> Result<PreviewRequest, ProtocolError> {
        Ok(PreviewRequest {
            revision: Revision(self.revision),
            payload: self.payload,
            profile_id: profile_from_number(self.profile).ok_or(ProtocolError::InvalidMessage)?,
            logo_enabled: self.logo_enabled,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerResponse {
    revision: u64,
    result: WorkerResult,
}

impl WorkerResponse {
    pub fn evaluate_json(json: &str) -> Result<Self, ProtocolError> {
        let request: WorkerRequest = serde_json::from_str(json)?;
        let request = request.into_preview_request()?;
        let revision = request.revision.0;
        let result = match evaluate_preview(&request) {
            Ok(preview) => WorkerResult::Ready(WirePreview::from_preview(preview)),
            Err(failure) => WorkerResult::Failed(WireFailure::from_failure(failure)),
        };
        Ok(Self { revision, result })
    }

    pub fn into_message_parts(mut self) -> Result<(String, Vec<u8>), ProtocolError> {
        let png = match &mut self.result {
            WorkerResult::Ready(preview) => std::mem::take(&mut preview.png),
            WorkerResult::Failed(_) => Vec::new(),
        };
        Ok((serde_json::to_string(&self)?, png))
    }

    pub fn from_message_parts(json: &str, png: Vec<u8>) -> Result<Self, ProtocolError> {
        let mut response: Self = serde_json::from_str(json)?;
        match &mut response.result {
            WorkerResult::Ready(preview) => preview.png = png,
            WorkerResult::Failed(_) if !png.is_empty() => {
                return Err(ProtocolError::InvalidMessage);
            }
            WorkerResult::Failed(_) => {}
        }
        Ok(response)
    }

    pub fn into_preview_result(self) -> (Revision, Result<Preview, WorkflowFailure>) {
        let result = match self.result {
            WorkerResult::Ready(preview) => preview.into_preview(),
            WorkerResult::Failed(failure) => Err(failure.into_failure()),
        };
        (Revision(self.revision), result)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidMessage,
    Json,
}

impl From<serde_json::Error> for ProtocolError {
    fn from(_: serde_json::Error) -> Self {
        Self::Json
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", content = "value")]
enum WorkerResult {
    Ready(WirePreview),
    Failed(WireFailure),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct WirePreview {
    svg: String,
    #[serde(skip)]
    png: Vec<u8>,
    diagnostics: WireDiagnostics,
}

impl WirePreview {
    fn from_preview(preview: Preview) -> Self {
        Self {
            svg: preview.svg,
            png: preview.png,
            diagnostics: WireDiagnostics::from_diagnostics(preview.diagnostics),
        }
    }

    fn into_preview(self) -> Result<Preview, WorkflowFailure> {
        Ok(Preview {
            svg: self.svg,
            png: self.png,
            diagnostics: self.diagnostics.into_diagnostics()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct WireDiagnostics {
    mode: u8,
    single_mode: Option<u8>,
    eci_utf8: bool,
    ecc: u8,
    mask: u8,
    minimum_version: u8,
    maximum_version: u8,
    selected_version: u8,
    branding_increased_version: bool,
    used_data_bits: u32,
    available_data_bits: u32,
    data_codewords: u16,
    matrix_modules: u16,
    svg_side_pixels: u32,
    png_side_pixels: u32,
    quiet_zone_modules: u32,
    module_scale: u32,
    rendered_symbol_side_pixels: u32,
    outer_padding_per_side: u32,
    profile: u8,
    safety: u8,
    contrast_hundredths: u16,
    logo_style: u8,
    logo_placement: Option<WireLogoDiagnostics>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct WireLogoDiagnostics {
    source_left: u32,
    source_top: u32,
    source_width: u32,
    source_height: u32,
    knockout_left: u32,
    knockout_top: u32,
    knockout_width: u32,
    knockout_height: u32,
    protected_clearance: u32,
    obscured_data_modules: u32,
    obscured_remainder_modules: u32,
}

impl From<LogoDiagnostics> for WireLogoDiagnostics {
    fn from(value: LogoDiagnostics) -> Self {
        Self {
            source_left: value.source_left_ten_thousandths(),
            source_top: value.source_top_ten_thousandths(),
            source_width: value.source_width_ten_thousandths(),
            source_height: value.source_height_ten_thousandths(),
            knockout_left: value.knockout_left(),
            knockout_top: value.knockout_top(),
            knockout_width: value.knockout_width(),
            knockout_height: value.knockout_height(),
            protected_clearance: value.protected_clearance(),
            obscured_data_modules: value.obscured_data_modules(),
            obscured_remainder_modules: value.obscured_remainder_modules(),
        }
    }
}

impl From<WireLogoDiagnostics> for LogoDiagnostics {
    fn from(value: WireLogoDiagnostics) -> Self {
        Self {
            source_left: value.source_left,
            source_top: value.source_top,
            source_width: value.source_width,
            source_height: value.source_height,
            knockout_left: value.knockout_left,
            knockout_top: value.knockout_top,
            knockout_width: value.knockout_width,
            knockout_height: value.knockout_height,
            protected_clearance: value.protected_clearance,
            obscured_data_modules: value.obscured_data_modules,
            obscured_remainder_modules: value.obscured_remainder_modules,
        }
    }
}

impl WireDiagnostics {
    fn from_diagnostics(value: Diagnostics) -> Self {
        let (mode, single_mode) = match value.mode {
            EncodingMode::Single(mode) => (0, Some(data_mode_number(mode))),
            EncodingMode::Mixed => (1, None),
        };
        Self {
            mode,
            single_mode,
            eci_utf8: value.eci_assignment == Some(EciAssignment::Utf8),
            ecc: ecc_number(value.ecc),
            mask: value.mask.number(),
            minimum_version: value.minimum_version.number(),
            maximum_version: value.maximum_version.number(),
            selected_version: value.selected_version.number(),
            branding_increased_version: value.branding_increased_version,
            used_data_bits: value.used_data_bits,
            available_data_bits: value.available_data_bits,
            data_codewords: value.data_codewords,
            matrix_modules: value.matrix_modules,
            svg_side_pixels: value.svg_side_pixels,
            png_side_pixels: value.png_side_pixels,
            quiet_zone_modules: value.quiet_zone_modules,
            module_scale: value.module_scale,
            rendered_symbol_side_pixels: value.rendered_symbol_side_pixels,
            outer_padding_per_side: value.outer_padding_per_side,
            profile: profile_number(value.profile_id),
            safety: matches!(value.safety, OutputSafety::Caution).into(),
            contrast_hundredths: value.contrast_ratio.hundredths(),
            logo_style: matches!(value.logo_style, LogoStyle::Bundled).into(),
            logo_placement: value.logo_placement.map(Into::into),
        }
    }

    fn into_diagnostics(self) -> Result<Diagnostics, WorkflowFailure> {
        let profile = profile_from_number(self.profile).ok_or(WorkflowFailure::Internal)?;
        let mode = match (self.mode, self.single_mode) {
            (0, Some(mode)) => {
                EncodingMode::Single(data_mode_from_number(mode).ok_or(WorkflowFailure::Internal)?)
            }
            (1, None) => EncodingMode::Mixed,
            _ => return Err(WorkflowFailure::Internal),
        };
        Ok(Diagnostics {
            profile_id: profile,
            mode,
            eci_assignment: self.eci_utf8.then_some(EciAssignment::Utf8),
            ecc: ecc_from_number(self.ecc).ok_or(WorkflowFailure::Internal)?,
            mask: MaskId::new(self.mask).map_err(|_| WorkflowFailure::Internal)?,
            minimum_version: Version::new(self.minimum_version)
                .map_err(|_| WorkflowFailure::Internal)?,
            maximum_version: Version::new(self.maximum_version)
                .map_err(|_| WorkflowFailure::Internal)?,
            selected_version: Version::new(self.selected_version)
                .map_err(|_| WorkflowFailure::Internal)?,
            branding_increased_version: self.branding_increased_version,
            used_data_bits: self.used_data_bits,
            available_data_bits: self.available_data_bits,
            data_codewords: self.data_codewords,
            matrix_modules: self.matrix_modules,
            svg_side_pixels: self.svg_side_pixels,
            png_side_pixels: self.png_side_pixels,
            quiet_zone_modules: self.quiet_zone_modules,
            module_scale: self.module_scale,
            rendered_symbol_side_pixels: self.rendered_symbol_side_pixels,
            outer_padding_per_side: self.outer_padding_per_side,
            print_guidance: profile_presentation(profile).guidance(),
            safety: if self.safety == 0 {
                OutputSafety::Safe
            } else {
                OutputSafety::Caution
            },
            contrast_ratio: ContrastRatio::from_hundredths(self.contrast_hundredths),
            logo_style: if self.logo_style == 0 {
                LogoStyle::None
            } else {
                LogoStyle::Bundled
            },
            logo_placement: self.logo_placement.map(Into::into),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum WireFailure {
    EmptyPayload,
    InputLimitExceeded {
        byte_length: usize,
        maximum: usize,
    },
    OverCapacity {
        maximum_version: u8,
        adaptive_recommended: bool,
    },
    LogoMinimumUnavailable {
        minimum_version: u8,
        maximum_version: u8,
        profile: u8,
    },
    UnsafeLogoGeometry {
        adaptive_recommended: bool,
    },
    Internal,
}

impl WireFailure {
    fn from_failure(value: WorkflowFailure) -> Self {
        match value {
            WorkflowFailure::EmptyPayload => Self::EmptyPayload,
            WorkflowFailure::InputLimitExceeded {
                byte_length,
                maximum,
            } => Self::InputLimitExceeded {
                byte_length,
                maximum,
            },
            WorkflowFailure::OverCapacity {
                maximum_version,
                adaptive_recommended,
            } => Self::OverCapacity {
                maximum_version: maximum_version.number(),
                adaptive_recommended,
            },
            WorkflowFailure::LogoMinimumUnavailable {
                minimum_version,
                maximum_version,
                profile_name,
            } => Self::LogoMinimumUnavailable {
                minimum_version: minimum_version.number(),
                maximum_version: maximum_version.number(),
                profile: profile_from_name(profile_name),
            },
            WorkflowFailure::UnsafeLogoGeometry {
                adaptive_recommended,
            } => Self::UnsafeLogoGeometry {
                adaptive_recommended,
            },
            WorkflowFailure::Internal => Self::Internal,
        }
    }

    fn into_failure(self) -> WorkflowFailure {
        match self {
            Self::EmptyPayload => WorkflowFailure::EmptyPayload,
            Self::InputLimitExceeded {
                byte_length,
                maximum,
            } => WorkflowFailure::InputLimitExceeded {
                byte_length,
                maximum,
            },
            Self::OverCapacity {
                maximum_version,
                adaptive_recommended,
            } => version(maximum_version).map_or(WorkflowFailure::Internal, |maximum_version| {
                WorkflowFailure::OverCapacity {
                    maximum_version,
                    adaptive_recommended,
                }
            }),
            Self::LogoMinimumUnavailable {
                minimum_version,
                maximum_version,
                profile,
            } => {
                match (
                    version(minimum_version),
                    version(maximum_version),
                    profile_from_number(profile),
                ) {
                    (Some(minimum_version), Some(maximum_version), Some(profile)) => {
                        WorkflowFailure::LogoMinimumUnavailable {
                            minimum_version,
                            maximum_version,
                            profile_name: profile_presentation(profile).name(),
                        }
                    }
                    _ => WorkflowFailure::Internal,
                }
            }
            Self::UnsafeLogoGeometry {
                adaptive_recommended,
            } => WorkflowFailure::UnsafeLogoGeometry {
                adaptive_recommended,
            },
            Self::Internal => WorkflowFailure::Internal,
        }
    }
}

fn version(number: u8) -> Option<Version> {
    Version::new(number).ok()
}

const fn profile_number(profile: ProfileId) -> u8 {
    match profile {
        ProfileId::Inline => 0,
        ProfileId::Content => 1,
        ProfileId::Landing => 2,
        ProfileId::Print => 3,
        ProfileId::Adaptive => 4,
    }
}

const fn profile_from_number(number: u8) -> Option<ProfileId> {
    match number {
        0 => Some(ProfileId::Inline),
        1 => Some(ProfileId::Content),
        2 => Some(ProfileId::Landing),
        3 => Some(ProfileId::Print),
        4 => Some(ProfileId::Adaptive),
        _ => None,
    }
}

fn profile_from_name(name: &str) -> u8 {
    [
        ProfileId::Inline,
        ProfileId::Content,
        ProfileId::Landing,
        ProfileId::Print,
        ProfileId::Adaptive,
    ]
    .into_iter()
    .find(|profile| profile_presentation(*profile).name() == name)
    .map_or(u8::MAX, profile_number)
}

const fn data_mode_number(mode: DataMode) -> u8 {
    match mode {
        DataMode::Numeric => 0,
        DataMode::Alphanumeric => 1,
        DataMode::Byte => 2,
        DataMode::Kanji => 3,
    }
}
const fn data_mode_from_number(number: u8) -> Option<DataMode> {
    match number {
        0 => Some(DataMode::Numeric),
        1 => Some(DataMode::Alphanumeric),
        2 => Some(DataMode::Byte),
        3 => Some(DataMode::Kanji),
        _ => None,
    }
}
const fn ecc_number(ecc: ErrorCorrection) -> u8 {
    match ecc {
        ErrorCorrection::Low => 0,
        ErrorCorrection::Medium => 1,
        ErrorCorrection::Quartile => 2,
        ErrorCorrection::High => 3,
    }
}
const fn ecc_from_number(number: u8) -> Option<ErrorCorrection> {
    match number {
        0 => Some(ErrorCorrection::Low),
        1 => Some(ErrorCorrection::Medium),
        2 => Some(ErrorCorrection::Quartile),
        3 => Some(ErrorCorrection::High),
        _ => None,
    }
}
