use proptest::prelude::*;
use qr_core::Version;
use qr_core::encoding::{EciAssignment, EncodeRequest, EncodingError, encode};
use qr_core::tables::{DataMode, ErrorCorrection};

#[test]
fn minimum_version_raises_a_small_payload_without_changing_its_encoding_policy() {
    let encoded = encode(EncodeRequest {
        text: "brand me",
        ecc: ErrorCorrection::High,
        min_version: Version::new(6).expect("Version 6 is valid"),
        max_version: Version::new(8).expect("Version 8 is valid"),
    })
    .expect("small payload fits the requested range");

    assert_eq!(encoded.version().number(), 6);
    assert_eq!(encoded.mode(), DataMode::Byte);
    assert_eq!(encoded.eci_assignment(), None);
    assert_eq!(encoded.data_bits_used(), 76);
    assert!(encoded.minimum_version_applied());
}

#[test]
fn checked_version_range_covers_exact_one_over_natural_fit_and_inversion() {
    let version_six = Version::new(6).expect("Version 6 is valid");
    let version_eight = Version::new(8).expect("Version 8 is valid");
    let exact = "a".repeat(58);
    let one_over = "a".repeat(59);

    let exact = encode(EncodeRequest {
        text: &exact,
        ecc: ErrorCorrection::High,
        min_version: version_six,
        max_version: version_six,
    })
    .expect("58 byte-mode characters fit Version 6-H");
    assert_eq!(exact.version(), version_six);
    assert_eq!(exact.data_bits_used(), 476);
    assert_eq!(exact.data_bits_capacity(), 480);

    assert!(matches!(
        encode(EncodeRequest {
            text: &one_over,
            ecc: ErrorCorrection::High,
            min_version: version_six,
            max_version: version_six,
        }),
        Err(EncodingError::PayloadTooLargeForProfile { required, maximum })
            if required.number() == 7 && maximum == version_six
    ));

    let natural_fit = encode(EncodeRequest {
        text: &one_over,
        ecc: ErrorCorrection::High,
        min_version: version_six,
        max_version: version_eight,
    })
    .expect("the range permits the naturally larger fit");
    assert_eq!(natural_fit.version().number(), 7);
    assert!(!natural_fit.minimum_version_applied());

    assert_eq!(
        encode(EncodeRequest {
            text: "unchanged",
            ecc: ErrorCorrection::High,
            min_version: version_eight,
            max_version: version_six,
        }),
        Err(EncodingError::InvalidVersionRange {
            minimum: version_eight,
            maximum: version_six,
        })
    );
}

#[test]
fn numeric_payload_uses_the_first_fitting_version_and_exact_padded_codewords() {
    // The literal was checked against pinned qrcodegen 1.8.0 and qrcode 8.2.
    let encoded = encode(EncodeRequest {
        text: "01234567",
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(40).expect("Version 40 is valid"),
    })
    .expect("numeric payload fits");

    assert_eq!(encoded.mode(), DataMode::Numeric);
    assert_eq!(encoded.version().number(), 1);
    assert_eq!(encoded.eci_assignment(), None);
    assert_eq!(encoded.data_bits_used(), 41);
    assert_eq!(encoded.data_bits_capacity(), 128);
    assert_eq!(
        encoded.data_codewords(),
        [
            0x10, 0x20, 0x0C, 0x56, 0x61, 0x80, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11,
            0xEC, 0x11,
        ]
    );
}

#[test]
fn numeric_groups_of_one_two_and_three_digits_match_oracle_codewords() {
    for (text, expected) in [
        (
            "1",
            &[
                0x10, 0x04, 0x40, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC,
                0x11, 0xEC, 0x11, 0xEC, 0x11,
            ][..],
        ),
        (
            "12",
            &[
                0x10, 0x08, 0x60, 0x00, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11,
                0xEC, 0x11, 0xEC, 0x11, 0xEC,
            ][..],
        ),
        (
            "123",
            &[
                0x10, 0x0C, 0x7B, 0x00, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11,
                0xEC, 0x11, 0xEC, 0x11, 0xEC,
            ][..],
        ),
    ] {
        let encoded = encode(EncodeRequest {
            text,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(1).expect("Version 1 is valid"),
        })
        .expect("numeric group fits");
        assert_eq!(encoded.data_codewords(), expected);
    }
}

