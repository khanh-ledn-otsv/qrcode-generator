//! Plain-Rust state transitions for the interactive QR workflow.

use qr_core::encoding::{EciAssignment, EncodingError};
use qr_core::matrix::MaskId;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_core::{EncodeError, EncodeRequest, EncodingMode, Version, encode};
use qr_render::{
    BRANDED_LOGO_VERSION, ContrastRatio, ForegroundTheme, LogoStyle, OutputProfile, OutputSafety,
    ProfileId, RenderError, RenderModel, RenderOptions, Rgba, SUPPORTED_PROFILES, render_png,
    render_svg,
};

use crate::textarea::{TextAreaBuffer, projected_utf16_length};

pub mod worker_protocol;

const CONTROL_CHARACTER_CAUTION: &str =
    "This payload contains control characters. Confirm that they are intentional.";
const LOGO_OUTPUT_CAUTION: &str = "The bundled logo obscures QR data modules. Validate the exported code in its actual environment.";
const LOGO_CAPACITY_FALLBACK_REASON: &str = "payload exceeds branded capacity";
const INTERNAL_FAILURE_MESSAGE: &str =
    "QR generation failed unexpectedly. Change the input and try again.";
const SAFE_OUTPUT_GUIDANCE: &str =
    "Use SVG when resizing and validate the QR code in its final environment.";
