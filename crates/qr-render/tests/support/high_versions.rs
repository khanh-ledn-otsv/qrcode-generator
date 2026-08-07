use std::error::Error;

use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};

pub fn payload_for_high_version(version: u8) -> Result<String, Box<dyn Error>> {
    for length in 1..=1_000 {
        let text = "a".repeat(length);
        if encode(EncodeRequest {
            text: &text,
            ecc: ErrorCorrection::High,
            max_version: Version::try_from(version)?,
        })
        .is_ok_and(|encoded| encoded.version().number() == version)
        {
            return Ok(text);
        }
    }
    Err(format!("no byte payload selected H-level version {version}").into())
}