#[test]
fn whole_payload_mode_selection_and_eci_match_oracle_codewords() {
    // ASCII literals agree across pinned qrcodegen 1.8.0 and qrcode 8.2. The
    // UTF-8+ECI literal comes from qrcodegen because qrcode 8.2 has no ECI API.
    let cases = [
        (
            "HELLO WORLD",
            ErrorCorrection::Quartile,
            DataMode::Alphanumeric,
            None,
            &[
                0x20, 0x5B, 0x0B, 0x78, 0xD1, 0x72, 0xDC, 0x4D, 0x43, 0x40, 0xEC, 0x11, 0xEC,
            ][..],
        ),
        (
            "hello",
            ErrorCorrection::Low,
            DataMode::Byte,
            None,
            &[
                0x40, 0x56, 0x86, 0x56, 0xC6, 0xC6, 0xF0, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC,
                0x11, 0xEC, 0x11, 0xEC, 0x11,
            ][..],
        ),
        (
            "é",
            ErrorCorrection::Low,
            DataMode::Byte,
            Some(EciAssignment::Utf8),
            &[
                0x71, 0xA4, 0x02, 0xC3, 0xA9, 0x00, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11,
                0xEC, 0x11, 0xEC, 0x11, 0xEC,
            ][..],
        ),
    ];

    for (text, error_correction, expected_mode, expected_eci, expected_codewords) in cases {
        let encoded = encode(EncodeRequest {
            text,
            ecc: error_correction,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        })
        .expect("test payload fits");
        assert_eq!(encoded.mode(), expected_mode);
        assert_eq!(encoded.eci_assignment(), expected_eci);
        assert_eq!(encoded.data_codewords(), expected_codewords);
    }
}

#[test]
fn whitespace_and_line_breaks_are_encoded_without_trimming_or_normalization() {
    let encoded = encode(EncodeRequest {
        text: "\nhello ",
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(1).expect("Version 1 is valid"),
    })
    .expect("preserved payload fits");

    assert_eq!(encoded.mode(), DataMode::Byte);
    assert_eq!(
        encoded.data_codewords(),
        [
            0x40, 0x70, 0xA6, 0x86, 0x56, 0xC6, 0xC6, 0xF2, 0x00, 0xEC, 0x11, 0xEC, 0x11, 0xEC,
            0x11, 0xEC, 0x11, 0xEC, 0x11,
        ]
    );
}

#[test]
fn complete_alphanumeric_alphabet_is_selected_as_one_segment() {
    let encoded = encode(EncodeRequest {
        text: "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(40).expect("Version 40 is valid"),
    })
    .expect("alphanumeric alphabet fits");

    assert_eq!(encoded.mode(), DataMode::Alphanumeric);
    assert_eq!(encoded.eci_assignment(), None);
}

#[test]
fn version_selection_uses_the_requested_error_correction_capacity() {
    let payload = "a".repeat(100);
    for (error_correction, expected_version) in [
        (ErrorCorrection::Low, 5),
        (ErrorCorrection::Medium, 6),
        (ErrorCorrection::Quartile, 8),
        (ErrorCorrection::High, 10),
    ] {
        let encoded = encode(EncodeRequest {
            text: &payload,
            ecc: error_correction,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        })
        .expect("payload fits");
        assert_eq!(encoded.ecc(), error_correction);
        assert_eq!(encoded.version().number(), expected_version);
    }
}

#[test]
fn truncated_terminator_fit_stays_in_version_one_and_one_more_digit_selects_version_two() {
    let exact = "1".repeat(41);
    let one_more = "1".repeat(42);
    let maximum = Version::new(40).expect("Version 40 is valid");

    let exact = encode(EncodeRequest {
        text: &exact,
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: maximum,
    })
    .expect("41 digits fit Version 1-L");
    let one_more = encode(EncodeRequest {
        text: &one_more,
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: maximum,
    })
    .expect("42 digits fit Version 2-L");

    assert_eq!(exact.version().number(), 1);
    assert_eq!(exact.data_bits_used(), 151);
    assert_eq!(exact.data_bits_capacity(), 152);
    assert_eq!(one_more.version().number(), 2);
}

#[test]
fn exact_bit_capacity_fit_has_no_terminator_or_pad_and_one_more_selects_version_two() {
    let exact_payload = "1".repeat(34);
    let one_more_payload = "1".repeat(35);
    let maximum = Version::new(40).expect("Version 40 is valid");

    let exact = encode(EncodeRequest {
        text: &exact_payload,
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: maximum,
    })
    .expect("34 digits exactly fill Version 1-M");
    let one_more = encode(EncodeRequest {
        text: &one_more_payload,
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: maximum,
    })
    .expect("35 digits fit Version 2-M");

    assert_eq!(exact.version().number(), 1);
    assert_eq!(exact.data_bits_used(), exact.data_bits_capacity());
    assert_eq!(exact.data_codewords().len(), 16);
    assert_eq!(one_more.version().number(), 2);
}

