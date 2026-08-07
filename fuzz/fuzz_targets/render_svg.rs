#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    Background, Foreground, RenderModel, RenderOptions, Rgba, SUPPORTED_PROFILES, render_svg,
};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let text = String::from_utf8_lossy(payload);
    let profile = SUPPORTED_PROFILES[usize::from(control & 0b11)];
    let Ok(encoded) = encode(EncodeRequest {
        text: &text,
        ecc: ErrorCorrection::Medium,
        max_version: profile.maximum_version(),
    }) else {
        return;
    };
    let background = if control & 0b100 == 0 {
        Background::Opaque(Rgba::WHITE)
    } else {
        Background::Transparent
    };
    let Ok(options) = RenderOptions::approved(profile, Foreground::Brand, background) else {
        return;
    };
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
