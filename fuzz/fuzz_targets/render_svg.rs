#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{LogoStyle, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_svg};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let text = String::from_utf8_lossy(payload);
    let profile = SUPPORTED_PROFILES[usize::from(control) % SUPPORTED_PROFILES.len()];
    let logo = control & 0b100 != 0;
    let ecc = if logo {
        ErrorCorrection::High
    } else {
        ErrorCorrection::Medium
    };
    let Ok(encoded) = encode(EncodeRequest::first_fit(
        &text,
        ecc,
        profile.maximum_version(),
    )) else {
        return;
    };
    let Ok(mut options) = RenderOptions::safe(profile) else {
        return;
    };
    if logo {
        let Ok(logo_options) = options.with_logo(LogoStyle::Bundled) else {
            return;
        };
        options = logo_options;
    }
    let Ok(model) = RenderModel::new(&encoded, options) else {
        return;
    };
    let Ok(svg) = render_svg(&model) else {
        return;
    };
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
    assert!(svg.len() < 4 * 1024 * 1024);
    assert_eq!(render_svg(&model).as_deref(), Ok(svg.as_str()));
});
