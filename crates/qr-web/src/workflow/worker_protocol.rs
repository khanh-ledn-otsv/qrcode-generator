//! JSON messages exchanged with the browser preview worker.

use qr_core::encoding::EciAssignment;
use qr_core::matrix::MaskId;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_core::{EncodingMode, Version};
use qr_render::{ContrastRatio, ForegroundTheme, LogoStyle, OutputSafety, ProfileId};
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
    foreground_theme: u8,
    logo_enabled: bool,
}

impl WorkerRequest {
    #[must_use]
    pub fn from_preview(request: &PreviewRequest) -> Self {
        Self {
            revision: request.revision.0,
            payload: request.payload.clone(),
            profile: profile_number(request.profile_id),
            foreground_theme: foreground_theme_number(request.foreground_theme),
            logo_enabled: request.logo_enabled,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    #[must_use]
    pub fn revision_from_json(json: &str) -> Option<u64> {
        serde_json::from_str::<Self>(json)
            .ok()
            .map(|request| request.revision)
    }

    fn into_preview_request(self) -> Result<PreviewRequest, ProtocolError> {
        Ok(PreviewRequest {
            revision: Revision(self.revision),
            payload: self.payload,
            profile_id: profile_from_number(self.profile).ok_or(ProtocolError::InvalidMessage)?,
            foreground_theme: foreground_theme_from_number(self.foreground_theme)
                .ok_or(ProtocolError::InvalidMessage)?,
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
            WorkerResult::Ready(preview) if is_png(&png) => preview.png = png,
            WorkerResult::Ready(_) => return Err(ProtocolError::InvalidMessage),
            WorkerResult::Failed(_) if !png.is_empty() => {
                return Err(ProtocolError::InvalidMessage);
            }
            WorkerResult::Failed(_) => {}
        }
        response.validate()?;
        Ok(response)
    }

    #[must_use]
    pub const fn revision(&self) -> Revision {
        Revision(self.revision)
    }

    #[must_use]
    pub const fn revision_number(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn revision_from_json(json: &str) -> Option<Revision> {
        #[derive(Deserialize)]
        struct RevisionOnly {
            revision: u64,
        }

        serde_json::from_str::<RevisionOnly>(json)
            .ok()
            .map(|value| Revision(value.revision))
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        match &self.result {
            WorkerResult::Ready(preview) => preview.validate(),
            WorkerResult::Failed(_) => Ok(()),
        }
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

    fn validate(&self) -> Result<(), ProtocolError> {
        if !valid_svg(&self.svg, self.diagnostics.svg_side_pixels)
            || !valid_png(&self.png, self.diagnostics.png_side_pixels)
        {
            return Err(ProtocolError::InvalidMessage);
        }
        self.diagnostics.validate()
    }
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}

fn valid_png(bytes: &[u8], side: u32) -> bool {
    bytes.len() >= 45
        && is_png(bytes)
        && bytes.get(8..16) == Some(&[0, 0, 0, 13, b'I', b'H', b'D', b'R'])
        && bytes.get(16..20) == Some(side.to_be_bytes().as_slice())
        && bytes.get(20..24) == Some(side.to_be_bytes().as_slice())
        && bytes.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82])
}

fn valid_svg(svg: &str, side: u32) -> bool {
    let prefix = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{side}\"");
    svg.starts_with(&prefix) && svg.ends_with("</svg>")
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
    foreground_theme: u8,
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
            foreground_theme: foreground_theme_number(value.foreground_theme),
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
            foreground_theme: foreground_theme_from_number(self.foreground_theme)
                .ok_or(WorkflowFailure::Internal)?,
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

    fn validate(&self) -> Result<(), ProtocolError> {
        let selected =
            Version::new(self.selected_version).map_err(|_| ProtocolError::InvalidMessage)?;
        let minimum =
            Version::new(self.minimum_version).map_err(|_| ProtocolError::InvalidMessage)?;
        let maximum =
            Version::new(self.maximum_version).map_err(|_| ProtocolError::InvalidMessage)?;
        let valid_mode = matches!((self.mode, self.single_mode), (0, Some(0..=3)) | (1, None));
        let valid_logo = matches!(self.logo_style, 0..=1)
            && (self.logo_style == 1) == self.logo_placement.is_some();
        let profile = profile_from_number(self.profile).ok_or(ProtocolError::InvalidMessage)?;
        let compiled_profile =
            super::supported_profile(profile).ok_or(ProtocolError::InvalidMessage)?;
        let expected_svg = compiled_profile
            .svg_dimensions_for(selected)
            .map_err(|_| ProtocolError::InvalidMessage)?
            .width()
            .get();
        let geometry = compiled_profile
            .geometry(selected)
            .map_err(|_| ProtocolError::InvalidMessage)?;
        let expected_data_codewords = qr_core::tables::lookup(
            selected,
            ecc_from_number(self.ecc).ok_or(ProtocolError::InvalidMessage)?,
        )
        .map_err(|_| ProtocolError::InvalidMessage)?
        .data_codewords();
        let valid_dimensions = self.matrix_modules == selected.symbol_size()
            && self
                .rendered_symbol_side_pixels
                .checked_add(self.outer_padding_per_side.saturating_mul(2))
                == Some(self.png_side_pixels)
            && self.svg_side_pixels == expected_svg
            && self.png_side_pixels == geometry.canvas_dimensions().width().get()
            && self.quiet_zone_modules == geometry.symbol().quiet_zone_modules_per_side().get()
            && self.module_scale == geometry.module_scale().get()
            && self.rendered_symbol_side_pixels
                == geometry.rendered_symbol_dimensions().width().get()
            && self.outer_padding_per_side == geometry.outer_padding().left.get();
        if ecc_from_number(self.ecc).is_none()
            || MaskId::new(self.mask).is_err()
            || minimum > selected
            || selected > maximum
            || !valid_mode
            || !matches!(self.safety, 0..=1)
            || !valid_logo
            || self.used_data_bits > self.available_data_bits
            || self.available_data_bits != u32::from(expected_data_codewords) * 8
            || self.data_codewords != expected_data_codewords
            || self.maximum_version != compiled_profile.maximum_version().number()
            || foreground_theme_from_number(self.foreground_theme).is_none()
            || self.contrast_hundredths
                != expected_contrast_hundredths(
                    foreground_theme_from_number(self.foreground_theme)
                        .ok_or(ProtocolError::InvalidMessage)?,
                )
            || self.safety != self.logo_style
            || !valid_dimensions
        {
            return Err(ProtocolError::InvalidMessage);
        }
        Ok(())
    }
}

const fn foreground_theme_number(theme: ForegroundTheme) -> u8 {
    match theme {
        ForegroundTheme::Magenta => 0,
        ForegroundTheme::Black => 1,
    }
}

const fn foreground_theme_from_number(number: u8) -> Option<ForegroundTheme> {
    match number {
        0 => Some(ForegroundTheme::Magenta),
        1 => Some(ForegroundTheme::Black),
        _ => None,
    }
}

const fn expected_contrast_hundredths(theme: ForegroundTheme) -> u16 {
    match theme {
        ForegroundTheme::Magenta => 604,
        ForegroundTheme::Black => 2100,
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
    },
    LogoMinimumUnavailable {
        minimum_version: u8,
        maximum_version: u8,
        profile: u8,
    },
    UnsafeLogoGeometry,
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
            WorkflowFailure::OverCapacity { maximum_version } => Self::OverCapacity {
                maximum_version: maximum_version.number(),
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
            WorkflowFailure::UnsafeLogoGeometry {} => Self::UnsafeLogoGeometry,
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
            Self::OverCapacity { maximum_version } => version(maximum_version)
                .map_or(WorkflowFailure::Internal, |maximum_version| {
                    WorkflowFailure::OverCapacity { maximum_version }
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
            Self::UnsafeLogoGeometry => WorkflowFailure::UnsafeLogoGeometry {},
            Self::Internal => WorkflowFailure::Internal,
        }
    }
}

fn version(number: u8) -> Option<Version> {
    Version::new(number).ok()
}

const fn profile_number(profile: ProfileId) -> u8 {
    match profile {
        ProfileId::Small => 0,
        ProfileId::Standard => 1,
        ProfileId::PrimaryCta => 2,
        ProfileId::HeroCampaign => 3,
        ProfileId::BusinessCard => 4,
        ProfileId::FlyerBrochure => 5,
        ProfileId::PosterPackage => 6,
    }
}

const fn profile_from_number(number: u8) -> Option<ProfileId> {
    match number {
        0 => Some(ProfileId::Small),
        1 => Some(ProfileId::Standard),
        2 => Some(ProfileId::PrimaryCta),
        3 => Some(ProfileId::HeroCampaign),
        4 => Some(ProfileId::BusinessCard),
        5 => Some(ProfileId::FlyerBrochure),
        6 => Some(ProfileId::PosterPackage),
        _ => None,
    }
}

fn profile_from_name(name: &str) -> u8 {
    [
        ProfileId::Small,
        ProfileId::Standard,
        ProfileId::PrimaryCta,
        ProfileId::HeroCampaign,
        ProfileId::BusinessCard,
        ProfileId::FlyerBrochure,
        ProfileId::PosterPackage,
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
