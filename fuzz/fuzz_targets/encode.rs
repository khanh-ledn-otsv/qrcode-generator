#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let payload = payload.strip_suffix(b"\n").unwrap_or(payload);
    let Ok(text) = std::str::from_utf8(payload) else {
        return;
    };
    let ecc = match control & 0b11 {
        0 => ErrorCorrection::Low,
        1 => ErrorCorrection::Medium,
        2 => ErrorCorrection::Quartile,
        _ => ErrorCorrection::High,
    };
    let Ok(version) = Version::new(control % 40 + 1) else {
        return;
    };
    let _ = encode(EncodeRequest {
        text,
        ecc,
        max_version: version,
    });
});
