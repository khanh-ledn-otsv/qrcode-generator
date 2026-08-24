use proptest::prelude::*;
use qr_core::Version;
use qr_core::reed_solomon::{
    ErrorCorrectionCodewordCount, ReedSolomonError, SUPPORTED_ECC_CODEWORD_COUNTS, divide,
    generate_error_correction, generator_polynomial, multiply,
};
use qr_core::tables::{ErrorCorrection, lookup};

#[test]
fn field_zero_inverse_and_known_product_are_exact() {
    assert_eq!(multiply(0, 0xA7), 0);
    assert_eq!(multiply(0x53, 0x8C), 1);
    assert_eq!(multiply(0x57, 0x83), 0x31);
    assert_eq!(divide(0, 0xA7), Ok(0));
    assert_eq!(divide(0x31, 0x83), Ok(0x57));
}

#[test]
fn every_qr_generator_and_remainder_matches_dual_oracle_fixtures() {
    let mut observed_degrees = Vec::new();
    for (line_number, line) in include_str!("../../../tests/fixtures/reed_solomon.csv")
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#') && !line.starts_with("degree,"))
    {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(
            fields.len(),
            5,
            "malformed fixture line {}",
            line_number + 1
        );
        let degree = fields[0].parse::<u8>().expect("fixture degree is a u8");
        let count = ErrorCorrectionCodewordCount::new(degree).expect("fixture degree is supported");
        let generator = decode_hex(fields[1]);
        let data = decode_hex(fields[2]);
        let remainder = decode_hex(fields[3]);

        assert_eq!(generator_polynomial(count), generator);
        assert_eq!(generate_error_correction(&data, count), Ok(remainder));
        observed_degrees.push(count);
    }
    observed_degrees.sort_unstable();
    observed_degrees.dedup();
    assert_eq!(observed_degrees, SUPPORTED_ECC_CODEWORD_COUNTS);
}

#[test]
fn supported_degrees_are_derived_from_all_standard_table_rows() {
    let mut table_degrees = Vec::new();
    for version_number in Version::MIN..=Version::MAX {
        let version = Version::new(version_number).expect("loop uses QR versions");
        for ecc in [
            ErrorCorrection::Low,
            ErrorCorrection::Medium,
            ErrorCorrection::Quartile,
            ErrorCorrection::High,
        ] {
            table_degrees.push(
                lookup(version, ecc)
                    .expect("every QR table row exists")
                    .ecc_codewords_per_block(),
            );
        }
    }
    table_degrees.sort_unstable();
    table_degrees.dedup();
    assert_eq!(
        table_degrees,
        SUPPORTED_ECC_CODEWORD_COUNTS.map(ErrorCorrectionCodewordCount::number)
    );
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "fixture hex must contain whole bytes");
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
fn field_generator_cycles_through_every_nonzero_value() {
    let mut value = 1;
    let mut seen = [false; 256];
    for _ in 0..255 {
        assert_ne!(value, 0);
        assert!(!seen[usize::from(value)]);
        seen[usize::from(value)] = true;
        value = multiply(value, 2);
    }
    assert_eq!(value, 1);
    assert_eq!(seen.into_iter().filter(|seen| *seen).count(), 255);
}

#[test]
fn every_field_product_and_quotient_agrees_with_polynomial_arithmetic() {
    for left in u8::MIN..=u8::MAX {
        for right in u8::MIN..=u8::MAX {
            assert_eq!(multiply(left, right), slow_multiply(left, right));
            if right != 0 {
                assert_eq!(divide(multiply(left, right), right), Ok(left));
            }
        }
    }
}

