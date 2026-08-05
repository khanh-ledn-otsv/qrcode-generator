use proptest::prelude::*;
use qr_core::bit_buffer::{BitBuffer, BitBufferError};

fn reference_bytes(operations: &[(u32, u8)]) -> Vec<u8> {
    let mut bits = Vec::new();
    for &(value, width) in operations {
        bits.extend((0..width).rev().map(|offset| ((value >> offset) & 1) as u8));
    }
    bits.chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0_u8, |byte, (index, bit)| byte | (*bit << (7 - index)))
        })
        .collect()
}

#[test]
fn writes_most_significant_bits_across_byte_boundaries() {
    let mut buffer = BitBuffer::new();
    buffer.append_bits(0b101, 3).expect("three bits fit");
    buffer.append_bits(0b1_1111, 5).expect("five bits fit");
    buffer.append_bits(0xABCD, 16).expect("sixteen bits fit");

    assert_eq!(buffer.bit_length(), 24);
    assert_eq!(buffer.into_bytes(), Ok(vec![0xBF, 0xAB, 0xCD]));
}

#[test]
fn malformed_bit_operations_return_typed_errors_without_partial_writes() {
    let mut buffer = BitBuffer::new();
    assert_eq!(
        buffer.append_bits(1, 33),
        Err(BitBufferError::InvalidWidth { width: 33 })
    );
    assert_eq!(
        buffer.append_bits(2, 1),
        Err(BitBufferError::ValueDoesNotFit { value: 2, width: 1 })
    );
    assert_eq!(buffer.bit_length(), 0);
    assert_eq!(buffer.append_bits(0, 0), Ok(()));
    assert_eq!(buffer.bit_length(), 0);

    buffer.append_bits(1, 1).expect("one bit fits");
    assert_eq!(
        buffer.into_bytes(),
        Err(BitBufferError::NotByteAligned { bit_length: 1 })
    );
}

#[test]
fn every_supported_width_has_exact_most_significant_bit_order() {
    for width in 1..=32 {
        let value = 0xA5C3_7E91_u32 >> (32 - width);
        let padding = (8 - usize::from(width) % 8) % 8;
        let mut buffer = BitBuffer::new();
        buffer.append_bits(value, width).expect("value fits width");
        buffer
            .append_bits(0, u8::try_from(padding).expect("padding is below eight"))
            .expect("padding fits");

        let mut operations = vec![(value, width)];
        if padding != 0 {
            operations.push((0, u8::try_from(padding).expect("padding is below eight")));
        }
        assert_eq!(buffer.into_bytes(), Ok(reference_bytes(&operations)));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn arbitrary_checked_writes_match_a_simple_bit_reference(
        operations in prop::collection::vec((any::<u32>(), 1_u8..=32), 0..80)
    ) {
        let normalized = operations
            .into_iter()
            .map(|(value, width)| {
                let value = if width == 32 {
                    value
                } else {
                    value & ((1_u32 << width) - 1)
                };
                (value, width)
            })
            .collect::<Vec<_>>();
        let mut buffer = BitBuffer::new();
        for &(value, width) in &normalized {
            buffer.append_bits(value, width).expect("normalized value fits");
        }
        let padding = (8 - buffer.bit_length() % 8) % 8;
        buffer
            .append_bits(0, u8::try_from(padding).expect("padding is below eight"))
            .expect("padding fits");
        let mut expected_operations = normalized;
        if padding != 0 {
            expected_operations.push((0, u8::try_from(padding).expect("small padding")));
        }

        prop_assert_eq!(buffer.into_bytes(), Ok(reference_bytes(&expected_operations)));
    }
}
