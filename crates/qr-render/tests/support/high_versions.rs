use std::error::Error;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};

pub fn payload_for_high_version(version: u8) -> Result<String, Box<dyn Error>> {
    let maximum_version = Version::try_from(version)?;
    let (mut lower, mut upper) = (1_usize, 4_096_usize);
    while lower <= upper {
        let length = lower + (upper - lower) / 2;
        let text = "a".repeat(length);
        match encode(EncodeRequest::first_fit(
            &text,
            ErrorCorrection::High,
            maximum_version,
        )) {
            Ok(encoded) if encoded.version().number() == version => return Ok(text),
            Ok(_) => lower = length + 1,
            Err(_) => {
                let Some(previous) = length.checked_sub(1) else {
                    break;
                };
                upper = previous;
            }
        }
    }
    Err(format!("no byte payload selected H-level version {version}").into())
}
