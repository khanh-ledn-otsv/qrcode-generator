use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use std::error::Error;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn Error>> {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text)?;
    let encoded = encode(EncodeRequest::first_fit(
        &text,
        ErrorCorrection::Medium,
        Version::new(40)?,
    ))?;
    println!(
        "version={} ecc={:?} mode={:?} mask={} bits={}/{} size={}",
        encoded.version().number(),
        encoded.ecc(),
        encoded.mode(),
        encoded.mask().number(),
        encoded.data_bits_used(),
        encoded.data_bits_capacity(),
        encoded.modules().size(),
    );
    Ok(())
}
