use qr_core::Version;
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::matrix::MaskId;
use qr_core::selection::select_mask;
use qr_core::tables::{ErrorCorrection, lookup};

#[test]
fn automatic_selection_chooses_an_oracle_agreed_minimum_candidate() {
    let version = Version::new(2).expect("version 2 is valid");
    let ecc = ErrorCorrection::Quartile;
    let stream = synthetic_stream(version, ecc);

    let selected = select_mask(&stream).expect("mask selection succeeds");

    assert_eq!(selected.mask(), MaskId::new(0).expect("mask 0 is valid"));
    assert_eq!(selected.penalty(), 387);
    assert_eq!(selected.matrix().version(), version);
}

#[test]
fn repeated_selection_is_identical() {
    let version = Version::new(7).expect("version 7 is valid");
    let ecc = ErrorCorrection::High;
    let stream = synthetic_stream(version, ecc);

    let first = select_mask(&stream).expect("first selection succeeds");
    let second = select_mask(&stream).expect("second selection succeeds");

    assert_eq!(first, second);
}

#[test]
fn public_selection_resolves_an_actual_minimum_score_tie_to_the_lower_mask() {
    let version = Version::new(1).expect("version 1 is valid");
    let ecc = ErrorCorrection::Medium;
    let data = [
        0xB6, 0x0E, 0x75, 0x0F, 0xC7, 0xBA, 0x21, 0xD9, 0x9C, 0xAE, 0x30, 0x4F, 0x31, 0x8C, 0xB1,
        0x00,
    ];
    let stream = construct(CodewordStreamRequest {
        version,
        ecc,
        data_codewords: &data,
    })
    .expect("tie fixture stream builds");

    let selected = select_mask(&stream).expect("tie fixture selection succeeds");

    assert_eq!(selected.penalty(), 309);
    assert_eq!(selected.mask(), MaskId::new(2).expect("mask 2 is valid"));
}

fn synthetic_stream(
    version: Version,
    ecc: ErrorCorrection,
) -> qr_core::codeword_stream::InterleavedCodewords {
    let row = lookup(version, ecc).expect("table row exists");
    let data = (0..row.data_codewords())
        .map(|index| ((usize::from(index) * 149 + usize::from(version.number())) & 0xFF) as u8)
        .collect::<Vec<_>>();
    construct(CodewordStreamRequest {
        version,
        ecc,
        data_codewords: &data,
    })
    .expect("stream builds")
}
