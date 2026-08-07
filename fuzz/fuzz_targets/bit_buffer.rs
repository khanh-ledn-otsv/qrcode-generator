#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::bit_buffer::BitBuffer;

fuzz_target!(|data: &[u8]| {
    let mut buffer = BitBuffer::new();
    for operation in data.chunks(5) {
        let width = operation.first().copied().unwrap_or(0) % 40;
        let mut value_bytes = [0_u8; 4];
        let available = operation.len().saturating_sub(1);
        value_bytes[..available].copy_from_slice(&operation[1..]);
        let _ = buffer.append_bits(u32::from_le_bytes(value_bytes), width);
    }
    let _ = buffer.into_bytes();
});