#[test]
fn profile_ceiling_versions_fit_and_one_byte_more_returns_required_version() {
    for (maximum, exact_bytes) in [(5, 106), (8, 192), (12, 367), (13, 425)] {
        let exact_payload = "a".repeat(exact_bytes);
        let one_more_payload = "a".repeat(exact_bytes + 1);
        let maximum_version = Version::new(maximum).expect("profile maximum is valid");

        let encoded = encode(EncodeRequest {
            text: &exact_payload,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: maximum_version,
        })
        .expect("exact profile ceiling fits");
        assert_eq!(encoded.version(), maximum_version);

        assert!(matches!(
            encode(EncodeRequest {
                text: &one_more_payload,
                ecc: ErrorCorrection::Low,
                min_version: qr_core::Version::MINIMUM,
                max_version: maximum_version,
            }),
            Err(EncodingError::PayloadTooLargeForProfile { required, maximum })
                if required.number() == maximum_version.number() + 1 && maximum == maximum_version
        ));
    }
}

#[test]
fn character_count_width_transitions_are_used_at_versions_ten_and_twenty_seven() {
    let byte_payload = "a".repeat(231);
    let byte_encoded = encode(EncodeRequest {
        text: &byte_payload,
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(40).expect("Version 40 is valid"),
    })
    .expect("byte payload fits");
    assert_eq!(byte_encoded.version().number(), 10);
    assert_eq!(byte_encoded.data_bits_used(), 4 + 16 + 231 * 8);

    let numeric_payload = "1".repeat(3_284);
    let numeric_encoded = encode(EncodeRequest {
        text: &numeric_payload,
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(40).expect("Version 40 is valid"),
    })
    .expect("numeric payload fits");
    assert_eq!(numeric_encoded.version().number(), 27);
    assert_eq!(numeric_encoded.data_bits_used(), 10_965);
}

#[test]
fn version_forty_and_input_limit_boundaries_return_typed_results() {
    let version_forty_payload = "a".repeat(2_953);
    let encoded = encode(EncodeRequest {
        text: &version_forty_payload,
        ecc: ErrorCorrection::Low,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(40).expect("Version 40 is valid"),
    })
    .expect("maximum byte payload fits Version 40-L");
    assert_eq!(encoded.version().number(), 40);
    assert_eq!(encoded.data_bits_used(), 23_644);
    assert_eq!(encoded.data_bits_capacity(), 23_648);

    let qr_overflow = "a".repeat(4_096);
    assert_eq!(
        encode(EncodeRequest {
            text: &qr_overflow,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        }),
        Err(EncodingError::PayloadTooLargeForQr)
    );

    let at_input_limit = "1".repeat(4_096);
    assert!(
        encode(EncodeRequest {
            text: &at_input_limit,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        })
        .is_ok()
    );

    let over_input_limit = "1".repeat(4_097);
    assert!(matches!(
        encode(EncodeRequest {
            text: &over_input_limit,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        }),
        Err(EncodingError::InputLimitExceeded {
            byte_length: 4_097,
            maximum: 4_096
        })
    ));

    let multibyte_at_limit = "é".repeat(2_048);
    assert_eq!(multibyte_at_limit.len(), 4_096);
    assert_eq!(
        encode(EncodeRequest {
            text: &multibyte_at_limit,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        }),
        Err(EncodingError::PayloadTooLargeForQr)
    );
    let multibyte_over_limit = "é".repeat(2_049);
    assert!(matches!(
        encode(EncodeRequest {
            text: &multibyte_over_limit,
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        }),
        Err(EncodingError::InputLimitExceeded {
            byte_length: 4_098,
            maximum: 4_096
        })
    ));
}

#[test]
fn empty_payload_is_a_typed_error() {
    assert_eq!(
        encode(EncodeRequest {
            text: "",
            ecc: ErrorCorrection::Low,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(1).expect("Version 1 is valid"),
        }),
        Err(EncodingError::EmptyPayload)
    );
}

fn error_correction_strategy() -> impl Strategy<Value = ErrorCorrection> {
    prop_oneof![
        Just(ErrorCorrection::Low),
        Just(ErrorCorrection::Medium),
        Just(ErrorCorrection::Quartile),
        Just(ErrorCorrection::High),
    ]
}

fn maximum_version_strategy() -> impl Strategy<Value = u8> {
    prop::sample::select(vec![1, 5, 8, 9, 10, 12, 13, 26, 27, 40])
}