const PRINT_OUTPUT_GUIDANCE: &str =
    "150 dpi artifact policy; test the final material, device, and surface.";

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
        ProfileId::Small => ProfilePresentation {
            name: "Small",
            value: "small",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::Standard => ProfilePresentation {
            name: "Standard",
            value: "standard",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::PrimaryCta => ProfilePresentation {
            name: "Primary CTA",
            value: "primary-cta",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::HeroCampaign => ProfilePresentation {
            name: "Hero / Campaign",
            value: "hero-campaign",
            guidance: SAFE_OUTPUT_GUIDANCE,
        },
        ProfileId::BusinessCard => ProfilePresentation {
            name: "Business card",
            value: "business-card",
            guidance: PRINT_OUTPUT_GUIDANCE,
        },
        ProfileId::FlyerBrochure => ProfilePresentation {
            name: "Flyer / Brochure",
            value: "flyer-brochure",
            guidance: PRINT_OUTPUT_GUIDANCE,
        },
        ProfileId::PosterPackage => ProfilePresentation {
            name: "Poster / Package",
            value: "poster-package",
            guidance: PRINT_OUTPUT_GUIDANCE,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinkCapacityGuideRow {
    profile_id: ProfileId,
    without_logo_ascii_bytes: usize,
    with_logo_ascii_bytes: usize,
}

impl LinkCapacityGuideRow {
    #[must_use]
    pub const fn profile_id(self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn without_logo_ascii_bytes(self) -> usize {
        self.without_logo_ascii_bytes
    }

    #[must_use]
    pub const fn with_logo_ascii_bytes(self) -> usize {
        self.with_logo_ascii_bytes
    }
}

#[must_use]
pub const fn link_capacity_guide() -> [LinkCapacityGuideRow; 7] {
    [
        LinkCapacityGuideRow {
            profile_id: ProfileId::Small,
            without_logo_ascii_bytes: 106,
            with_logo_ascii_bytes: 58,
        },
        LinkCapacityGuideRow {
            profile_id: ProfileId::Standard,
            without_logo_ascii_bytes: 152,
            with_logo_ascii_bytes: 84,
        },
        LinkCapacityGuideRow {
            profile_id: ProfileId::PrimaryCta,
            without_logo_ascii_bytes: 287,
            with_logo_ascii_bytes: 137,
        },
        LinkCapacityGuideRow {
            profile_id: ProfileId::HeroCampaign,
            without_logo_ascii_bytes: 287,
            with_logo_ascii_bytes: 137,
        },
        LinkCapacityGuideRow {
            profile_id: ProfileId::BusinessCard,
            without_logo_ascii_bytes: 287,
            with_logo_ascii_bytes: 137,
        },
        LinkCapacityGuideRow {
            profile_id: ProfileId::FlyerBrochure,
            without_logo_ascii_bytes: 287,
            with_logo_ascii_bytes: 137,
        },
        LinkCapacityGuideRow {
            profile_id: ProfileId::PosterPackage,
            without_logo_ascii_bytes: 287,
            with_logo_ascii_bytes: 137,
        },
    ]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Revision(u64);

impl Revision {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_wire_number(value: f64) -> Option<Self> {
        if value.is_finite()
            && value.fract() == 0.0
            && (0.0..=9_007_199_254_740_991.0).contains(&value)
        {
            Some(Self(value as u64))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRequest {
    revision: Revision,
    payload: String,
    profile_id: ProfileId,
    foreground_theme: ForegroundTheme,
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
    pub const fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn foreground_theme(&self) -> ForegroundTheme {
        self.foreground_theme
    }

    #[must_use]
    pub const fn logo_enabled(&self) -> bool {
        self.logo_enabled
    }

    #[must_use]
    pub const fn ecc(&self) -> ErrorCorrection {
        if self.logo_enabled {
            ErrorCorrection::High
        } else {
            ErrorCorrection::Medium
        }
    }

    #[must_use]
    pub const fn minimum_version(&self) -> Version {
        if self.logo_enabled {
            BRANDED_LOGO_VERSION
        } else {
            Version::MINIMUM
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostics {
    profile_id: ProfileId,
    foreground_theme: ForegroundTheme,
    mode: EncodingMode,
    eci_assignment: Option<EciAssignment>,
    ecc: ErrorCorrection,
    mask: MaskId,
    minimum_version: Version,
    maximum_version: Version,
    selected_version: Version,
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
    print_guidance: &'static str,
    safety: OutputSafety,
    contrast_ratio: ContrastRatio,
    requested_logo_style: LogoStyle,
    logo_style: LogoStyle,
    logo_fallback_reason: Option<&'static str>,
    logo_placement: Option<LogoDiagnostics>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogoDiagnostics {
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

impl LogoDiagnostics {
    fn from_placement(placement: qr_render::LogoPlacement) -> Self {
        let source = placement.source_bounds();
        let knockout = placement.knockout_bounds();
        Self {
            source_left: source.left_ten_thousandths(),
            source_top: source.top_ten_thousandths(),
            source_width: source.width_ten_thousandths(),
            source_height: source.height_ten_thousandths(),
            knockout_left: knockout.left().get(),
            knockout_top: knockout.top().get(),
            knockout_width: knockout.width().get(),
            knockout_height: knockout.height().get(),
            protected_clearance: placement.protected_clearance(),
            obscured_data_modules: placement.obscured_data_modules(),
            obscured_remainder_modules: placement.obscured_remainder_modules(),
        }
    }

    #[must_use]
    pub const fn source_left_ten_thousandths(self) -> u32 {
        self.source_left
    }
    #[must_use]
    pub const fn source_top_ten_thousandths(self) -> u32 {
        self.source_top
    }
    #[must_use]
    pub const fn source_width_ten_thousandths(self) -> u32 {
        self.source_width
    }
    #[must_use]
    pub const fn source_height_ten_thousandths(self) -> u32 {
        self.source_height
    }
    #[must_use]
    pub const fn knockout_left(self) -> u32 {
        self.knockout_left
    }
    #[must_use]
    pub const fn knockout_top(self) -> u32 {
        self.knockout_top
    }
    #[must_use]
    pub const fn knockout_width(self) -> u32 {
        self.knockout_width
    }
    #[must_use]
    pub const fn knockout_height(self) -> u32 {
        self.knockout_height
    }
    #[must_use]
    pub const fn protected_clearance(self) -> u32 {
        self.protected_clearance
    }
    #[must_use]
    pub const fn obscured_data_modules(self) -> u32 {
        self.obscured_data_modules
    }
    #[must_use]
    pub const fn obscured_remainder_modules(self) -> u32 {
        self.obscured_remainder_modules
    }
}

impl Diagnostics {
    #[must_use]
    pub const fn profile_id(self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn foreground_theme(self) -> ForegroundTheme {
        self.foreground_theme
    }

    #[must_use]
    pub const fn mode(self) -> EncodingMode {
        self.mode
    }

    #[must_use]
    pub const fn eci_assignment(self) -> Option<EciAssignment> {
        self.eci_assignment
    }

    #[must_use]
    pub const fn ecc(self) -> ErrorCorrection {
        self.ecc
    }

    #[must_use]
    pub const fn mask(self) -> MaskId {
        self.mask
    }

    #[must_use]
    pub const fn minimum_version(self) -> Version {
        self.minimum_version
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
    pub const fn branding_increased_version(self) -> bool {
        self.branding_increased_version
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
    pub const fn quiet_zone_modules(self) -> u32 {
        self.quiet_zone_modules
    }

    #[must_use]
    pub const fn module_scale(self) -> u32 {
        self.module_scale
    }

    #[must_use]
    pub const fn rendered_symbol_side_pixels(self) -> u32 {
        self.rendered_symbol_side_pixels
    }

    #[must_use]
    pub const fn outer_padding_per_side(self) -> u32 {
        self.outer_padding_per_side
    }

    #[must_use]
    pub const fn print_guidance(self) -> &'static str {
        self.print_guidance
    }

    #[must_use]
    pub const fn foreground(self) -> Rgba {
        match self.foreground_theme {
            ForegroundTheme::Magenta => Rgba::BRAND,
            ForegroundTheme::Black => Rgba::BLACK,
        }
    }

    #[must_use]
    pub const fn background(self) -> Rgba {
        Rgba::WHITE
    }

    #[must_use]
    pub const fn safety(self) -> OutputSafety {
        self.safety
    }

    #[must_use]
    pub const fn contrast_ratio(self) -> ContrastRatio {
        self.contrast_ratio
    }

    #[must_use]
    pub const fn logo_style(self) -> LogoStyle {
        self.logo_style
    }

    #[must_use]
    pub const fn requested_logo_style(self) -> LogoStyle {
        self.requested_logo_style
    }

    #[must_use]
    pub const fn logo_fallback_reason(self) -> Option<&'static str> {
        self.logo_fallback_reason
    }

    #[must_use]
    pub const fn logo_placement(self) -> Option<LogoDiagnostics> {
        self.logo_placement
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    svg: String,
    png: Vec<u8>,
    diagnostics: Diagnostics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactKind {
    Svg,
    Png,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownloadArtifact<'preview> {
    filename: &'static str,
    mime_type: &'static str,
    bytes: &'preview [u8],
}

impl DownloadArtifact<'_> {
    #[must_use]
    pub const fn filename(self) -> &'static str {
        self.filename
    }

    #[must_use]
    pub const fn mime_type(self) -> &'static str {
        self.mime_type
    }

    #[must_use]
    pub const fn bytes(&self) -> &[u8] {
        self.bytes
    }
}

impl Preview {
    #[must_use]
    pub fn svg(&self) -> &str {
        &self.svg
    }

    #[must_use]
    pub fn artifact(&self, kind: ArtifactKind) -> DownloadArtifact<'_> {
        match kind {
            ArtifactKind::Svg => DownloadArtifact {
                filename: "qr-code.svg",
                mime_type: "image/svg+xml",
                bytes: self.svg.as_bytes(),
            },
            ArtifactKind::Png => DownloadArtifact {
                filename: "qr-code.png",
                mime_type: "image/png",
                bytes: &self.png,
            },
        }
    }

    #[must_use]
    pub fn accessible_label(&self) -> String {
        format!(
            "Generated QR code preview: {} mode, version {}, ECC {}.",
            mode_label(self.diagnostics.mode),
            self.diagnostics.selected_version.number(),
            ecc_label(self.diagnostics.ecc),
        )
    }

    #[must_use]
    pub const fn diagnostics(&self) -> Diagnostics {
        self.diagnostics
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkflowFailure {
    EmptyPayload,
    InvalidUrl,
    MissingParameterName,
    InputLimitExceeded {
        byte_length: usize,
        maximum: usize,
    },
    OverCapacity {
        maximum_version: Version,
    },
    LogoMinimumUnavailable {
        minimum_version: Version,
        maximum_version: Version,
        profile_name: &'static str,
    },
    UnsafeLogoGeometry {},
    Internal,
}

impl WorkflowFailure {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::EmptyPayload => "Enter a URL to generate a QR code.".to_owned(),
            Self::InvalidUrl => {
                "Enter a valid URL beginning with http:// or https://.".to_owned()
            }
            Self::MissingParameterName => {
                "Enter a name for each custom parameter that has a value.".to_owned()
            }
            Self::InputLimitExceeded {
                byte_length,
                maximum,
            } => format!("The payload is {byte_length} bytes; the input limit is {maximum} bytes."),
            Self::OverCapacity { maximum_version } => format!(
                "The payload does not fit this output variant's maximum QR version {}.",
                maximum_version.number(),
            ),
            Self::LogoMinimumUnavailable {
                minimum_version,
                maximum_version,
                profile_name,
            } => format!(
                "Logo mode requires QR version {} or larger, but {profile_name} supports up to version {}.",
                minimum_version.number(),
                maximum_version.number(),
            ),
            Self::UnsafeLogoGeometry {} => "Logo mode is approved only at QR Version 6 for these fixed output variants. Disable the logo to keep this exact payload.".to_owned(),
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
    payload: TextAreaBuffer,
    profile_id: ProfileId,
    foreground_theme: ForegroundTheme,
    logo_enabled: bool,
    revision: Revision,
    preview_state: PreviewState,
}

impl WorkflowState {
    #[must_use]
    pub fn new(profile_id: ProfileId) -> Self {
        Self {
            payload: TextAreaBuffer::new(String::new()),
            profile_id,
            foreground_theme: ForegroundTheme::Magenta,
            logo_enabled: true,
            revision: Revision(0),
            preview_state: PreviewState::Invalid(WorkflowFailure::EmptyPayload),
        }
    }

    pub fn set_payload(&mut self, payload: String) -> Result<PreviewRequest, WorkflowFailure> {
        self.payload.replace_raw(payload);
        self.begin_preview()
    }

    pub fn set_display_payload_at(
        &mut self,
        display_payload: String,
        edit_start_utf16: u32,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        if self
            .payload
            .replace_from_display(display_payload, edit_start_utf16)
            .is_err()
        {
            return Err(self.set_internal_failure());
        }
        self.begin_preview()
    }

    pub fn replace_display_range(
        &mut self,
        start_utf16: u32,
        end_utf16: u32,
        inserted_raw: &str,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        if self
            .payload
            .replace_display_range(start_utf16, end_utf16, inserted_raw)
            .is_err()
        {
            return Err(self.set_internal_failure());
        }
        self.begin_preview()
    }

    /// Returns the exact raw text represented by a textarea display selection.
    ///
    /// Browser textareas project CRLF and lone CR as LF. Drag operations must
    /// therefore source selected text here instead of from the DOM value.
    pub fn raw_text_for_display_range(
        &self,
        start_utf16: u32,
        end_utf16: u32,
    ) -> Result<String, WorkflowFailure> {
        self.payload
            .raw_text_for_display_range(start_utf16, end_utf16)
            .map_err(|_| WorkflowFailure::Internal)
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

    pub fn set_foreground_theme(
        &mut self,
        foreground_theme: ForegroundTheme,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        self.foreground_theme = foreground_theme;
        self.begin_preview()
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        self.payload.raw()
    }

    #[must_use]
    pub fn textarea_value(&self) -> String {
        self.payload.display()
    }

    #[must_use]
    pub fn character_count(&self) -> usize {
        self.payload.raw().chars().count()
    }

    #[must_use]
    pub fn byte_count(&self) -> usize {
        self.payload.raw().len()
    }

    #[must_use]
    pub const fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn foreground(&self) -> Rgba {
        match self.foreground_theme {
            ForegroundTheme::Magenta => Rgba::BRAND,
            ForegroundTheme::Black => Rgba::BLACK,
        }
    }

    #[must_use]
    pub const fn foreground_theme(&self) -> ForegroundTheme {
        self.foreground_theme
    }

    #[must_use]
    pub const fn background(&self) -> Rgba {
        Rgba::WHITE
    }

    #[must_use]
    pub const fn logo_enabled(&self) -> bool {
        self.logo_enabled
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
    pub fn export_disabled_reason(&self) -> Option<String> {
        match &self.preview_state {
            PreviewState::Pending => Some("QR preview is updating.".to_owned()),
            PreviewState::Invalid(failure) => Some(failure.message()),
            PreviewState::Ready(_) => None,
        }
    }

    #[must_use]
    pub fn caution(&self) -> Option<String> {
        let control = control_character_caution(self.payload.raw());
        let logo = self.logo_enabled.then_some(LOGO_OUTPUT_CAUTION);
        let cautions = [control, logo].into_iter().flatten().collect::<Vec<_>>();
        (!cautions.is_empty()).then(|| cautions.join(" "))
    }

    pub fn complete_preview(
        &mut self,
        revision: Revision,
        result: Result<Preview, WorkflowFailure>,
    ) -> bool {
        if revision != self.revision || !matches!(self.preview_state, PreviewState::Pending) {
            return false;
        }
        self.preview_state = match result {
            Ok(preview) => PreviewState::Ready(preview),
            Err(failure) => PreviewState::Invalid(failure),
        };
        true
    }

    pub fn reject_internal_failure(&mut self) {
        _ = self.set_internal_failure();
    }

    pub fn reject_url_failure(&mut self, failure: WorkflowFailure) {
        debug_assert!(matches!(
            failure,
            WorkflowFailure::InvalidUrl | WorkflowFailure::MissingParameterName
        ));
        self.preview_state = PreviewState::Invalid(failure);
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
            payload: self.payload.raw().to_owned(),
            profile_id: self.profile_id,
            foreground_theme: self.foreground_theme,
            logo_enabled: self.logo_enabled,
        })
    }

    fn set_internal_failure(&mut self) -> WorkflowFailure {
        self.preview_state = PreviewState::Invalid(WorkflowFailure::Internal);
        WorkflowFailure::Internal
    }
}

pub fn evaluate_preview(request: &PreviewRequest) -> Result<Preview, WorkflowFailure> {
    let profile = supported_profile(request.profile_id).ok_or(WorkflowFailure::Internal)?;
    let branded_minimum_version = if request.logo_enabled {
        profile.minimum_version().max(BRANDED_LOGO_VERSION)
    } else {
        profile.minimum_version()
    };
    let branded_attempt = encode(EncodeRequest::with_version_range(
        request.payload(),
        request.ecc(),
        branded_minimum_version,
        profile.maximum_version(),
    ));

    // The same payload may fit at ECC M without the logo even when the ECC-H
    // branded encoding does not; try that before surfacing a capacity error.
    let (encoded, minimum_version, logo_style, logo_fallback_reason) = match branded_attempt {
        Ok(encoded) => (
            encoded,
            branded_minimum_version,
            if request.logo_enabled {
                LogoStyle::Bundled
            } else {
                LogoStyle::None
            },
            None,
        ),
        Err(EncodeError::Payload(
            EncodingError::PayloadTooLargeForProfile { .. }
            | EncodingError::PayloadTooLargeForQr
            | EncodingError::InvalidVersionRange { .. },
        )) if request.logo_enabled => {
            let fallback_minimum_version = profile.minimum_version();
            let encoded = encode(EncodeRequest::with_version_range(
                request.payload(),
                ErrorCorrection::Medium,
                fallback_minimum_version,
                profile.maximum_version(),
            ))
            .map_err(|error| classify_encode_error(error, request, profile))?;
            (
                encoded,
                fallback_minimum_version,
                LogoStyle::None,
                Some(LOGO_CAPACITY_FALLBACK_REASON),
            )
        }
        Err(error) => return Err(classify_encode_error(error, request, profile)),
    };
    let options = RenderOptions::safe(profile)
        .and_then(|options| options.with_logo(logo_style))
        .and_then(|options| options.with_foreground_theme(request.foreground_theme))
        .map_err(|error| classify_render_error(error, profile))?;
    let model = RenderModel::new(&encoded, options)
        .map_err(|error| classify_render_error(error, profile))?;
    let svg = render_svg(&model).map_err(|_| WorkflowFailure::Internal)?;
    let png = render_png(&model).map_err(|_| WorkflowFailure::Internal)?;
    let png_placement = model.png_placement();
    let outer_padding = png_placement.outer_padding();
    if outer_padding.left != outer_padding.right
        || outer_padding.left != outer_padding.top
        || outer_padding.left != outer_padding.bottom
    {
        return Err(WorkflowFailure::Internal);
    }
    let data_codewords =
        u16::try_from(encoded.data_bits_capacity() / 8).map_err(|_| WorkflowFailure::Internal)?;
    Ok(Preview {
        svg,
        png,
        diagnostics: Diagnostics {
            profile_id: profile.id(),
            foreground_theme: model.options().foreground_theme(),
            mode: encoded.mode(),
            eci_assignment: encoded.eci_assignment(),
            ecc: encoded.ecc(),
            mask: encoded.mask(),
            minimum_version,
            maximum_version: profile.maximum_version(),
            selected_version: encoded.version(),
            branding_increased_version: request.logo_enabled
                && logo_fallback_reason.is_none()
                && encoded.minimum_version_increased_selection(),
            used_data_bits: encoded.data_bits_used(),
            available_data_bits: encoded.data_bits_capacity(),
            data_codewords,
            matrix_modules: encoded.version().symbol_size(),
            svg_side_pixels: model.svg_placement().output_dimensions().width().get(),
            png_side_pixels: png_placement.canvas_dimensions().width().get(),
            quiet_zone_modules: png_placement.symbol().quiet_zone_modules_per_side().get(),
            module_scale: png_placement.module_scale().get(),
            rendered_symbol_side_pixels: png_placement.rendered_symbol_dimensions().width().get(),
            outer_padding_per_side: outer_padding.left.get(),
            print_guidance: profile_presentation(profile.id()).guidance(),
            safety: model.options().safety(),
            contrast_ratio: model.options().contrast_ratio(),
            requested_logo_style: if request.logo_enabled {
                LogoStyle::Bundled
            } else {
                LogoStyle::None
            },
            logo_style: model.options().logo_style(),
            logo_fallback_reason: logo_fallback_reason.or_else(|| model.logo_fallback_reason()),
            logo_placement: model.logo_placement().map(LogoDiagnostics::from_placement),
        },
    })
}

#[must_use]
pub const fn mode_label(mode: EncodingMode) -> &'static str {
    match mode {
        EncodingMode::Single(DataMode::Numeric) => "Numeric",
        EncodingMode::Single(DataMode::Alphanumeric) => "Alphanumeric",
        EncodingMode::Single(DataMode::Byte) => "Byte",
        EncodingMode::Single(DataMode::Kanji) => "Kanji",
        EncodingMode::Mixed => "Mixed",
    }
}

#[must_use]
pub const fn ecc_label(ecc: ErrorCorrection) -> &'static str {
    match ecc {
        ErrorCorrection::Low => "L",
        ErrorCorrection::Medium => "M",
        ErrorCorrection::Quartile => "Q",
        ErrorCorrection::High => "H",
    }
}

#[must_use]
pub fn version_label(diagnostics: Diagnostics) -> String {
    if diagnostics.branding_increased_version() {
        format!(
            "V{} / V{} max · raised to V{} for branding",
            diagnostics.selected_version().number(),
            diagnostics.maximum_version().number(),
            diagnostics.minimum_version().number(),
        )
    } else {
        format!(
            "V{} / V{} max",
            diagnostics.selected_version().number(),
            diagnostics.maximum_version().number(),
        )
    }
}

fn supported_profile(profile_id: ProfileId) -> Option<OutputProfile> {
    SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == profile_id)
}

fn classify_encode_error(
    error: EncodeError,
    request: &PreviewRequest,
    profile: OutputProfile,
) -> WorkflowFailure {
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
        ) => WorkflowFailure::OverCapacity {
            maximum_version: profile.maximum_version(),
        },
        EncodeError::Payload(EncodingError::InvalidVersionRange { minimum, maximum })
            if request.logo_enabled =>
        {
            WorkflowFailure::LogoMinimumUnavailable {
                minimum_version: minimum,
                maximum_version: maximum,
                profile_name: profile_presentation(profile.id()).name(),
            }
        }
        _ => WorkflowFailure::Internal,
    }
}

fn classify_render_error(error: RenderError, _profile: OutputProfile) -> WorkflowFailure {
    match error {
        RenderError::UnsafeLogoGeometry => WorkflowFailure::UnsafeLogoGeometry {},
        _ => WorkflowFailure::Internal,
    }
}

#[must_use]
fn control_character_caution(payload: &str) -> Option<&'static str> {
    payload
        .chars()
        .any(char::is_control)
        .then_some(CONTROL_CHARACTER_CAUTION)
}

#[must_use]
pub fn textarea_display_utf16_length(payload: &str) -> Option<u32> {
    projected_utf16_length(payload)
}
