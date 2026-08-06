use qr_core::Version;
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::matrix::{
    MaskId, MatrixBuilder, Module, ModuleKind, PlacementError, build_function_matrix, place_data,
};
use qr_core::tables::{ErrorCorrection, lookup};
use std::collections::HashSet;

#[test]
fn version_one_places_bits_in_the_bottom_right_zig_zag_with_mask_zero() {
    let version = Version::new(1).expect("version 1 is valid");
    let row = lookup(version, ErrorCorrection::Medium).expect("version 1-M table exists");
    let mut data = vec![0; usize::from(row.data_codewords())];
    data[0] = 0b1011_0010;
    let stream = construct(CodewordStreamRequest {
        version,
        ecc: ErrorCorrection::Medium,
        data_codewords: &data,
    })
    .expect("version 1-M stream builds");
    let function_matrix = build_function_matrix(version).expect("function matrix builds");

    let matrix = place_data(
        function_matrix,
        &stream,
        MaskId::new(0).expect("mask 0 is valid"),
    )
    .expect("data placement succeeds");

    let expected = [
        ((20, 20), false),
        ((19, 20), false),
        ((20, 19), true),
        ((19, 19), false),
        ((20, 18), true),
        ((19, 18), false),
        ((20, 17), true),
        ((19, 17), true),
    ];
    for ((x, y), dark) in expected {
        assert_eq!(
            matrix.module(x, y),
            Some(Module::new(dark, ModuleKind::Data))
        );
    }
}

#[test]
fn version_two_classifies_remainder_bits_and_preserves_function_modules() {
    let version = Version::new(2).expect("version 2 is valid");
    let row = lookup(version, ErrorCorrection::Quartile).expect("version 2-Q table exists");
    let data = (0..row.data_codewords())
        .map(|index| index.wrapping_mul(149) as u8)
        .collect::<Vec<_>>();
    let stream = construct(CodewordStreamRequest {
        version,
        ecc: ErrorCorrection::Quartile,
        data_codewords: &data,
    })
    .expect("version 2-Q stream builds");
    let function_matrix = build_function_matrix(version).expect("function matrix builds");
    let original = function_matrix.clone();
    let mask = MaskId::new(3).expect("mask 3 is valid");

    let matrix = place_data(function_matrix, &stream, mask).expect("data placement succeeds");

    assert_eq!(
        matrix
            .modules()
            .filter(|module| module.kind() == ModuleKind::Data)
            .count(),
        stream.codewords().len() * 8
    );
    assert_eq!(
        matrix
            .modules()
            .filter(|module| module.kind() == ModuleKind::Remainder)
            .count(),
        7
    );
    for y in 0..matrix.size() {
        for x in 0..matrix.size() {
            let before = original.module(x, y).expect("coordinate is in bounds");
            let after = matrix.module(x, y).expect("coordinate is in bounds");
            if before.kind() == ModuleKind::Data {
                if after.kind() == ModuleKind::Remainder {
                    assert_eq!(after.is_dark(), mask.applies(x, y));
                }
            } else {
                assert_eq!(after, before, "function module changed at ({x}, {y})");
            }
        }
    }
}

#[test]
fn all_explicit_mask_predicates_match_fixed_truth_tables() {
    let coordinates = [(0, 0), (1, 0), (2, 1), (3, 4), (5, 6), (2, 2)];
    let expected = [
        [true, false, false, false, false, true],
        [true, true, false, true, true, true],
        [true, false, false, true, false, false],
        [true, false, true, false, false, false],
        [true, true, true, false, true, false],
        [true, true, false, true, true, false],
        [true, true, true, true, true, false],
        [true, false, false, false, false, false],
    ];

    for mask_number in MaskId::MIN..=MaskId::MAX {
        let mask = MaskId::new(mask_number).expect("generated mask is valid");
        let actual = coordinates.map(|(x, y)| mask.applies(x, y));
        assert_eq!(actual, expected[usize::from(mask_number)]);
    }
    assert_eq!(MaskId::new(8).expect_err("mask 8 is invalid").number(), 8);
}

