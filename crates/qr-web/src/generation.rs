//! Product generation policy shared by the native tests and WASM adapter.

use qr_core::encoding::{EncodingError, EncodingMode};
use qr_core::matrix::MaskId;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeError, EncodeRequest, Version, encode};
use qr_render::{
    BRANDED_LOGO_VERSION, ContrastRatio, ForegroundTheme, LogoStyle, OutputProfile, OutputSafety,
    ProfileId, RenderError, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png, render_svg,
};

const LOGO_CAPACITY_FALLBACK_REASON: &str = "payload exceeds branded capacity";
const INTERNAL_FAILURE_MESSAGE: &str =
    "QR generation failed unexpectedly. Change the input and try again.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenerationRequest {
    payload: String,
    profile_id: ProfileId,
    foreground_theme: ForegroundTheme,
    logo_enabled: bool,
}

impl GenerationRequest {
    pub(crate) const fn new(
        payload: String,
        profile_id: ProfileId,
        foreground_theme: ForegroundTheme,
        logo_enabled: bool,
    ) -> Self {
        Self {
            payload,
            profile_id,
            foreground_theme,
            logo_enabled,
        }
    }

    fn ecc(&self) -> ErrorCorrection {
        if self.logo_enabled {
            ErrorCorrection::High
        } else {
            ErrorCorrection::Medium
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GenerationDiagnostics {
    pub(crate) mode: EncodingMode,
    pub(crate) ecc: ErrorCorrection,
    pub(crate) mask: MaskId,
    pub(crate) minimum_version: Version,
    pub(crate) maximum_version: Version,
    pub(crate) selected_version: Version,
    pub(crate) branding_increased_version: bool,
    pub(crate) matrix_modules: u16,
    pub(crate) svg_side_pixels: u32,
    pub(crate) png_side_pixels: u32,
    pub(crate) safety: OutputSafety,
    pub(crate) contrast_ratio: ContrastRatio,
    pub(crate) requested_logo_style: LogoStyle,
    pub(crate) logo_style: LogoStyle,
    pub(crate) logo_fallback_reason: Option<&'static str>,
    pub(crate) obscured_data_modules: u32,
    pub(crate) obscured_remainder_modules: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedPreview {
    pub(crate) svg: String,
    pub(crate) png: Vec<u8>,
    pub(crate) diagnostics: GenerationDiagnostics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GenerationFailure {
    EmptyPayload,
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
    },
    UnsafeLogoGeometry,
    Internal,
}

impl GenerationFailure {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::EmptyPayload => "Enter a URL to generate a QR code.".to_owned(),
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
            } => format!(
                "Logo mode requires QR version {} or larger, but this output variant supports up to version {}.",
                minimum_version.number(),
                maximum_version.number(),
            ),
            Self::UnsafeLogoGeometry => "Logo mode is not approved for this QR geometry. Disable the logo to keep this exact payload.".to_owned(),
            Self::Internal => INTERNAL_FAILURE_MESSAGE.to_owned(),
        }
    }
}

pub(crate) fn generate(request: &GenerationRequest) -> Result<GeneratedPreview, GenerationFailure> {
    let profile = supported_profile(request.profile_id).ok_or(GenerationFailure::Internal)?;
    let branded_minimum_version = if request.logo_enabled {
        profile.minimum_version().max(BRANDED_LOGO_VERSION)
    } else {
        profile.minimum_version()
    };
    let branded_attempt = encode(EncodeRequest::with_version_range(
        &request.payload,
        request.ecc(),
        branded_minimum_version,
        profile.maximum_version(),
    ));

    // A request that does not fit at ECC H may still fit unchanged at ECC M.
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
                &request.payload,
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
        .map_err(classify_render_error)?;
    let model = RenderModel::new(&encoded, options).map_err(classify_render_error)?;
    let svg = render_svg(&model).map_err(|_| GenerationFailure::Internal)?;
    let png = render_png(&model).map_err(|_| GenerationFailure::Internal)?;
    let png_placement = model.png_placement();
    let outer_padding = png_placement.outer_padding();
    if outer_padding.left != outer_padding.right
        || outer_padding.left != outer_padding.top
        || outer_padding.left != outer_padding.bottom
    {
        return Err(GenerationFailure::Internal);
    }
    let logo_placement = model.logo_placement();

    Ok(GeneratedPreview {
        svg,
        png,
        diagnostics: GenerationDiagnostics {
            mode: encoded.mode(),
            ecc: encoded.ecc(),
            mask: encoded.mask(),
            minimum_version,
            maximum_version: profile.maximum_version(),
            selected_version: encoded.version(),
            branding_increased_version: request.logo_enabled
                && logo_fallback_reason.is_none()
                && encoded.minimum_version_increased_selection(),
            matrix_modules: encoded.version().symbol_size(),
            svg_side_pixels: model.svg_placement().output_dimensions().width().get(),
            png_side_pixels: png_placement.canvas_dimensions().width().get(),
            safety: model.options().safety(),
            contrast_ratio: model.options().contrast_ratio(),
            requested_logo_style: if request.logo_enabled {
                LogoStyle::Bundled
            } else {
                LogoStyle::None
            },
            logo_style: model.options().logo_style(),
            logo_fallback_reason: logo_fallback_reason.or_else(|| model.logo_fallback_reason()),
            obscured_data_modules: logo_placement
                .map(qr_render::LogoPlacement::obscured_data_modules)
                .unwrap_or_default(),
            obscured_remainder_modules: logo_placement
                .map(qr_render::LogoPlacement::obscured_remainder_modules)
                .unwrap_or_default(),
        },
    })
}

pub(crate) const fn ascii_capacity_limit(profile: ProfileId, logo_enabled: bool) -> usize {
    match (profile, logo_enabled) {
        (ProfileId::Small, false) => 106,
        (ProfileId::Small, true) => 58,
        (ProfileId::Standard, false) => 152,
        (ProfileId::Standard, true) => 84,
        (
            ProfileId::PrimaryCta
            | ProfileId::HeroCampaign
            | ProfileId::BusinessCard
            | ProfileId::FlyerBrochure
            | ProfileId::PosterPackage,
            false,
        ) => 287,
        (
            ProfileId::PrimaryCta
            | ProfileId::HeroCampaign
            | ProfileId::BusinessCard
            | ProfileId::FlyerBrochure
            | ProfileId::PosterPackage,
            true,
        ) => 137,
    }
}

fn supported_profile(profile_id: ProfileId) -> Option<OutputProfile> {
    SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == profile_id)
}

