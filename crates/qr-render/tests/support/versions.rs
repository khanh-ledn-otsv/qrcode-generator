pub fn first_byte_length(version: u8) -> usize {
    const FIRST_BYTE_LENGTHS: [usize; 13] =
        [1, 15, 27, 43, 63, 85, 107, 123, 153, 181, 214, 252, 288];
    FIRST_BYTE_LENGTHS[usize::from(version - 1)]
}
