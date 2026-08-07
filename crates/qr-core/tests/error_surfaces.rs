use std::error::Error;

use qr_core::bit_buffer::BitBufferError;
use qr_core::codeword_stream::CodewordStreamError;
use qr_core::encoding::EncodingError;
use qr_core::matrix::{InformationError, MaskId, MatrixError, Module, ModuleKind, PlacementError};
use qr_core::reed_solomon::ReedSolomonError;
use qr_core::selection::SelectionError;
use qr_core::tables::{DataMode, ErrorCorrection, TableLookupError};
use qr_core::{EncodeError, Version};

fn assert_error(error: &(dyn Error + 'static), fragment: &str, has_source: bool) {
    assert!(error.to_string().contains(fragment));
    assert_eq!(error.source().is_some(), has_source);
}

#[test]
fn every_public_core_error_variant_has_stable_context_and_source_chaining() {
    let version_one = Version::new(1).unwrap();
    let version_two = Version::new(2).unwrap();
    let version_error = Version::new(0).unwrap_err();
    assert_error(&version_error, "between 1 and 40", false);

    let bit_errors = [
        BitBufferError::InvalidWidth { width: 33 },
        BitBufferError::ValueDoesNotFit { value: 8, width: 2 },
        BitBufferError::LengthOverflow,
        BitBufferError::NotByteAligned { bit_length: 7 },
    ];
    for (error, fragment) in bit_errors.iter().zip([
        "exceeds 32",
        "does not fit",
        "length overflow",
        "not byte-aligned",
    ]) {
        assert_error(error, fragment, false);
    }

    let reed_solomon_errors = [
        ReedSolomonError::DivisionByZero,
        ReedSolomonError::UnsupportedCodewordCount { requested: 1 },
        ReedSolomonError::BlockTooLong {
            data_codewords: 250,
            ecc_codewords: 30,
            maximum_total_codewords: 255,
        },
    ];
    for (error, fragment) in
        reed_solomon_errors
            .iter()
            .zip(["divide", "must be one of", "exceeding 255"])
    {
        assert_error(error, fragment, false);
    }

    let table_errors = [
        TableLookupError::InvalidVersion(version_error),
        TableLookupError::MissingRow {
            version: version_one,
            error_correction: ErrorCorrection::Medium,
        },
        TableLookupError::MissingVersionData {
            version: version_one,
        },
        TableLookupError::InconsistentRow {
            version: version_one,
            error_correction: ErrorCorrection::High,
        },
    ];
    for (error, (fragment, has_source)) in table_errors.iter().zip([
        ("between 1 and 40", true),
        ("missing QR table row", false),
        ("missing QR version data", false),
        ("inconsistent QR table row", false),
    ]) {
        assert_error(error, fragment, has_source);
    }

    let matrix_errors = [
        MatrixError::TableLookup(table_errors[1]),
        MatrixError::DimensionOverflow { size: u16::MAX },
        MatrixError::OutOfBounds {
            x: 22,
            y: 0,
            size: 21,
        },
        MatrixError::DoubleWrite { x: 1, y: 2 },
        MatrixError::InvalidReservation {
            x: 1,
            y: 2,
            kind: ModuleKind::Data,
        },
        MatrixError::Incomplete { unwritten: 3 },
    ];
    for (error, fragment) in matrix_errors.iter().zip([
        "missing QR table row",
        "cannot be represented",
        "outside",
        "written twice",
        "cannot be reserved",
        "unwritten modules",
    ]) {
        assert_error(error, fragment, false);
    }

    let light_data = Module::new(false, ModuleKind::Data);
    let dark_finder = Module::new(true, ModuleKind::Finder);
    let placement_errors = [
        PlacementError::Matrix(matrix_errors[2]),
        PlacementError::AlreadyPlaced,
        PlacementError::VersionMismatch {
            matrix: version_one,
            stream: version_two,
        },
        PlacementError::OwnershipMismatch {
            x: 1,
            y: 2,
            expected: light_data,
            actual: dark_finder,
        },
        PlacementError::LengthOverflow,
        PlacementError::StreamLengthMismatch {
            writable_modules: 10,
            data_bits: 8,
            remainder_bits: 7,
        },
        PlacementError::TraversalIncomplete {
            expected: 10,
            placed: 9,
        },
    ];
    for (error, (fragment, has_source)) in placement_errors.iter().zip([
        ("outside", true),
        ("already been placed", false),
        ("does not match", false),
        ("ownership mismatch", false),
        ("length overflow", false),
        ("writable modules", false),
        ("traversal expected", false),
    ]) {
        assert_error(error, fragment, has_source);
    }

    let information_errors = [
        InformationError::DataNotPlaced,
        InformationError::AlreadyFinalized,
        InformationError::OutOfBounds { x: 30, y: 30 },
        InformationError::OwnershipMismatch {
            x: 1,
            y: 2,
            expected: ModuleKind::Format,
            actual: ModuleKind::Data,
        },
    ];
    for (error, fragment) in information_errors.iter().zip([
        "before data placement",
        "already finalized",
        "out of bounds",
        "requires Format",
    ]) {
        assert_error(error, fragment, false);
    }

    let mask_error = MaskId::new(8).unwrap_err();
    assert_error(&mask_error, "between 0 and 7", false);
    let selection_errors = [
        SelectionError::NoCandidates,
        SelectionError::Mask(mask_error),
        SelectionError::Matrix(matrix_errors[3]),
        SelectionError::Placement(placement_errors[1].clone()),
        SelectionError::Information(information_errors[0]),
    ];
    for (error, (fragment, has_source)) in selection_errors.iter().zip([
        ("no candidates", false),
        ("between 0 and 7", true),
        ("written twice", true),
        ("already been placed", true),
        ("before data placement", true),
    ]) {
        assert_error(error, fragment, has_source);
    }

    let codeword_errors = [
        CodewordStreamError::Table(table_errors[2]),
        CodewordStreamError::ReedSolomon(reed_solomon_errors[0]),
        CodewordStreamError::DataLengthMismatch {
            expected: 16,
            actual: 15,
        },
        CodewordStreamError::LengthOverflow,
        CodewordStreamError::InconsistentBlockLayout {
            expected: 16,
            consumed: 15,
        },
        CodewordStreamError::InconsistentErrorCorrectionLength {
            expected: 10,
            actual: 9,
        },
        CodewordStreamError::InconsistentStreamLength {
            expected: 26,
            actual: 25,
        },
    ];
    for (error, (fragment, has_source)) in codeword_errors.iter().zip([
        ("missing QR version data", true),
        ("divide", true),
        ("requires 16", false),
        ("arithmetic overflowed", false),
        ("declares 16", false),
        ("requires 10", false),
        ("requires 26", false),
    ]) {
        assert_error(error, fragment, has_source);
    }

    let encoding_errors = [
        EncodingError::EmptyPayload,
        EncodingError::InputLimitExceeded {
            byte_length: 4_097,
            maximum: 4_096,
        },
        EncodingError::PayloadTooLargeForProfile {
            required: version_two,
            maximum: version_one,
        },
        EncodingError::PayloadTooLargeForQr,
        EncodingError::MalformedPayload {
            mode: DataMode::Numeric,
        },
        EncodingError::LengthOverflow,
        EncodingError::BitBuffer(bit_errors[0]),
        EncodingError::Table(table_errors[3]),
        EncodingError::InvalidVersion(version_error),
    ];
    for (error, (fragment, has_source)) in encoding_errors.iter().zip([
        ("must not be empty", false),
        ("4097 bytes", false),
        ("above profile maximum", false),
        ("Version 40", false),
        ("malformed", false),
        ("length overflow", false),
        ("exceeds 32", true),
        ("inconsistent QR table", true),
        ("between 1 and 40", true),
    ]) {
        assert_error(error, fragment, has_source);
    }

    let encode_errors = [
        EncodeError::Payload(encoding_errors[0].clone()),
        EncodeError::Codewords(codeword_errors[2].clone()),
        EncodeError::Selection(selection_errors[0].clone()),
    ];
    for (error, fragment) in
        encode_errors
            .iter()
            .zip(["must not be empty", "requires 16", "no candidates"])
    {
        assert_error(error, fragment, true);
    }

    let converted_errors: [Box<dyn Error>; 16] = [
        Box::new(TableLookupError::from(version_error)),
        Box::new(MatrixError::from(table_errors[0])),
        Box::new(PlacementError::from(matrix_errors[0])),
        Box::new(SelectionError::from(mask_error)),
        Box::new(SelectionError::from(matrix_errors[1])),
        Box::new(SelectionError::from(placement_errors[0].clone())),
        Box::new(SelectionError::from(information_errors[1])),
        Box::new(CodewordStreamError::from(table_errors[1])),
        Box::new(CodewordStreamError::from(reed_solomon_errors[1])),
        Box::new(EncodingError::from(bit_errors[1])),
        Box::new(EncodingError::from(table_errors[2])),
        Box::new(EncodingError::from(version_error)),
        Box::new(EncodeError::from(encoding_errors[1].clone())),
        Box::new(EncodeError::from(codeword_errors[0].clone())),
        Box::new(EncodeError::from(selection_errors[1].clone())),
        Box::new(SelectionError::from(information_errors[2])),
    ];
    for error in converted_errors {
        assert!(!error.to_string().is_empty());
    }
}
