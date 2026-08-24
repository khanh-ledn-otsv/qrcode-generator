//! Minimal WebAssembly adapter for the Astro preview worker.

use qr_core::EncodingMode;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_render::{ForegroundTheme, LogoStyle, OutputSafety, ProfileId};
use wasm_bindgen::prelude::*;

use crate::generation::{GeneratedPreview, GenerationRequest, ascii_capacity_limit, generate};

#[wasm_bindgen]
pub struct WasmPreview {
    preview: GeneratedPreview,
}

#[wasm_bindgen]
impl WasmPreview {
    pub fn svg(&self) -> String {
        self.preview.svg.clone()
    }

    pub fn png(&self) -> Vec<u8> {
        self.preview.png.clone()
    }

    pub fn mode(&self) -> u8 {
        match self.preview.diagnostics.mode {
            EncodingMode::Single(DataMode::Numeric) => 0,
            EncodingMode::Single(DataMode::Alphanumeric) => 1,
            EncodingMode::Single(DataMode::Byte) => 2,
            EncodingMode::Single(DataMode::Kanji) => 3,
            EncodingMode::Mixed => 4,
        }
    }

    pub fn ecc(&self) -> u8 {
        match self.preview.diagnostics.ecc {
            ErrorCorrection::Low => 0,
            ErrorCorrection::Medium => 1,
            ErrorCorrection::Quartile => 2,
            ErrorCorrection::High => 3,
        }
    }

    pub fn mask(&self) -> u8 {
        self.preview.diagnostics.mask.number()
    }

    pub fn minimum_version(&self) -> u8 {
        self.preview.diagnostics.minimum_version.number()
    }

    pub fn maximum_version(&self) -> u8 {
        self.preview.diagnostics.maximum_version.number()
    }

    pub fn selected_version(&self) -> u8 {
        self.preview.diagnostics.selected_version.number()
    }

    pub fn branding_increased_version(&self) -> bool {
        self.preview.diagnostics.branding_increased_version
    }

    pub fn matrix_modules(&self) -> u16 {
        self.preview.diagnostics.matrix_modules
    }

    pub fn svg_side_pixels(&self) -> u32 {
        self.preview.diagnostics.svg_side_pixels
    }

    pub fn png_side_pixels(&self) -> u32 {
        self.preview.diagnostics.png_side_pixels
    }

    pub fn safety(&self) -> u8 {
        match self.preview.diagnostics.safety {
            OutputSafety::Safe => 0,
            OutputSafety::Caution => 1,
        }
    }

    pub fn contrast_hundredths(&self) -> u16 {
        self.preview.diagnostics.contrast_ratio.hundredths()
    }

    pub fn requested_logo(&self) -> bool {
        self.preview.diagnostics.requested_logo_style == LogoStyle::Bundled
    }

    pub fn rendered_logo(&self) -> bool {
        self.preview.diagnostics.logo_style == LogoStyle::Bundled
    }

    pub fn logo_fallback_reason(&self) -> Option<String> {
        self.preview
            .diagnostics
            .logo_fallback_reason
            .map(str::to_owned)
    }

    pub fn obscured_data_modules(&self) -> u32 {
        self.preview.diagnostics.obscured_data_modules
    }

    pub fn obscured_remainder_modules(&self) -> u32 {
        self.preview.diagnostics.obscured_remainder_modules
    }
}

#[wasm_bindgen]
pub fn generate_preview(
    payload: &str,
    profile: &str,
    foreground_theme: &str,
    logo_enabled: bool,
) -> Result<WasmPreview, JsValue> {
    let profile_id = profile_from_value(profile).ok_or_else(internal_failure)?;
    let foreground_theme = foreground_from_value(foreground_theme).ok_or_else(internal_failure)?;
    let request = GenerationRequest::new(
        payload.to_owned(),
        profile_id,
        foreground_theme,
        logo_enabled,
    );
    generate(&request)
        .map(|preview| WasmPreview { preview })
        .map_err(|failure| JsValue::from_str(&failure.message()))
}

#[wasm_bindgen]
pub fn capacity_limit(profile: &str, logo_enabled: bool) -> usize {
    profile_from_value(profile)
        .map(|profile| ascii_capacity_limit(profile, logo_enabled))
        .unwrap_or_default()
}

fn internal_failure() -> JsValue {
    JsValue::from_str("QR generation failed unexpectedly. Change the input and try again.")
}

fn profile_from_value(value: &str) -> Option<ProfileId> {
    match value {
        "small" => Some(ProfileId::Small),
        "standard" => Some(ProfileId::Standard),
        "primary-cta" => Some(ProfileId::PrimaryCta),
        "hero-campaign" => Some(ProfileId::HeroCampaign),
        "business-card" => Some(ProfileId::BusinessCard),
        "flyer-brochure" => Some(ProfileId::FlyerBrochure),
        "poster-package" => Some(ProfileId::PosterPackage),
        _ => None,
    }
}

fn foreground_from_value(value: &str) -> Option<ForegroundTheme> {
    match value {
        "magenta" => Some(ForegroundTheme::Magenta),
        "black" => Some(ForegroundTheme::Black),
        _ => None,
    }
}