#[test]
fn placement_rejects_stream_length_and_matrix_ownership_mismatches() {
    let version_one = Version::new(1).expect("version 1 is valid");
    let version_two = Version::new(2).expect("version 2 is valid");
    let version_two_row =
        lookup(version_two, ErrorCorrection::Low).expect("version 2-L table exists");
    let version_two_data = vec![0; usize::from(version_two_row.data_codewords())];
    let version_two_stream = construct(CodewordStreamRequest {
        version: version_two,
        ecc: ErrorCorrection::Low,
        data_codewords: &version_two_data,
    })
    .expect("version 2-L stream builds");
    let version_one_matrix = build_function_matrix(version_one).expect("function matrix builds");

    assert_eq!(
        place_data(
            version_one_matrix,
            &version_two_stream,
            MaskId::new(0).expect("mask 0 is valid"),
        ),
        Err(PlacementError::VersionMismatch {
            matrix: version_one,
            stream: version_two,
        })
    );

    let mut malformed = MatrixBuilder::new(version_one).expect("builder exists");
    for y in 0..malformed.size() {
        for x in 0..malformed.size() {
            malformed
                .write(x, y, Module::new(false, ModuleKind::Data))
                .expect("each coordinate is written once");
        }
    }
    let malformed = malformed.finish().expect("malformed matrix is complete");
    let version_one_row =
        lookup(version_one, ErrorCorrection::Medium).expect("version 1-M table exists");
    let version_one_data = vec![0; usize::from(version_one_row.data_codewords())];
    let version_one_stream = construct(CodewordStreamRequest {
        version: version_one,
        ecc: ErrorCorrection::Medium,
        data_codewords: &version_one_data,
    })
    .expect("version 1-M stream builds");
    assert_eq!(
        place_data(
            malformed,
            &version_one_stream,
            MaskId::new(0).expect("mask 0 is valid"),
        ),
        Err(PlacementError::OwnershipMismatch {
            x: 0,
            y: 0,
            expected: Module::new(true, ModuleKind::Finder),
            actual: Module::new(false, ModuleKind::Data),
        })
    );

    let placed = place_data(
        build_function_matrix(version_one).expect("function matrix builds"),
        &version_one_stream,
        MaskId::new(0).expect("mask 0 is valid"),
    )
    .expect("first placement succeeds");
    assert_eq!(
        place_data(
            placed,
            &version_one_stream,
            MaskId::new(0).expect("mask 0 is valid"),
        ),
        Err(PlacementError::AlreadyPlaced)
    );
}

#[test]
fn explicit_placement_matches_dual_oracle_fingerprints_and_readable_maps() {
    let fixture = include_str!("../../../tests/fixtures/placement_matrices.txt");
    let mut lines = fixture.lines().filter(|line| !line.starts_with('#'));
    assert_eq!(
        lines.next(),
        Some("version,ecc,mask,data_codewords,interleaved_codewords,remainder_bits,fnv1a64")
    );
    let mut observed = HashSet::new();
    loop {
        let row = lines.next().expect("fixture hash section ends");
        if row == "endhashes" {
            break;
        }
        let fields = row.split(',').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7);
        let version_number = fields[0].parse::<u8>().expect("version is a u8");
        let ecc = fixture_ecc(fields[1]);
        let mask_number = fields[2].parse::<u8>().expect("mask is a u8");
        let expected_data_count = fields[3].parse::<usize>().expect("data count is a usize");
        let expected_interleaved_count = fields[4]
            .parse::<usize>()
            .expect("interleaved count is a usize");
        let expected_remainder = fields[5].parse::<u8>().expect("remainder count is a u8");
        let matrix = fixture_placement(version_number, ecc, mask_number);
        let version = Version::new(version_number).expect("fixture version is valid");
        let row = lookup(version, ecc).expect("fixture table row exists");

        assert_eq!(usize::from(row.data_codewords()), expected_data_count);
        assert_eq!(
            usize::from(row.total_codewords()),
            expected_interleaved_count
        );
        assert_eq!(row.remainder_bits(), expected_remainder);
        assert_eq!(
            format!("{:016x}", fnv1a64(&fixture_matrix_text(&matrix))),
            fields[6]
        );
        observed.insert((version_number, mask_number));
    }
    assert_eq!(observed.len(), 4 * 8);
    for version in [1, 2, 7, 40] {
        for mask in MaskId::MIN..=MaskId::MAX {
            assert!(observed.contains(&(version, mask)));
        }
    }

    let mut readable_versions = Vec::new();
    while let Some(header) = lines.next() {
        let fields = header.split_ascii_whitespace().collect::<Vec<_>>();
        assert_eq!(fields.len(), 4);
        let version_number = fields[0]
            .strip_prefix("version=")
            .expect("version field")
            .parse::<u8>()
            .expect("version is a u8");
        let ecc = fixture_ecc(fields[1].strip_prefix("ecc=").expect("ECC field"));
        let mask_number = fields[2]
            .strip_prefix("mask=")
            .expect("mask field")
            .parse::<u8>()
            .expect("mask is a u8");
        let size = fields[3]
            .strip_prefix("size=")
            .expect("size field")
            .parse::<u16>()
            .expect("size is a u16");
        let matrix = fixture_placement(version_number, ecc, mask_number);
        assert_eq!(matrix.size(), size);
        for y in 0..size {
            let row = lines.next().expect("readable fixture has every row");
            assert_eq!(row.len(), usize::from(size));
            for (x, glyph) in row.chars().enumerate() {
                assert_eq!(
                    matrix.module(u16::try_from(x).expect("x fits u16"), y),
                    Some(fixture_module(glyph)),
                    "Version {version_number} mask {mask_number} at ({x}, {y})"
                );
            }
        }
        assert_eq!(lines.next(), Some("end"));
        readable_versions.push(version_number);
    }
    assert_eq!(readable_versions, [1, 2, 7, 40]);
}

