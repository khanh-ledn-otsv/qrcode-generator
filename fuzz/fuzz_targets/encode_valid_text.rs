#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};

fuzz_target!(|text: &str| {
    let control = text.as_bytes().first().copied().unwrap_or(0);
    let ecc = [
        ErrorCorrection::Low,
        ErrorCorrection::Medium,
        ErrorCorrection::Quartile,
        ErrorCorrection::High,
    ][usize::from(control & 0b11)];
    let max_version = Version::new(control % 40 + 1).expect("derived version is valid");
    let first = encode(EncodeRequest::first_fit(text, ecc, max_version));
    let second = encode(EncodeRequest::first_fit(text, ecc, max_version));
    assert_eq!(first, second);
});
