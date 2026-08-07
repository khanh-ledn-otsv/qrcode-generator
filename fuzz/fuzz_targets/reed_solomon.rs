#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::reed_solomon::{SUPPORTED_ECC_CODEWORD_COUNTS, generate_error_correction};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let degree = SUPPORTED_ECC_CODEWORD_COUNTS
        [usize::from(control) % SUPPORTED_ECC_CODEWORD_COUNTS.len()];
    let _ = generate_error_correction(payload, degree);
});