#[test]
fn every_version_places_each_writable_module_and_preserves_function_ownership() {
    for version_number in Version::MIN..=Version::MAX {
        let version = Version::new(version_number).expect("generated version is valid");
        let row = lookup(version, ErrorCorrection::Low).expect("version-L table exists");
        let data = synthetic_data(version_number, usize::from(row.data_codewords()));
        let stream = construct(CodewordStreamRequest {
            version,
            ecc: ErrorCorrection::Low,
            data_codewords: &data,
        })
        .expect("generated stream builds");
        let function_matrix = build_function_matrix(version).expect("function matrix builds");
        let original = function_matrix.clone();
        let matrix = place_data(
            function_matrix,
            &stream,
            MaskId::new((version_number - 1) % 8).expect("generated mask is valid"),
        )
        .expect("generated placement succeeds");

        assert_eq!(matrix.version(), version);
        assert_eq!(
            matrix
                .modules()
                .filter(|module| module.kind() == ModuleKind::Data)
                .count(),
            stream.codewords().len() * 8
        );
        assert_eq!(
            matrix
                .modules()
                .filter(|module| module.kind() == ModuleKind::Remainder)
                .count(),
            usize::from(stream.remainder_bit_count())
        );
        for y in 0..matrix.size() {
            for x in 0..matrix.size() {
                let before = original.module(x, y).expect("coordinate is in bounds");
                let after = matrix.module(x, y).expect("coordinate is in bounds");
                if before.kind() != ModuleKind::Data {
                    assert_eq!(after, before, "function module changed at ({x}, {y})");
                }
            }
        }
    }
}

fn fixture_placement(
    version_number: u8,
    ecc: ErrorCorrection,
    mask_number: u8,
) -> qr_core::matrix::ModuleMatrix {
    let version = Version::new(version_number).expect("fixture version is valid");
    let row = lookup(version, ecc).expect("fixture table row exists");
    let data = synthetic_data(version_number, usize::from(row.data_codewords()));
    let stream = construct(CodewordStreamRequest {
        version,
        ecc,
        data_codewords: &data,
    })
    .expect("fixture stream builds");
    place_data(
        build_function_matrix(version).expect("fixture function matrix builds"),
        &stream,
        MaskId::new(mask_number).expect("fixture mask is valid"),
    )
    .expect("fixture placement succeeds")
}

fn synthetic_data(version_number: u8, length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| ((index * 149 + usize::from(version_number)) & 0xFF) as u8)
        .collect()
}

fn fixture_ecc(value: &str) -> ErrorCorrection {
    match value {
        "L" => ErrorCorrection::Low,
        "M" => ErrorCorrection::Medium,
        "Q" => ErrorCorrection::Quartile,
        "H" => ErrorCorrection::High,
        unexpected => panic!("unexpected fixture ECC {unexpected}"),
    }
}

fn fixture_matrix_text(matrix: &qr_core::matrix::ModuleMatrix) -> String {
    let size = matrix.size();
    let mut text = String::with_capacity(usize::from(size) * usize::from(size + 1));
    for y in 0..size {
        for x in 0..size {
            text.push(fixture_glyph(
                matrix.module(x, y).expect("coordinate is in bounds"),
            ));
        }
        text.push('\n');
    }
    text
}

fn fnv1a64(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |value, byte| {
        (value ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn fixture_glyph(module: Module) -> char {
    match (module.kind(), module.is_dark()) {
        (ModuleKind::Finder, true) => 'F',
        (ModuleKind::Finder, false) => 'f',
        (ModuleKind::Separator, false) => 's',
        (ModuleKind::Timing, true) => 'T',
        (ModuleKind::Timing, false) => 't',
        (ModuleKind::Alignment, true) => 'A',
        (ModuleKind::Alignment, false) => 'a',
        (ModuleKind::Format, false) => 'r',
        (ModuleKind::Version, false) => 'v',
        (ModuleKind::Dark, true) => 'D',
        (ModuleKind::Data, true) => 'B',
        (ModuleKind::Data, false) => 'b',
        (ModuleKind::Remainder, true) => 'E',
        (ModuleKind::Remainder, false) => 'e',
        unexpected => panic!("unexpected placed module {unexpected:?}"),
    }
}

fn fixture_module(glyph: char) -> Module {
    match glyph {
        'F' => Module::new(true, ModuleKind::Finder),
        'f' => Module::new(false, ModuleKind::Finder),
        's' => Module::new(false, ModuleKind::Separator),
        'T' => Module::new(true, ModuleKind::Timing),
        't' => Module::new(false, ModuleKind::Timing),
        'A' => Module::new(true, ModuleKind::Alignment),
        'a' => Module::new(false, ModuleKind::Alignment),
        'r' => Module::new(false, ModuleKind::Format),
        'v' => Module::new(false, ModuleKind::Version),
        'D' => Module::new(true, ModuleKind::Dark),
        'B' => Module::new(true, ModuleKind::Data),
        'b' => Module::new(false, ModuleKind::Data),
        'E' => Module::new(true, ModuleKind::Remainder),
        'e' => Module::new(false, ModuleKind::Remainder),
        unexpected => panic!("unexpected fixture glyph {unexpected:?}"),
    }
}
