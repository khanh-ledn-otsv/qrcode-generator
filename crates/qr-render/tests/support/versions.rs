use qr_core::Version;
use qr_core::tables::ErrorCorrection;
use qr_core::tables::lookup;

pub fn first_byte_length(version: u8) -> usize {
    first_byte_length_at_ecc(version, ErrorCorrection::Medium)
}

pub fn first_byte_length_at_ecc(version: u8, ecc: ErrorCorrection) -> usize {
    if version == Version::MINIMUM.number() {
        return 1;
    }
    let previous = Version::new(version - 1).expect("matrix cases use supported QR versions");
    let capacity_bits = usize::from(
        lookup(previous, ecc)
            .expect("matrix cases use supported QR ECC rows")
            .data_codewords(),
    ) * 8;
    // ISO/IEC 18004:2024, 7.4.3 and 7.4.7 define the mode/count overhead;
    // 7.5.1 defines the capacity represented by the dual-oracle table row.
    let count_bits = if previous.number() <= 9 { 8 } else { 16 };
    (capacity_bits - 4 - count_bits) / 8 + 1
}
