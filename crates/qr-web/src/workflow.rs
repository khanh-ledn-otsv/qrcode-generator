//! Plain-Rust state transitions for the interactive QR workflow.

use qr_core::encoding::EncodingError;
use qr_core::matrix::MaskId;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_core::{EncodeError, EncodeRequest, Version, encode};
use qr_render::{
    Background, ContrastRatio, FinderStyle, Foreground, LogoPlacement, LogoStyle, ModuleStyle,
    OutputProfile, OutputSafety, ProfileId, RenderError, RenderModel, RenderOptions, Rgba,
    SUPPORTED_PROFILES, render_png, render_svg,
};

use crate::textarea::{TextAreaBuffer, projected_utf16_length};

const CONTROL_CHARACTER_CAUTION: &str =
    "This payload contains control characters. Confirm that they are intentional.";
const TRANSPARENT_OUTPUT_CAUTION: &str = "Transparent output has unknown effective contrast. Check it on white, light-gray, dark, and patterned placement surfaces.";
const LOGO_OUTPUT_CAUTION: &str = "The bundled logo obscures QR data modules. Validate the exported code in its actual environment.";
const INTERNAL_FAILURE_MESSAGE: &str =
    "QR generation failed unexpectedly. Change the input and try again.";
const SAFE_OUTPUT_GUIDANCE: &str =
    "Use SVG when resizing and validate the QR code in its final environment.";
const PRINT_OUTPUT_GUIDANCE: &str =
    "Place at 25–30 mm or larger; validate for the actual environment.";
