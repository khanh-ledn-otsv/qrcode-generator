use qr_core::Version;
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::matrix::{MaskId, build_function_matrix, finalize_information, place_data};
use qr_core::penalty::penalty_score;
use qr_core::selection::select_mask;
use qr_core::tables::{ErrorCorrection, lookup};

#[test]
fn automatic_selection_chooses_an_oracle_agreed_minimum_candidate() {
    let version = Version::new(2).expect("version 2 is valid");
    let ecc = ErrorCorrection::Quartile;
    let stream = synthetic_stream(version, ecc);

    let selected = select_mask(&stream).expect("mask selection succeeds");

    assert_eq!(selected.mask(), MaskId::new(0).expect("mask 0 is valid"));
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
    let row = lookup(version, ecc).expect("Version 1-M table row exists");
    // Deterministic synthetic regression input found by scanning this simple
    // byte formula. It asserts selection mechanics only; it is not accepted
    // penalty-oracle evidence while the Rule 3 disagreement is quarantined.
    let data = (0..row.data_codewords())
        .map(|index| ((usize::from(index) * 149 + 238) & 0xFF) as u8)
        .collect::<Vec<_>>();
    let stream = construct(CodewordStreamRequest {
        version,
        ecc,
        data_codewords: data.as_slice(),
    })
    .expect("tie regression stream builds");

    let mut scores = Vec::new();
    for mask_number in MaskId::MIN..=MaskId::MAX {
        let mask = MaskId::new(mask_number).expect("generated mask is valid");
        let placed = place_data(
            build_function_matrix(version).expect("function matrix builds"),
            &stream,
            mask,
        )
        .expect("candidate placement succeeds");
        let candidate = finalize_information(placed).expect("candidate finalization succeeds");
        scores.push((mask, penalty_score(&candidate)));
    }
    let minimum = scores
        .iter()
        .map(|(_, score)| *score)
        .min()
        .expect("eight candidate scores exist");
    let tied_masks = scores
        .iter()
        .filter_map(|(mask, score)| (*score == minimum).then_some(*mask))
        .collect::<Vec<_>>();
    assert!(tied_masks.len() >= 2, "regression input must retain a tie");

    let selected = select_mask(&stream).expect("tie regression selection succeeds");

    assert_eq!(selected.penalty(), minimum);
    assert_eq!(
        selected.mask(),
        tied_masks.first().copied().expect("at least two masks tie")
    );
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
