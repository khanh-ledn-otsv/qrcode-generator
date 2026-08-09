#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, encode};
use qr_render::{
    LogoStyle, MAX_RGBA_BUFFER_BYTES, RenderModel, RenderOptions, SUPPORTED_PROFILES, render_png,
};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(payload) else {
        return;
    };
    let profile = SUPPORTED_PROFILES[usize::from(control & 0b11)];
    let logo = control & 0b100 != 0;
    let ecc = if logo {
        ErrorCorrection::High
    } else {
        ErrorCorrection::Medium
    };
    let Ok(encoded) = encode(EncodeRequest::first_fit(text, ecc, profile.maximum_version())) else {
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
    assert!(
        u64::try_from(model.png_placement().rgba_buffer_len())
            .expect("approved buffer length fits u64")
            <= MAX_RGBA_BUFFER_BYTES
    );
    let Ok(bytes) = render_png(&model) else {
        return;
    };
    let Ok(mut reader) = png::Decoder::new(Cursor::new(&bytes)).read_info() else {
        panic!("successful PNG output must parse");
    };
    let dimensions = profile.png_dimensions();
    assert_eq!(reader.info().width, dimensions.width().get());
    assert_eq!(reader.info().height, dimensions.height().get());
    let Some(buffer_size) = reader.output_buffer_size() else {
        panic!("approved PNG output must have a bounded buffer");
    };
    assert!(
        u64::try_from(buffer_size).expect("approved buffer length fits u64")
            <= MAX_RGBA_BUFFER_BYTES
    );
    let mut pixels = vec![0; buffer_size];
    assert!(reader.next_frame(&mut pixels).is_ok());
});
