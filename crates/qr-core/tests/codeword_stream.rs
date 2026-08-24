use qr_core::Version;
use qr_core::codeword_stream::{CodewordStreamError, CodewordStreamRequest, construct};
use qr_core::reed_solomon::{ErrorCorrectionCodewordCount, generate_error_correction};
use qr_core::tables::{ErrorCorrection, lookup, lookup_version_number};
use std::error::Error;

#[test]
fn one_group_and_two_group_streams_match_dual_oracle_fixtures() {
    let mut observed_cases = Vec::new();
    for line in include_str!("../../../tests/fixtures/interleaved_codewords.csv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("version,"))
    {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 6);
        let version_number = fields[0].parse().expect("fixture version is a u8");
        let ecc = match fields[1] {
            "L" => ErrorCorrection::Low,
            "M" => ErrorCorrection::Medium,
            "Q" => ErrorCorrection::Quartile,
            "H" => ErrorCorrection::High,
            value => panic!("unknown fixture ECC {value}"),
        };
        let data = decode_hex(fields[2]);
        let expected = decode_hex(fields[3]);
        let remainder_bit_count = fields[4].parse().expect("fixture remainder count is a u8");
        let stream = construct(CodewordStreamRequest {
            version: Version::new(version_number).expect("fixture version is valid"),
            ecc,
            data_codewords: &data,
        })
        .expect("fixture data capacity is valid");

        assert_eq!(stream.codewords(), expected);
        assert_eq!(stream.remainder_bit_count(), remainder_bit_count);
        observed_cases.push(fields[5]);
    }
    assert_eq!(observed_cases, ["one-group", "two-group-short-long"]);
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("fixture hex is ASCII");
            u8::from_str_radix(pair, 16).expect("fixture hex contains bytes")
        })
        .collect()
}

#[test]
fn every_table_row_round_trips_through_independent_deinterleaving() {
    let mut observed_one_group = false;
    let mut observed_two_groups = false;
    for version_number in Version::MIN..=Version::MAX {
        let version = Version::new(version_number).expect("loop uses valid versions");
        for ecc in error_correction_levels() {
            let row = lookup(version, ecc).expect("every table row exists");
            let data: Vec<u8> = (0..usize::from(row.data_codewords()))
                .map(|index| ((index * 149 + usize::from(version_number)) & 0xFF) as u8)
                .collect();
            let original = data.clone();
            let stream = construct(CodewordStreamRequest {
                version,
                ecc,
                data_codewords: &data,
            })
            .expect("validated table row constructs a stream");

            let block_lengths: Vec<_> = row
                .block_groups()
                .flat_map(|group| {
                    std::iter::repeat_n(
                        usize::from(group.data_codewords_per_block()),
                        usize::from(group.block_count()),
                    )
                })
                .collect();
            observed_one_group |= block_lengths
                .iter()
                .all(|length| *length == block_lengths[0]);
            observed_two_groups |= block_lengths.windows(2).any(|pair| pair[0] != pair[1]);

            let (data_blocks, ecc_blocks) = deinterleave(
                stream.codewords(),
                &block_lengths,
                usize::from(row.ecc_codewords_per_block()),
            );
            assert_eq!(data_blocks.concat(), original);
            let ecc_count = ErrorCorrectionCodewordCount::new(row.ecc_codewords_per_block())
                .expect("table ECC degree is supported");
            for (data_block, ecc_block) in data_blocks.iter().zip(&ecc_blocks) {
                assert_eq!(
                    generate_error_correction(data_block, ecc_count),
                    Ok(ecc_block.clone())
                );
            }
            assert_eq!(stream.codewords().len(), usize::from(row.total_codewords()));
            assert_eq!(stream.remainder_bit_count(), row.remainder_bits());
            assert_eq!(data, original, "input is not mutated");
        }
    }
    assert!(observed_one_group);
    assert!(observed_two_groups);
}

#[test]
fn malformed_data_lengths_return_typed_errors_without_partial_output() {
    let version = Version::new(1).expect("Version 1 is valid");
    for (data, expected, actual) in [(vec![0; 15], 16, 15), (vec![0; 17], 16, 17)] {
        let error = construct(CodewordStreamRequest {
            version,
            ecc: ErrorCorrection::Medium,
            data_codewords: &data,
        })
        .expect_err("malformed data length is rejected");
        assert_eq!(
            error,
            CodewordStreamError::DataLengthMismatch { expected, actual }
        );
        assert_eq!(
            error.to_string(),
            format!("QR block construction requires {expected} data codewords, got {actual}")
        );
    }
}

#[test]
fn wrapped_table_and_reed_solomon_errors_preserve_their_sources() {
    let table_error =
        lookup_version_number(0, ErrorCorrection::Low).expect_err("Version 0 is invalid");
    let error = CodewordStreamError::Table(table_error);
    assert_eq!(
        error.source().map(ToString::to_string),
        Some(table_error.to_string())
    );

    let reed_solomon_error =
        ErrorCorrectionCodewordCount::new(8).expect_err("eight is not a supported QR ECC degree");
    let error = CodewordStreamError::ReedSolomon(reed_solomon_error);
    assert_eq!(
        error.source().map(ToString::to_string),
        Some(reed_solomon_error.to_string())
    );
}

fn error_correction_levels() -> [ErrorCorrection; 4] {
    [
        ErrorCorrection::Low,
        ErrorCorrection::Medium,
        ErrorCorrection::Quartile,
        ErrorCorrection::High,
    ]
}

fn deinterleave(
    stream: &[u8],
    data_lengths: &[usize],
    ecc_length: usize,
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut cursor = 0;
    let mut data_blocks: Vec<Vec<u8>> = data_lengths
        .iter()
        .map(|length| Vec::with_capacity(*length))
        .collect();
    let maximum_data_length = data_lengths.iter().copied().max().unwrap_or(0);
    for index in 0..maximum_data_length {
        for (block, &length) in data_blocks.iter_mut().zip(data_lengths) {
            if index < length {
                block.push(stream[cursor]);
                cursor += 1;
            }
        }
    }

    let mut ecc_blocks = vec![Vec::with_capacity(ecc_length); data_lengths.len()];
    for _ in 0..ecc_length {
        for block in &mut ecc_blocks {
            block.push(stream[cursor]);
            cursor += 1;
        }
    }
    assert_eq!(cursor, stream.len());
    (data_blocks, ecc_blocks)
}