fn classify_encode_error(
    error: EncodeError,
    request: &GenerationRequest,
    profile: OutputProfile,
) -> GenerationFailure {
    match error {
        EncodeError::Payload(EncodingError::EmptyPayload) => GenerationFailure::EmptyPayload,
        EncodeError::Payload(EncodingError::InputLimitExceeded {
            byte_length,
            maximum,
        }) => GenerationFailure::InputLimitExceeded {
            byte_length,
            maximum,
        },
        EncodeError::Payload(
            EncodingError::PayloadTooLargeForProfile { .. } | EncodingError::PayloadTooLargeForQr,
        ) => GenerationFailure::OverCapacity {
            maximum_version: profile.maximum_version(),
        },
        EncodeError::Payload(EncodingError::InvalidVersionRange { minimum, maximum })
            if request.logo_enabled =>
        {
            GenerationFailure::LogoMinimumUnavailable {
                minimum_version: minimum,
                maximum_version: maximum,
            }
        }
        _ => GenerationFailure::Internal,
    }
}

fn classify_render_error(error: RenderError) -> GenerationFailure {
    match error {
        RenderError::UnsafeLogoGeometry => GenerationFailure::UnsafeLogoGeometry,
        _ => GenerationFailure::Internal,
    }
}

#[cfg(test)]
mod tests {
    use qr_core::tables::ErrorCorrection;
    use qr_render::{ForegroundTheme, LogoStyle, ProfileId};

    use super::{GenerationFailure, GenerationRequest, ascii_capacity_limit, generate};