#[test]
fn malformed_requests_return_typed_errors_without_mutating_input() {
    let division_error = divide(1, 0).expect_err("division by zero is invalid");
    assert_eq!(division_error, ReedSolomonError::DivisionByZero);
    assert_eq!(
        division_error.to_string(),
        "cannot divide a GF(256) value by zero"
    );
    for requested in [0, 1, 6, 8, 29, 31, u8::MAX] {
        assert_eq!(
            ErrorCorrectionCodewordCount::new(requested),
            Err(ReedSolomonError::UnsupportedCodewordCount { requested })
        );
    }

    let data = vec![0xA5; 249];
    let unchanged = data.clone();
    let seven = ErrorCorrectionCodewordCount::new(7).expect("seven is a QR ECC degree");
    assert_eq!(
        generate_error_correction(&data, seven),
        Err(ReedSolomonError::BlockTooLong {
            data_codewords: 249,
            ecc_codewords: 7,
            maximum_total_codewords: 255,
        })
    );
    assert_eq!(
        generate_error_correction(&data, seven)
            .expect_err("oversized block is invalid")
            .to_string(),
        "Reed–Solomon block has 249 data and 7 error-correction codewords, exceeding 255 total codewords"
    );
    assert_eq!(
        ErrorCorrectionCodewordCount::new(8)
            .expect_err("unsupported degree is invalid")
            .to_string(),
        "QR error-correction codeword count must be one of [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30], got 8"
    );
    assert_eq!(data, unchanged);
}

#[test]
fn field_block_length_boundaries_are_exact_for_every_supported_degree() {
    for degree in SUPPORTED_ECC_CODEWORD_COUNTS {
        let maximum_data_codewords = usize::from(u8::MAX - degree.number());
        let accepted = vec![0xA5; maximum_data_codewords];
        assert_eq!(
            generate_error_correction(&accepted, degree)
                .expect("exact field-sized block is accepted")
                .len(),
            usize::from(degree.number())
        );

        let rejected = vec![0xA5; maximum_data_codewords + 1];
        assert!(matches!(
            generate_error_correction(&rejected, degree),
            Err(ReedSolomonError::BlockTooLong {
                data_codewords,
                ecc_codewords,
                maximum_total_codewords: 255,
            }) if data_codewords == maximum_data_codewords + 1 && ecc_codewords == degree.number()
        ));
    }
}

fn slow_multiply(mut left: u8, mut right: u8) -> u8 {
    let mut product = 0_u8;
    while right != 0 {
        if right & 1 != 0 {
            product ^= left;
        }
        let carry = left & 0x80;
        left <<= 1;
        if carry != 0 {
            left ^= 0x1D;
        }
        right >>= 1;
    }
    product
}

fn slow_generator(degree: u8) -> Vec<u8> {
    let mut polynomial = vec![1_u8];
    let mut root = 1_u8;
    for _ in 0..degree {
        let mut product = vec![0_u8; polynomial.len() + 1];
        for (index, coefficient) in polynomial.iter().copied().enumerate() {
            product[index] ^= coefficient;
            product[index + 1] ^= slow_multiply(coefficient, root);
        }
        polynomial = product;
        root = slow_multiply(root, 2);
    }
    polynomial
}

fn slow_remainder(data: &[u8], degree: u8) -> Vec<u8> {
    let generator = slow_generator(degree);
    let mut dividend = data.to_vec();
    dividend.resize(data.len() + usize::from(degree), 0);
    for index in 0..data.len() {
        let factor = dividend[index];
        for (offset, coefficient) in generator.iter().copied().enumerate() {
            dividend[index + offset] ^= slow_multiply(coefficient, factor);
        }
    }
    dividend[data.len()..].to_vec()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn error_correction_matches_independent_long_division(
        data in prop::collection::vec(any::<u8>(), 0..=123),
        degree in prop::sample::select(SUPPORTED_ECC_CODEWORD_COUNTS.to_vec()),
    ) {
        let original = data.clone();
        let remainder = generate_error_correction(&data, degree)
            .expect("generated QR-sized block is valid");
        prop_assert_eq!(&remainder, &slow_remainder(&data, degree.number()));
        prop_assert_eq!(remainder.len(), usize::from(degree.number()));
        prop_assert_eq!(data, original);
    }
}
