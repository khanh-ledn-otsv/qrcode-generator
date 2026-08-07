#![no_main]

use libfuzzer_sys::fuzz_target;
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::encoding;
use qr_core::matrix::{
    MaskId, MatrixBuilder, Module, ModuleKind, build_function_matrix, finalize_information,
    place_data,
};
use qr_core::penalty::penalty_score;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version};

fuzz_target!(|data: &[u8]| {
    let Some((&control, payload)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(payload) else {
        return;
    };
    let ecc = match control & 0b11 {
        0 => ErrorCorrection::Low,
        1 => ErrorCorrection::Medium,
        2 => ErrorCorrection::Quartile,
        _ => ErrorCorrection::High,
    };
    let Ok(max_version) = Version::new(control % 40 + 1) else {
        return;
    };
    let mut malformed = match MatrixBuilder::new(max_version) {
        Ok(builder) => builder,
        Err(_) => return,
    };
    let x = u16::from(payload.first().copied().unwrap_or(0));
    let y = u16::from(payload.get(1).copied().unwrap_or(0));
    let kinds = [
        ModuleKind::Data,
        ModuleKind::Remainder,
        ModuleKind::Finder,
        ModuleKind::Separator,
        ModuleKind::Timing,
        ModuleKind::Alignment,
        ModuleKind::Format,
        ModuleKind::Version,
        ModuleKind::Dark,
    ];
    let kind = kinds[usize::from(control) % kinds.len()];
    let _ = malformed.write(x, y, Module::new(control & 0x80 != 0, kind));
    let _ = malformed.write(x, y, Module::new(false, kind));
    let _ = malformed.reserve(x, y, kind);
    let _ = malformed.finish();
    let Ok(encoded) = encoding::encode(EncodeRequest {
        text,
        ecc,
        max_version,
    }) else {
        return;
    };
    let Ok(stream) = construct(CodewordStreamRequest {
        version: encoded.version(),
        ecc,
        data_codewords: encoded.data_codewords(),
    }) else {
        return;
    };
    let Ok(mask) = MaskId::new((control >> 2) & 0b111) else {
        return;
    };
    let Ok(functions) = build_function_matrix(encoded.version()) else {
        return;
    };
    let Ok(placed) = place_data(functions, &stream, mask) else {
        return;
    };
    if let Ok(matrix) = finalize_information(placed) {
        let _ = penalty_score(&matrix);
    }
});
