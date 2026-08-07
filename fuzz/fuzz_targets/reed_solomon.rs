#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::reed_solomon::{SUPPORTED_ECC_CODEWORD_COUNTS, generate_error_correction};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let degree = SUPPORTED_ECC_CODEWORD_COUNTS
        [usize::from(control) % SUPPORTED_ECC_CODEWORD_COUNTS.len()];
    let _ = generate_error_correction(payload, degree);
    let maximum_data_length = usize::from(u8::MAX - degree.number());
    let bounded = &payload[..payload.len().min(maximum_data_length)];
    assert_eq!(
        generate_error_correction(bounded, degree).as_deref(),
        Ok(slow_remainder(bounded, degree.number()).as_slice())
    );
});

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

fn slow_remainder(data: &[u8], degree: u8) -> Vec<u8> {
    let mut generator = vec![1_u8];
    let mut root = 1_u8;
    for _ in 0..degree {
        let mut product = vec![0_u8; generator.len() + 1];
        for (index, coefficient) in generator.iter().copied().enumerate() {
            product[index] ^= coefficient;
            product[index + 1] ^= slow_multiply(coefficient, root);
        }
        generator = product;
        root = slow_multiply(root, 2);
    }

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
