use qr_core::Version;
use qr_core::bch::{format_bits, version_bits};
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::matrix::{
    MaskId, MatrixBuilder, Module, ModuleKind, build_function_matrix, finalize_information,
    place_data,
};
use qr_core::penalty::penalty_score;
use qr_core::selection::select_mask;
use qr_core::tables::{ErrorCorrection, lookup};

#[test]
fn automatic_selection_chooses_the_dual_oracle_minimum_candidate() {
    let version = Version::new(2).expect("version 2 is valid");
    let ecc = ErrorCorrection::Quartile;
    let stream = synthetic_stream(version, ecc);

    let selected = select_mask(version, ecc, &stream).expect("mask selection succeeds");

    assert_eq!(selected.mask(), MaskId::new(0).expect("mask 0 is valid"));
    assert_eq!(selected.penalty(), 387);
    assert_eq!(selected.matrix().version(), version);
}

#[test]
fn repeated_selection_is_identical() {
    let version = Version::new(7).expect("version 7 is valid");
    let ecc = ErrorCorrection::High;
    let stream = synthetic_stream(version, ecc);

    let first = select_mask(version, ecc, &stream).expect("first selection succeeds");
    let second = select_mask(version, ecc, &stream).expect("second selection succeeds");

    assert_eq!(first, second);
}

#[test]
fn bch_candidates_penalties_and_selection_match_the_committed_oracles() {
    let fixture = include_str!("../../../tests/fixtures/mask_selection.csv");
    for line in fixture
        .lines()
        .filter(|line| !line.starts_with('#'))
        .skip(1)
    {
        let fields = line.split(',').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6);
        match fields[0] {
            "format" => {
                let ecc = fixture_ecc(fields[2]);
                let mask = MaskId::new(fields[3].parse().expect("mask is a u8"))
                    .expect("fixture mask is valid");
                assert_eq!(
                    format_bits(ecc, mask),
                    u16::from_str_radix(fields[4], 16).expect("format bits are hexadecimal")
                );
            }
            "version" => {
                let version = Version::new(fields[1].parse().expect("version is a u8"))
                    .expect("fixture version is valid");
                assert_eq!(
                    version_bits(version),
                    Some(u32::from_str_radix(fields[4], 16).expect("version bits are hexadecimal"))
                );
            }
            "candidate" => {
                let version = Version::new(fields[1].parse().expect("version is a u8"))
                    .expect("fixture version is valid");
                let ecc = fixture_ecc(fields[2]);
                let mask = MaskId::new(fields[3].parse().expect("mask is a u8"))
                    .expect("fixture mask is valid");
                let matrix = explicit_candidate(version, ecc, mask);
                assert_eq!(
                    penalty_score(&matrix),
                    fields[4].parse().expect("score is a u32")
                );
                assert_eq!(format!("{:016x}", matrix_fingerprint(&matrix)), fields[5]);
            }
            "selected" => {
                let version = Version::new(fields[1].parse().expect("version is a u8"))
                    .expect("fixture version is valid");
                let ecc = fixture_ecc(fields[2]);
                let selected = select_mask(version, ecc, &synthetic_stream(version, ecc))
                    .expect("fixture selection succeeds");
                assert_eq!(selected.mask().number().to_string(), fields[3]);
                assert_eq!(selected.penalty().to_string(), fields[4]);
            }
            "synthetic" => {
                let matrix = synthetic_penalty_matrix(fields[2]);
                assert_eq!(penalty_score(&matrix).to_string(), fields[4]);
                assert_eq!(format!("{:016x}", matrix_fingerprint(&matrix)), fields[5]);
            }
            unexpected => panic!("unexpected fixture kind {unexpected}"),
        }
    }
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

fn explicit_candidate(
    version: Version,
    ecc: ErrorCorrection,
    mask: MaskId,
) -> qr_core::matrix::ModuleMatrix {
    let stream = synthetic_stream(version, ecc);
    let placed = place_data(
        build_function_matrix(version).expect("function matrix builds"),
        &stream,
        mask,
    )
    .expect("data placement succeeds");
    finalize_information(placed, ecc, mask).expect("information finalization succeeds")
}

fn fixture_ecc(value: &str) -> ErrorCorrection {
    match value {
        "L" => ErrorCorrection::Low,
        "M" => ErrorCorrection::Medium,
        "Q" => ErrorCorrection::Quartile,
        "H" => ErrorCorrection::High,
        unexpected => panic!("unexpected fixture ECC {unexpected}"),
    }
}

fn matrix_fingerprint(matrix: &qr_core::matrix::ModuleMatrix) -> u64 {
    let mut value = 0xCBF2_9CE4_8422_2325_u64;
    for y in 0..matrix.size() {
        for x in 0..matrix.size() {
            let byte = if matrix
                .module(x, y)
                .expect("matrix coordinate is in bounds")
                .is_dark()
            {
                b'1'
            } else {
                b'0'
            };
            value ^= u64::from(byte);
            value = value.wrapping_mul(0x0000_0100_0000_01B3);
        }
        value ^= u64::from(b'\n');
        value = value.wrapping_mul(0x0000_0100_0000_01B3);
    }
    value
}

fn synthetic_penalty_matrix(name: &str) -> qr_core::matrix::ModuleMatrix {
    let version = Version::new(1).expect("version 1 is valid");
    let mut builder = MatrixBuilder::new(version).expect("builder exists");
    let pattern = [
        false, false, false, false, true, false, true, true, true, false, true, false, true,
    ];
    for y in 0..builder.size() {
        for x in 0..builder.size() {
            let checker = (x + y) % 2 == 0;
            let dark = match name {
                "checkerboard" => checker,
                "contextual-finder" if y == 10 && (3..16).contains(&x) => pattern
                    .get(usize::from(x - 3))
                    .copied()
                    .expect("pattern coordinate exists"),
                "contextual-finder" => checker,
                unexpected => panic!("unexpected synthetic matrix {unexpected}"),
            };
            builder
                .write(x, y, Module::new(dark, ModuleKind::Data))
                .expect("each synthetic coordinate is written once");
        }
    }
    builder.finish().expect("synthetic matrix is complete")
}