    fn request(
        payload: impl Into<String>,
        profile: ProfileId,
        logo_enabled: bool,
    ) -> GenerationRequest {
        GenerationRequest::new(
            payload.into(),
            profile,
            ForegroundTheme::Magenta,
            logo_enabled,
        )
    }

    fn synthetic_ascii_url(length: usize) -> String {
        const PREFIX: &str = "https://example.test/";
        assert!(length >= PREFIX.len());
        format!("{PREFIX}{}", "a".repeat(length - PREFIX.len()))
    }

    #[test]
    fn exact_payload_is_preserved_across_theme_and_logo_choices() {
        let payload = "  café\n";
        let magenta = generate(&GenerationRequest::new(
            payload.to_owned(),
            ProfileId::Standard,
            ForegroundTheme::Magenta,
            false,
        ))
        .unwrap();
        let black = generate(&GenerationRequest::new(
            payload.to_owned(),
            ProfileId::Standard,
            ForegroundTheme::Black,
            false,
        ))
        .unwrap();

        assert_eq!(magenta.diagnostics.mode, black.diagnostics.mode);
        assert_eq!(magenta.diagnostics.mask, black.diagnostics.mask);
        assert!(black.svg.contains("fill=\"#000000\""));
        assert!(!magenta.svg.contains(payload));
    }

    #[test]
    fn logo_changes_ecc_and_minimum_before_fitting() {
        let branded = generate(&request("a".repeat(30), ProfileId::BusinessCard, true)).unwrap();
        let unbranded = generate(&request("a".repeat(30), ProfileId::BusinessCard, false)).unwrap();

        assert_eq!(branded.diagnostics.ecc, ErrorCorrection::High);
        assert_eq!(branded.diagnostics.minimum_version.number(), 6);
        assert_eq!(branded.diagnostics.logo_style, LogoStyle::Bundled);
        assert_eq!(unbranded.diagnostics.ecc, ErrorCorrection::Medium);
        assert_eq!(unbranded.diagnostics.selected_version.number(), 5);
    }

    #[test]
    fn capacities_are_exact_fit_and_one_over() {
        for profile in [
            ProfileId::Small,
            ProfileId::Standard,
            ProfileId::PrimaryCta,
            ProfileId::HeroCampaign,
            ProfileId::BusinessCard,
            ProfileId::FlyerBrochure,
            ProfileId::PosterPackage,
        ] {
            for logo_enabled in [false, true] {
                let limit = ascii_capacity_limit(profile, logo_enabled);
                let exact =
                    generate(&request(synthetic_ascii_url(limit), profile, logo_enabled)).unwrap();
                if logo_enabled {
                    assert_eq!(exact.diagnostics.logo_style, LogoStyle::Bundled);
                    let one_over =
                        generate(&request(synthetic_ascii_url(limit + 1), profile, true)).expect(
                            "one-over branded output falls back without changing the payload",
                        );
                    assert_eq!(one_over.diagnostics.logo_style, LogoStyle::None);
                } else {
                    assert!(matches!(
                        generate(&request(synthetic_ascii_url(limit + 1), profile, false,)),
                        Err(GenerationFailure::OverCapacity { .. })
                    ));
                }
            }
        }
    }

    #[test]
    fn repeated_artifacts_are_deterministic() {
        let request = request(
            "https://example.test/deterministic",
            ProfileId::Standard,
            false,
        );
        let first = generate(&request).unwrap();
        let second = generate(&request).unwrap();

        assert_eq!(first.svg, second.svg);
        assert_eq!(first.png, second.png);
    }

    #[test]
    fn invalid_payloads_are_typed() {
        assert_eq!(
            generate(&request("", ProfileId::Small, false)),
            Err(GenerationFailure::EmptyPayload)
        );
        assert!(matches!(
            generate(&request("x".repeat(4097), ProfileId::Small, false)),
            Err(GenerationFailure::InputLimitExceeded {
                byte_length: 4097,
                maximum: 4096,
            })
        ));
    }
}