const BRANDED_MINIMUM_VERSION: Version = match Version::new(6) {
    Ok(version) => version,
    Err(_) => panic!("the approved branded minimum must be a valid QR version"),
};

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
    foreground: Foreground,
    background: Background,
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

    #[must_use]
    pub const fn minimum_version(&self) -> Version {
        if self.logo_enabled {
            BRANDED_MINIMUM_VERSION
        } else {
            Version::MINIMUM
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostics {
    mode: DataMode,
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
    foreground: Foreground,
    background: Background,
    safety: OutputSafety,
    contrast_ratio: Option<ContrastRatio>,
    module_style: ModuleStyle,
    finder_style: FinderStyle,
    logo_style: LogoStyle,
    logo_placement: Option<LogoPlacement>,
}

impl Diagnostics {
    #[must_use]
    pub const fn mode(self) -> DataMode {
        self.mode
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
    pub const fn foreground(self) -> Foreground {
        self.foreground
    }

    #[must_use]
    pub const fn background(self) -> Background {
        self.background
    }

    #[must_use]
    pub const fn safety(self) -> OutputSafety {
        self.safety
    }

    #[must_use]
    pub const fn contrast_ratio(self) -> Option<ContrastRatio> {
        self.contrast_ratio
    }

    #[must_use]
    pub const fn module_style(self) -> ModuleStyle {
        self.module_style
    }

    #[must_use]
    pub const fn finder_style(self) -> FinderStyle {
        self.finder_style
    }

    #[must_use]
    pub const fn logo_style(self) -> LogoStyle {
        self.logo_style
    }

    #[must_use]
    pub const fn logo_placement(self) -> Option<LogoPlacement> {
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
    InputLimitExceeded {
        byte_length: usize,
        maximum: usize,
    },
    OverCapacity {
        maximum_version: Version,
    },
    UnsafeContrast {
        actual: ContrastRatio,
        minimum: ContrastRatio,
    },
    LogoRequiresOpaqueWhite,
    LogoMinimumUnavailable {
        minimum_version: Version,
        maximum_version: Version,
        profile_name: &'static str,
    },
    UnsafeLogoGeometry,
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
            Self::UnsafeContrast { actual, minimum } => format!(
                "Opaque QR contrast is {}.{:02}:1; at least {}.{:02}:1 is required.",
                actual.hundredths() / 100,
                actual.hundredths() % 100,
                minimum.hundredths() / 100,
                minimum.hundredths() % 100,
            ),
            Self::LogoRequiresOpaqueWhite => {
                "Logo mode requires an opaque white QR background.".to_owned()
            }
            Self::LogoMinimumUnavailable {
                minimum_version,
                maximum_version,
                profile_name,
            } => format!(
                "Logo mode requires QR version {} or larger, but {profile_name} supports up to version {}.",
                minimum_version.number(),
                maximum_version.number(),
            ),
            Self::UnsafeLogoGeometry => {
                "Logo mode is unavailable because no safe placement exists for this QR version."
                    .to_owned()
            }
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
    logo_enabled: bool,
    foreground: Foreground,
    background: Background,
    revision: Revision,
    preview_state: PreviewState,
}

impl WorkflowState {
    #[must_use]
    pub fn new(profile_id: ProfileId) -> Self {
        Self {
            payload: TextAreaBuffer::new(String::new()),
            profile_id,
            logo_enabled: true,
            foreground: Foreground::Brand,
            background: Background::Opaque(Rgba::WHITE),
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
        if enabled {
            self.background = Background::Opaque(Rgba::WHITE);
        }
        self.begin_preview()
    }

    pub fn select_foreground(
        &mut self,
        foreground: Foreground,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        self.foreground = foreground;
        self.begin_preview()
    }

    pub fn select_background(
        &mut self,
        background: Background,
    ) -> Result<PreviewRequest, WorkflowFailure> {
        if self.logo_enabled && background != Background::Opaque(Rgba::WHITE) {
            return Err(WorkflowFailure::LogoRequiresOpaqueWhite);
        }
        self.background = background;
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
    pub const fn foreground(&self) -> Foreground {
        self.foreground
    }

    #[must_use]
    pub const fn background(&self) -> Background {
        self.background
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
        let transparent = matches!(self.background, Background::Transparent)
            .then_some(TRANSPARENT_OUTPUT_CAUTION);
        let logo = self.logo_enabled.then_some(LOGO_OUTPUT_CAUTION);
        let cautions = [control, transparent, logo]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
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
            logo_enabled: self.logo_enabled,
            foreground: self.foreground,
            background: self.background,
        })
    }

    fn set_internal_failure(&mut self) -> WorkflowFailure {
        self.preview_state = PreviewState::Invalid(WorkflowFailure::Internal);
        WorkflowFailure::Internal
    }
}

pub fn evaluate_preview(request: &PreviewRequest) -> Result<Preview, WorkflowFailure> {
    let profile = supported_profile(request.profile_id).ok_or(WorkflowFailure::Internal)?;
    let encoded = encode(EncodeRequest {
        text: request.payload(),
        ecc: request.ecc(),
        min_version: request.minimum_version(),
        max_version: profile.maximum_version(),
    })
    .map_err(|error| classify_encode_error(error, request, profile))?;
    let options = RenderOptions::approved(profile, request.foreground, request.background)
        .and_then(|options| {
            options.with_logo(if request.logo_enabled {
                LogoStyle::Bundled
            } else {
                LogoStyle::None
            })
        })
        .map_err(classify_render_error)?;
    let model = RenderModel::new(&encoded, options).map_err(classify_render_error)?;
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
            mode: encoded.mode(),
            ecc: encoded.ecc(),
            mask: encoded.mask(),
            minimum_version: request.minimum_version(),
            maximum_version: profile.maximum_version(),
            selected_version: encoded.version(),
            branding_increased_version: request.logo_enabled && encoded.minimum_version_applied(),
            used_data_bits: encoded.data_bits_used(),
            available_data_bits: encoded.data_bits_capacity(),
            data_codewords,
            matrix_modules: encoded.version().symbol_size(),
            svg_side_pixels: profile.svg_dimensions().width().get(),
            png_side_pixels: profile.png_dimensions().width().get(),
            quiet_zone_modules: png_placement.symbol().quiet_zone_modules_per_side().get(),
            module_scale: png_placement.module_scale().get(),
            rendered_symbol_side_pixels: png_placement.rendered_symbol_dimensions().width().get(),
            outer_padding_per_side: outer_padding.left.get(),
            print_guidance: profile_presentation(profile.id()).guidance(),
            foreground: request.foreground,
            background: request.background,
            safety: options.safety(),
            contrast_ratio: options.contrast_ratio(),
            module_style: options.module_style(),
            finder_style: options.finder_style(),
            logo_style: options.logo_style(),
            logo_placement: model.logo_placement(),
        },
    })
}

#[must_use]
pub const fn mode_label(mode: DataMode) -> &'static str {
    match mode {
        DataMode::Numeric => "Numeric",
        DataMode::Alphanumeric => "Alphanumeric",
        DataMode::Byte => "Byte",
        DataMode::Kanji => "Kanji",
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

fn classify_render_error(error: RenderError) -> WorkflowFailure {
    match error {
        RenderError::UnsafeContrast { actual, minimum } => {
            WorkflowFailure::UnsafeContrast { actual, minimum }
        }
        RenderError::LogoRequiresOpaqueWhite => WorkflowFailure::LogoRequiresOpaqueWhite,
        RenderError::UnsafeLogoGeometry => WorkflowFailure::UnsafeLogoGeometry,
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
