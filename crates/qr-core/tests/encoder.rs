use proptest::prelude::*;
use qr_core::encoding::EncodingError;
use qr_core::matrix::ModuleKind;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_core::{EncodeError, EncodeRequest, Version, encode};

#[test]
fn public_encoder_returns_diagnostics_and_a_complete_immutable_matrix() {
    let encoded = encode(EncodeRequest {
        text: "HELLO WORLD",
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: Version::new(40).expect("Version 40 is valid"),
    })
    .expect("representative payload encodes");

    assert_eq!(
        encoded.version(),
        Version::new(1).expect("Version 1 is valid")
    );
    assert_eq!(encoded.ecc(), ErrorCorrection::Medium);
    assert_eq!(encoded.mode(), DataMode::Alphanumeric);
    assert_eq!(encoded.data_bits_used(), 74);
    assert_eq!(encoded.data_bits_capacity(), 128);
    assert_eq!(encoded.modules().version(), encoded.version());
    assert_eq!(encoded.modules().size(), 21);
    assert_eq!(
        encoded.modules().modules().len(),
        usize::from(encoded.modules().size()).pow(2)
    );
}

#[test]
fn public_encoder_reports_typed_profile_boundaries() {
    let version_one = Version::new(1).expect("Version 1 is valid");
    let exact_fit = encode(EncodeRequest {
        text: "1".repeat(34).as_str(),
        ecc: ErrorCorrection::Medium,
        min_version: qr_core::Version::MINIMUM,
        max_version: version_one,
    });
    assert!(exact_fit.is_ok());

    let one_over = "1".repeat(35);
    assert!(matches!(
        encode(EncodeRequest {
            text: &one_over,
            ecc: ErrorCorrection::Medium,
            min_version: qr_core::Version::MINIMUM,
            max_version: version_one,
        }),
        Err(EncodeError::Payload(_))
    ));
    assert!(matches!(
        encode(EncodeRequest {
            text: "",
            ecc: ErrorCorrection::Medium,
            min_version: qr_core::Version::MINIMUM,
            max_version: version_one,
        }),
        Err(EncodeError::Payload(_))
    ));
}

#[test]
fn public_encoder_honors_and_checks_the_complete_version_range() {
    let version_six = Version::new(6).expect("Version 6 is valid");
    let version_eight = Version::new(8).expect("Version 8 is valid");
    let encoded = encode(EncodeRequest {
        text: "public range",
        ecc: ErrorCorrection::High,
        min_version: version_six,
        max_version: version_eight,
    })
    .expect("the payload fits the checked range");

    assert_eq!(encoded.version(), version_six);
    assert!(encoded.minimum_version_applied());
    assert!(matches!(
        encode(EncodeRequest {
            text: "public range",
            ecc: ErrorCorrection::High,
            min_version: version_eight,
            max_version: version_six,
        }),
        Err(EncodeError::Payload(EncodingError::InvalidVersionRange {
            minimum,
            maximum,
        })) if minimum == version_eight && maximum == version_six
    ));
}

#[test]
fn public_encoder_matches_dual_oracle_composed_matrix_goldens() {
    for line in include_str!("../../../tests/fixtures/encoder_goldens.csv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .skip(1)
    {
        let fields = line.split(',').collect::<Vec<_>>();
        assert_eq!(fields.len(), 8, "golden row must have eight fields");
        let payload = decode_hex(fields[6]);
        let text = std::str::from_utf8(&payload).expect("golden payload is valid UTF-8");
        let ecc = match fields[2] {
            "L" => ErrorCorrection::Low,
            "M" => ErrorCorrection::Medium,
            "Q" => ErrorCorrection::Quartile,
            "H" => ErrorCorrection::High,
            value => panic!("unknown golden ECC {value}"),
        };
        let encoded = encode(EncodeRequest {
            text,
            ecc,
            min_version: qr_core::Version::MINIMUM,
            max_version: Version::new(40).expect("Version 40 is valid"),
        })
        .expect("golden payload encodes");
        let expected_mode = match fields[1] {
            "numeric" => DataMode::Numeric,
            "alphanumeric" => DataMode::Alphanumeric,
            "byte" | "utf8" => DataMode::Byte,
            value => panic!("unknown golden mode {value}"),
        };
        assert_eq!(encoded.mode(), expected_mode, "golden {}", fields[0]);
        assert_eq!(encoded.ecc(), ecc, "golden {}", fields[0]);
        assert_eq!(
            encoded.version().number().to_string(),
            fields[3],
            "golden {}",
            fields[0]
        );
        assert_eq!(
            encoded.mask().number().to_string(),
            fields[4],
            "golden {}",
            fields[0]
        );
        assert_eq!(
            encoded
                .eci_assignment()
                .map(|eci| eci.number())
                .unwrap_or(0)
                .to_string(),
            fields[5],
            "golden {}",
            fields[0]
        );
        assert_eq!(
            format!("{:016x}", matrix_fingerprint(&encoded)),
            fields[7],
            "golden {}",
            fields[0]
        );
    }
}

fn decode_hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "golden payload hex has whole bytes");
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("golden hex is ASCII");
            u8::from_str_radix(pair, 16).expect("golden hex contains bytes")
        })
        .collect()
}

fn matrix_fingerprint(encoded: &qr_core::EncodedQr) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    let size = encoded.modules().size();
    for y in 0..size {
        for x in 0..size {
            let byte = if encoded
                .modules()
                .module(x, y)
                .expect("golden matrix is complete")
                .is_dark()
            {
                b'1'
            } else {
                b'0'
            };
            value = (value ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
        value = (value ^ u64::from(b'\n')).wrapping_mul(0x100_0000_01b3);
    }
    value
}

proptest! {
    #[test]
    fn public_encoder_is_deterministic_monotonic_and_fully_classified(
        text in "[A-Za-z0-9 .:/_-]{1,120}",
        lower_max in 1_u8..=20,
        extra in 0_u8..=20,
    ) {
        let lower = Version::new(lower_max).expect("generated lower version is valid");
        let upper_number = lower_max.saturating_add(extra).min(40);
        let upper = Version::new(upper_number).expect("generated upper version is valid");
        let request = EncodeRequest {
            text: &text,
            ecc: ErrorCorrection::Medium,
            min_version: qr_core::Version::MINIMUM,
            max_version: lower,
        };
        let first = encode(request);
        let second = encode(request);
        prop_assert_eq!(&first, &second);
        if let Ok(encoded) = first {
            let expanded = encode(EncodeRequest { min_version: qr_core::Version::MINIMUM, max_version: upper, ..request })
                .expect("a larger maximum preserves a successful fit");
            prop_assert_eq!(expanded.version(), encoded.version());
            prop_assert_eq!(&expanded, &encoded);
            prop_assert_eq!(
                encoded.modules().modules().len(),
                usize::from(encoded.modules().size()).pow(2),
            );
            prop_assert!(encoded.modules().modules().all(|module| matches!(
                module.kind(),
                ModuleKind::Data
                    | ModuleKind::Remainder
                    | ModuleKind::Finder
                    | ModuleKind::Separator
                    | ModuleKind::Timing
                    | ModuleKind::Alignment
                    | ModuleKind::Format
                    | ModuleKind::Version
                    | ModuleKind::Dark
            )));
        }
    }
}
