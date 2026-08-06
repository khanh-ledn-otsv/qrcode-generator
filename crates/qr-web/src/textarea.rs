//! Exact raw-text storage behind the browser textarea's normalized display.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextAreaBuffer {
    raw: String,
}

impl TextAreaBuffer {
    pub(crate) fn new(raw: String) -> Self {
        Self { raw }
    }

    pub(crate) fn raw(&self) -> &str {
        &self.raw
    }

    pub(crate) fn replace_raw(&mut self, raw: String) {
        self.raw = raw;
    }

    pub(crate) fn display(&self) -> String {
        normalize_line_endings(&self.raw)
    }

    pub(crate) fn replace_from_display(
        &mut self,
        display: String,
        edit_start_utf16: u32,
    ) -> Result<(), TextAreaError> {
        let previous_display = self.display();
        let previous_prefix = byte_at_utf16(&previous_display, edit_start_utf16)
            .ok_or(TextAreaError::InvalidCoordinate)?;
        let updated_prefix =
            byte_at_utf16(&display, edit_start_utf16).ok_or(TextAreaError::InvalidCoordinate)?;
        if previous_display[..previous_prefix] != display[..updated_prefix] {
            return Err(TextAreaError::InvalidCoordinate);
        }
        let (previous_suffix, updated_suffix) =
            common_suffix_bytes(&previous_display, &display, previous_prefix, updated_prefix);
        let previous_end = previous_display
            .len()
            .checked_sub(previous_suffix)
            .ok_or(TextAreaError::LengthOverflow)?;
        let updated_end = display
            .len()
            .checked_sub(updated_suffix)
            .ok_or(TextAreaError::LengthOverflow)?;
        let previous_end_utf16 = utf16_length(&previous_display[..previous_end])?;
        self.replace_display_range(
            edit_start_utf16,
            previous_end_utf16,
            &display[updated_prefix..updated_end],
        )
    }

    pub(crate) fn replace_display_range(
        &mut self,
        start_utf16: u32,
        end_utf16: u32,
        inserted_raw: &str,
    ) -> Result<(), TextAreaError> {
        if start_utf16 > end_utf16 {
            return Err(TextAreaError::InvalidCoordinate);
        }
        let start_byte = raw_byte_at_display_utf16(&self.raw, start_utf16)
            .ok_or(TextAreaError::InvalidCoordinate)?;
        let end_byte = raw_byte_at_display_utf16(&self.raw, end_utf16)
            .ok_or(TextAreaError::InvalidCoordinate)?;
        let capacity = start_byte
            .checked_add(inserted_raw.len())
            .and_then(|length| length.checked_add(self.raw.len().checked_sub(end_byte)?))
            .ok_or(TextAreaError::LengthOverflow)?;
        let mut updated = String::new();
        updated
            .try_reserve(capacity)
            .map_err(|_| TextAreaError::AllocationFailure)?;
        updated.push_str(&self.raw[..start_byte]);
        updated.push_str(inserted_raw);
        updated.push_str(&self.raw[end_byte..]);
        self.raw = updated;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextAreaError {
    InvalidCoordinate,
    LengthOverflow,
    AllocationFailure,
}

pub(crate) fn projected_utf16_length(payload: &str) -> Option<u32> {
    let mut length = 0_u32;
    for (_, character) in ProjectedCharacters::new(payload) {
        length = length.checked_add(u32::try_from(character.len_utf16()).ok()?)?;
    }
    Some(length)
}

fn normalize_line_endings(payload: &str) -> String {
    let mut normalized = String::with_capacity(payload.len());
    for (_, character) in ProjectedCharacters::new(payload) {
        normalized.push(character);
    }
    normalized
}

fn raw_byte_at_display_utf16(payload: &str, requested: u32) -> Option<usize> {
    let mut raw_byte = 0_usize;
    let mut display_utf16 = 0_u32;
    while raw_byte < payload.len() {
        if display_utf16 == requested {
            return Some(raw_byte);
        }
        let (raw_width, character) = ProjectedCharacters::new(payload.get(raw_byte..)?).next()?;
        let display_width = u32::try_from(character.len_utf16()).ok()?;
        raw_byte = raw_byte.checked_add(raw_width)?;
        display_utf16 = display_utf16.checked_add(display_width)?;
        if display_utf16 > requested {
            return None;
        }
    }
    (display_utf16 == requested).then_some(raw_byte)
}

struct ProjectedCharacters<'raw> {
    remaining: &'raw str,
}

impl<'raw> ProjectedCharacters<'raw> {
    fn new(raw: &'raw str) -> Self {
        Self { remaining: raw }
    }
}

impl Iterator for ProjectedCharacters<'_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let character = self.remaining.chars().next()?;
        let raw_width = if character == '\r' {
            if self.remaining.starts_with("\r\n") {
                2
            } else {
                1
            }
        } else {
            character.len_utf8()
        };
        self.remaining = self.remaining.get(raw_width..)?;
        Some((raw_width, if character == '\r' { '\n' } else { character }))
    }
}

fn byte_at_utf16(value: &str, requested: u32) -> Option<usize> {
    let mut utf16 = 0_u32;
    for (byte, character) in value.char_indices() {
        if utf16 == requested {
            return Some(byte);
        }
        utf16 = utf16.checked_add(u32::try_from(character.len_utf16()).ok()?)?;
        if utf16 > requested {
            return None;
        }
    }
    (utf16 == requested).then_some(value.len())
}

fn common_suffix_bytes(
    left: &str,
    right: &str,
    left_prefix: usize,
    right_prefix: usize,
) -> (usize, usize) {
    let left_limit = left.len().saturating_sub(left_prefix);
    let right_limit = right.len().saturating_sub(right_prefix);
    let mut left_bytes = 0_usize;
    let mut right_bytes = 0_usize;
    for (left_character, right_character) in left.chars().rev().zip(right.chars().rev()) {
        let Some(next_left) = left_bytes.checked_add(left_character.len_utf8()) else {
            break;
        };
        let Some(next_right) = right_bytes.checked_add(right_character.len_utf8()) else {
            break;
        };
        if left_character != right_character || next_left > left_limit || next_right > right_limit {
            break;
        }
        left_bytes = next_left;
        right_bytes = next_right;
    }
    (left_bytes, right_bytes)
}

fn utf16_length(value: &str) -> Result<u32, TextAreaError> {
    u32::try_from(value.encode_utf16().count()).map_err(|_| TextAreaError::LengthOverflow)
}