fn payload_strategy() -> impl Strategy<Value = String> {
    let alphanumeric = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".to_vec();
    prop_oneof![
        3 => prop::collection::vec(b'0'..=b'9', 1..700)
            .prop_map(|bytes| String::from_utf8(bytes).expect("digits are UTF-8")),
        3 => prop::collection::vec(prop::sample::select(alphanumeric), 1..500)
            .prop_map(|bytes| String::from_utf8(bytes).expect("alphabet is UTF-8")),
        3 => prop::collection::vec(b'a'..=b'z', 1..500)
            .prop_map(|bytes| String::from_utf8(bytes).expect("lowercase ASCII is UTF-8")),
        1 => prop::collection::vec(prop::sample::select(vec!['é', '漢', '🙂']), 1..120)
            .prop_map(|characters| characters.into_iter().collect()),
    ]
}

fn independent_used_bits(text: &str, encoded: &qr_core::encoding::EncodedData) -> u32 {
    let count = u32::try_from(text.len()).expect("generated payload is small");
    let band = match encoded.version().number() {
        1..=9 => 0,
        10..=26 => 1,
        _ => 2,
    };
    let (count_width, payload_bits) = match encoded.mode() {
        DataMode::Numeric => (
            [10, 12, 14]
                .get(band)
                .copied()
                .expect("version band exists"),
            (count / 3) * 10
                + match count % 3 {
                    0 => 0,
                    1 => 4,
                    _ => 7,
                },
        ),
        DataMode::Alphanumeric => (
            [9, 11, 13].get(band).copied().expect("version band exists"),
            (count / 2) * 11 + if count % 2 == 0 { 0 } else { 6 },
        ),
        DataMode::Byte => (
            [8, 16, 16].get(band).copied().expect("version band exists"),
            count * 8,
        ),
        DataMode::Kanji => panic!("release-one encoder never selects Kanji"),
    };
    let eci_bits = if encoded.eci_assignment().is_some() {
        12
    } else {
        0
    };
    eci_bits + 4 + count_width + payload_bits
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(192))]

    #[test]
    fn encoding_is_deterministic_first_fit_and_capacity_safe(
        text in payload_strategy(),
        ecc in error_correction_strategy(),
        maximum_number in maximum_version_strategy(),
    ) {
        let maximum = Version::new(maximum_number).expect("strategy emits valid versions");
        let request = EncodeRequest { text: &text, ecc, min_version: qr_core::Version::MINIMUM, max_version: maximum };
        let first = encode(request);
        prop_assert_eq!(&first, &encode(request));

        if let Ok(encoded) = &first {
            prop_assert!(encoded.version() <= maximum);
            prop_assert_eq!(encoded.data_bits_used(), independent_used_bits(&text, encoded));
            prop_assert_eq!(
                encoded.data_codewords().len() * 8,
                usize::try_from(encoded.data_bits_capacity()).expect("QR capacity fits usize")
            );
            if encoded.version().number() > Version::MIN {
                let previous = Version::new(encoded.version().number() - 1)
                    .expect("preceding version is valid");
                let rejects_previous = matches!(
                    encode(EncodeRequest { text: &text, ecc, min_version: qr_core::Version::MINIMUM, max_version: previous }),
                    Err(EncodingError::PayloadTooLargeForProfile { required, maximum })
                        if required == encoded.version() && maximum == previous
                );
                prop_assert!(rejects_previous);
            }
        }

        let unrestricted = encode(EncodeRequest {
            text: &text,
            ecc,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        });
        if first.is_ok() {
            prop_assert_eq!(first, unrestricted);
        }
    }

    #[test]
    fn appending_a_same_mode_character_never_reduces_used_bits(
        text in payload_strategy(),
        ecc in error_correction_strategy(),
    ) {
        let maximum = Version::new(40).expect("Version 40 is valid");
        let original = encode(EncodeRequest { text: &text, ecc, min_version: qr_core::Version::MINIMUM, max_version: maximum });
        if let Ok(original) = original {
            let suffix = match (original.mode(), original.eci_assignment()) {
                (DataMode::Numeric, _) => "1",
                (DataMode::Alphanumeric, _) => "A",
                (DataMode::Byte, Some(EciAssignment::Utf8)) => "é",
                (DataMode::Byte, None) => "a",
                (DataMode::Kanji, _) => unreachable!("release-one mode policy"),
            };
            let appended = format!("{text}{suffix}");
            if appended.len() <= 4_096
                && let Ok(appended) = encode(EncodeRequest {
                    text: &appended,
                    ecc,
                    min_version: qr_core::Version::MINIMUM,
                    max_version: maximum,
                })
            {
                prop_assert_eq!(appended.mode(), original.mode());
                prop_assert!(appended.data_bits_used() >= original.data_bits_used());
            }
        }
    }
}
