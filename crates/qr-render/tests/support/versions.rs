use qr_core::tables::ErrorCorrection;

pub fn first_byte_length(version: u8) -> usize {
    first_byte_length_at_ecc(version, ErrorCorrection::Medium)
}

pub fn first_byte_length_at_ecc(version: u8, ecc: ErrorCorrection) -> usize {
    const MEDIUM: [usize; 13] = [1, 15, 27, 43, 63, 85, 107, 123, 153, 181, 214, 252, 288];
    const HIGH: [usize; 13] = [1, 8, 15, 25, 35, 45, 59, 65, 85, 99, 120, 138, 156];
    let lengths = match ecc {
        ErrorCorrection::Medium => MEDIUM,
        ErrorCorrection::High => HIGH,
        unexpected => panic!("no approved byte-version fixture for {unexpected:?}"),
    };
    lengths[usize::from(version - 1)]
}
