use qr_core::Version;
use qr_core::bch::{format_bits, version_bits};
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::matrix::{
    InformationError, MaskId, Module, ModuleKind, build_function_matrix, finalize_information,
    place_data,
};
use qr_core::tables::{ErrorCorrection, lookup};

#[test]
fn every_format_information_value_matches_the_pinned_oracles() {
    let expected = [
        (
            ErrorCorrection::Low,
            [
                0x77C4, 0x72F3, 0x7DAA, 0x789D, 0x662F, 0x6318, 0x6C41, 0x6976,
            ],
        ),
        (
            ErrorCorrection::Medium,
            [
                0x5412, 0x5125, 0x5E7C, 0x5B4B, 0x45F9, 0x40CE, 0x4F97, 0x4AA0,
            ],
        ),
        (
            ErrorCorrection::Quartile,
            [
                0x355F, 0x3068, 0x3F31, 0x3A06, 0x24B4, 0x2183, 0x2EDA, 0x2BED,
            ],
        ),
        (
            ErrorCorrection::High,
            [
                0x1689, 0x13BE, 0x1CE7, 0x19D0, 0x0762, 0x0255, 0x0D0C, 0x083B,
            ],
        ),
    ];

    for (ecc, values) in expected {
        for (mask_number, expected_bits) in values.into_iter().enumerate() {
            let mask = MaskId::new(u8::try_from(mask_number).expect("mask index fits u8"))
                .expect("fixture mask is valid");
            assert_eq!(format_bits(ecc, mask), expected_bits);
        }
    }
}

#[test]
fn version_information_values_cover_the_first_and_last_bch_versions() {
    // Public-corroborated, non-normative values from Nayuki 1.8.0
    // `_draw_version`/`draw_version` and python-qrcode 8.2
    // `setup_type_number`/`BCH_type_number`; fixture acceptance remains
    // blocked as recorded in `.scratch/qrcode-generator/penalty-oracle-disagreement.md`.
    let expected = [
        0x07C94, 0x085BC, 0x09A99, 0x0A4D3, 0x0BBF6, 0x0C762, 0x0D847, 0x0E60D, 0x0F928, 0x10B78,
        0x1145D, 0x12A17, 0x13532, 0x149A6, 0x15683, 0x168C9, 0x177EC, 0x18EC4, 0x191E1, 0x1AFAB,
        0x1B08E, 0x1CC1A, 0x1D33F, 0x1ED75, 0x1F250, 0x209D5, 0x216F0, 0x228BA, 0x2379F, 0x24B0B,
        0x2542E, 0x26A64, 0x27541, 0x28C69,
    ];
    for (offset, expected_bits) in expected.into_iter().enumerate() {
        let version_number = u8::try_from(offset + 7).expect("fixture version fits u8");
        let version = Version::new(version_number).expect("fixture version is valid");
        assert_eq!(version_bits(version), Some(expected_bits));
    }
    assert_eq!(
        version_bits(Version::new(6).expect("version 6 is valid")),
        None
    );
}

#[test]
fn finalization_writes_both_format_copies_in_the_required_coordinates() {
    let version = Version::new(1).expect("version 1 is valid");
    let ecc = ErrorCorrection::Medium;
    let mask = MaskId::new(0).expect("mask 0 is valid");
    let matrix = completed_matrix(version, ecc, mask);
    let expected = 0x5412_u16;

    for bit in 0..15_u16 {
        let dark = ((expected >> bit) & 1) != 0;
        for (x, y) in format_coordinates(matrix.size(), bit) {
            assert_eq!(
                matrix.module(x, y),
                Some(Module::new(dark, ModuleKind::Format)),
                "format bit {bit} at ({x}, {y})"
            );
        }
    }
}

#[test]
fn finalization_writes_both_version_copies_from_version_seven() {
    let version = Version::new(7).expect("version 7 is valid");
    let matrix = completed_matrix(
        version,
        ErrorCorrection::High,
        MaskId::new(3).expect("mask 3 is valid"),
    );
    let expected = 0x07C94_u32;
    let start = matrix.size() - 11;

    for bit in 0..18_u16 {
        let a = start + bit % 3;
        let b = bit / 3;
        let expected_module = Some(Module::new(
            ((expected >> bit) & 1) != 0,
            ModuleKind::Version,
        ));
        assert_eq!(matrix.module(a, b), expected_module);
        assert_eq!(matrix.module(b, a), expected_module);
    }
}

#[test]
fn finalization_rejects_a_matrix_without_placed_data() {
    let matrix = build_function_matrix(Version::new(1).expect("version 1 is valid"))
        .expect("function matrix builds");
    assert_eq!(
        finalize_information(matrix),
        Err(InformationError::DataNotPlaced)
    );
}

fn completed_matrix(
    version: Version,
    ecc: ErrorCorrection,
    mask: MaskId,
) -> qr_core::matrix::ModuleMatrix {
    let row = lookup(version, ecc).expect("table row exists");
    let data = (0..row.data_codewords())
        .map(|index| index.wrapping_mul(149) as u8)
        .collect::<Vec<_>>();
    let stream = construct(CodewordStreamRequest {
        version,
        ecc,
        data_codewords: &data,
    })
    .expect("stream builds");
    let placed = place_data(
        build_function_matrix(version).expect("function matrix builds"),
        &stream,
        mask,
    )
    .expect("data placement succeeds");
    finalize_information(placed).expect("information finalization succeeds")
}

fn format_coordinates(size: u16, bit: u16) -> Vec<(u16, u16)> {
    let primary = match bit {
        0..=5 => (8, bit),
        6 => (8, 7),
        7 => (8, 8),
        8 => (7, 8),
        9..=14 => (14 - bit, 8),
        _ => unreachable!("test only requests 15 format bits"),
    };
    let secondary = if bit <= 7 {
        (size - 1 - bit, 8)
    } else {
        (8, size - 15 + bit)
    };
    vec![primary, secondary]
}
