use qr_core::encoding::EncodingError;
use qr_core::tables::{DataMode, ErrorCorrection};
use qr_core::{EciAssignment, EncodeError, EncodeRequest, Version, encode};

#[test]
fn committed_fuzz_regressions_replay_intended_paths_without_panics() {
    let cases = [
        (
            include_bytes!("../../../fuzz/corpus/encode/empty").as_slice(),
            None,
        ),
        (
            include_bytes!("../../../fuzz/corpus/encode/numeric_boundary").as_slice(),
            Some((DataMode::Numeric, None)),
        ),
        (
            include_bytes!("../../../fuzz/corpus/encode/utf8_eci").as_slice(),
            Some((DataMode::Byte, Some(EciAssignment::Utf8))),
        ),
        (
            include_bytes!("../../../fuzz/corpus/encode/byte_mode").as_slice(),
            Some((DataMode::Byte, None)),
        ),
    ];
    for (input, expected) in cases {
        let Some((&control, payload)) = input.split_first() else {
            continue;
        };
        let payload = payload.strip_suffix(b"\n").unwrap_or(payload);
        let Ok(text) = std::str::from_utf8(payload) else {
            continue;
        };
        let ecc = match control & 0b11 {
            0 => ErrorCorrection::Low,
            1 => ErrorCorrection::Medium,
            2 => ErrorCorrection::Quartile,
            _ => ErrorCorrection::High,
        };
        let maximum = Version::new(control % 40 + 1).expect("control always maps to Version 1-40");
        let result = encode(EncodeRequest {
            text,
            ecc,
            max_version: maximum,
        });
        match expected {
            None => assert!(matches!(
                result,
                Err(EncodeError::Payload(EncodingError::EmptyPayload))
            )),
            Some((mode, eci)) => {
                let encoded = result.expect("named corpus input must encode");
                assert_eq!(encoded.mode(), mode);
                assert_eq!(encoded.eci_assignment(), eci);
            }
        }
    }
}
