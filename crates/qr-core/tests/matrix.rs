use qr_core::Version;
use qr_core::matrix::{MatrixBuilder, MatrixError, Module, ModuleKind, build_function_matrix};
use qr_core::tables;

#[test]
fn version_one_function_matrix_classifies_representative_cells() {
    let matrix = build_function_matrix(Version::new(1).expect("version 1 is valid"))
        .expect("version 1 function matrix builds");

    assert_eq!(matrix.size(), 21);
    assert_eq!(
        matrix.module(0, 0),
        Some(Module::new(true, ModuleKind::Finder))
    );
    assert_eq!(
        matrix.module(1, 1),
        Some(Module::new(false, ModuleKind::Finder))
    );
    assert_eq!(
        matrix.module(7, 7),
        Some(Module::new(false, ModuleKind::Separator))
    );
    assert_eq!(
        matrix.module(8, 6),
        Some(Module::new(true, ModuleKind::Timing))
    );
    assert_eq!(
        matrix.module(8, 8),
        Some(Module::new(false, ModuleKind::Format))
    );
    assert_eq!(
        matrix.module(8, 13),
        Some(Module::new(true, ModuleKind::Dark))
    );
    assert_eq!(
        matrix.module(9, 9),
        Some(Module::new(false, ModuleKind::Data))
    );
}

#[test]
fn version_two_places_the_first_alignment_pattern() {
    let matrix = build_function_matrix(Version::new(2).expect("version 2 is valid"))
        .expect("version 2 function matrix builds");

    assert_eq!(
        matrix.module(16, 16),
        Some(Module::new(true, ModuleKind::Alignment))
    );
    assert_eq!(
        matrix.module(17, 17),
        Some(Module::new(false, ModuleKind::Alignment))
    );
    assert_eq!(
        matrix.module(18, 18),
        Some(Module::new(true, ModuleKind::Alignment))
    );
}

#[test]
fn version_information_regions_start_at_version_seven() {
    let version_six = build_function_matrix(Version::new(6).expect("version 6 is valid"))
        .expect("version 6 function matrix builds");
    assert_eq!(
        version_six
            .modules()
            .filter(|module| module.kind() == ModuleKind::Version)
            .count(),
        0
    );

    let version_seven = build_function_matrix(Version::new(7).expect("version 7 is valid"))
        .expect("version 7 function matrix builds");
    let size = version_seven.size();
    for offset in 0..6 {
        for band in 0..3 {
            assert_eq!(
                version_seven.module(size - 11 + band, offset),
                Some(Module::new(false, ModuleKind::Version))
            );
            assert_eq!(
                version_seven.module(offset, size - 11 + band),
                Some(Module::new(false, ModuleKind::Version))
            );
        }
    }
}

#[test]
fn builder_rejects_out_of_bounds_and_duplicate_writes() {
    let version = Version::new(1).expect("version 1 is valid");
    let mut builder = MatrixBuilder::new(version).expect("builder allocation succeeds");
    let finder = Module::new(true, ModuleKind::Finder);

    assert_eq!(
        builder.write(21, 0, finder),
        Err(MatrixError::OutOfBounds {
            x: 21,
            y: 0,
            size: 21,
        })
    );
    assert_eq!(builder.write(0, 0, finder), Ok(()));
    assert_eq!(
        builder.write(0, 0, finder),
        Err(MatrixError::DoubleWrite { x: 0, y: 0 })
    );
}

#[test]
fn builder_rejects_invalid_reservations_and_incomplete_finalization() {
    let version = Version::new(1).expect("version 1 is valid");
    let mut builder = MatrixBuilder::new(version).expect("builder allocation succeeds");

    assert_eq!(
        builder.reserve(0, 0, ModuleKind::Data),
        Err(MatrixError::InvalidReservation {
            x: 0,
            y: 0,
            kind: ModuleKind::Data,
        })
    );
    assert_eq!(
        builder.reserve(0, 0, ModuleKind::Format),
        Err(MatrixError::InvalidReservation {
            x: 0,
            y: 0,
            kind: ModuleKind::Format,
        })
    );
    assert_eq!(
        builder.reserve(0, 0, ModuleKind::Version),
        Err(MatrixError::InvalidReservation {
            x: 0,
            y: 0,
            kind: ModuleKind::Version,
        })
    );
    assert_eq!(builder.reserve(8, 0, ModuleKind::Format), Ok(()));
    assert_eq!(
        builder.write(9, 9, Module::new(false, ModuleKind::Format)),
        Err(MatrixError::InvalidReservation {
            x: 9,
            y: 9,
            kind: ModuleKind::Format,
        })
    );
    assert_eq!(
        builder.finish(),
        Err(MatrixError::Incomplete { unwritten: 440 })
    );

    let mut version_seven =
        MatrixBuilder::new(Version::new(7).expect("version 7 is valid")).expect("builder exists");
    assert_eq!(version_seven.reserve(34, 0, ModuleKind::Version), Ok(()));
}

#[test]
fn every_version_is_complete_and_leaves_exactly_the_raw_data_region() {
    for version_number in Version::MIN..=Version::MAX {
        let version = Version::new(version_number).expect("generated version is valid");
        let info = tables::version_info(version).expect("version table is complete");
        let matrix = build_function_matrix(version).expect("function matrix builds");
        let modules = matrix.modules().collect::<Vec<_>>();
        let count = |kind| {
            modules
                .iter()
                .filter(|module| module.kind() == kind)
                .count()
        };

        assert_eq!(matrix.size(), version.symbol_size());
        assert_eq!(
            modules.len(),
            usize::from(matrix.size()) * usize::from(matrix.size())
        );
        assert_eq!(count(ModuleKind::Finder), 3 * 7 * 7);
        assert_eq!(count(ModuleKind::Separator), 3 * 15);
        assert_eq!(count(ModuleKind::Format), 30);
        assert_eq!(count(ModuleKind::Dark), 1);
        assert_eq!(
            count(ModuleKind::Version),
            if version_number >= 7 { 36 } else { 0 }
        );
        assert_eq!(
            count(ModuleKind::Alignment),
            info.alignment_pattern_positions().count() * 25
        );
        assert_eq!(
            count(ModuleKind::Data),
            usize::from(info.total_codewords()) * 8 + usize::from(info.remainder_bits())
        );
        assert_eq!(count(ModuleKind::Remainder), 0);
        assert!(
            modules
                .iter()
                .filter(|module| matches!(
                    module.kind(),
                    ModuleKind::Data
                        | ModuleKind::Separator
                        | ModuleKind::Format
                        | ModuleKind::Version
                ))
                .all(|module| !module.is_dark())
        );
        assert_eq!(matrix.module(matrix.size(), 0), None);
        assert_eq!(matrix.module(0, matrix.size()), None);
    }
}

#[test]
fn version_forty_places_all_alignment_centers_without_finder_conflicts() {
    let version = Version::new(40).expect("version 40 is valid");
    let info = tables::version_info(version).expect("version 40 table is present");
    let matrix = build_function_matrix(version).expect("version 40 function matrix builds");

    assert_eq!(matrix.size(), 177);
    assert_eq!(info.alignment_pattern_positions().count(), 46);
    for (x, y) in info.alignment_pattern_positions() {
        assert_eq!(
            matrix.module(u16::from(x), u16::from(y)),
            Some(Module::new(true, ModuleKind::Alignment))
        );
    }
}

#[test]
fn every_version_and_human_review_case_matches_dual_oracle_fixtures() {
    let fixture = include_str!("../../../tests/fixtures/function_matrices.txt");
    let mut lines = fixture.lines().filter(|line| !line.starts_with('#'));
    assert_eq!(lines.next(), Some("version,size,fnv1a64"));
    for expected_version in Version::MIN..=Version::MAX {
        let row = lines.next().expect("fixture contains every version hash");
        let fields = row.split(',').collect::<Vec<_>>();
        assert_eq!(fields.len(), 3);
        let version_number = fields[0].parse::<u8>().expect("hash version is a u8");
        let size = fields[1].parse::<u16>().expect("hash size is a u16");
        assert_eq!(version_number, expected_version);
        let version = Version::new(version_number).expect("hash version is valid");
        let matrix = build_function_matrix(version).expect("hashed function matrix builds");
        assert_eq!(matrix.size(), size);
        assert_eq!(
            format!("{:016x}", fnv1a64(&fixture_matrix_text(&matrix))),
            fields[2]
        );
    }
    assert_eq!(lines.next(), Some("endhashes"));

    let mut covered_versions = Vec::new();

    while let Some(header) = lines.next() {
        let mut fields = header.split_ascii_whitespace();
        let version_number = fields
            .next()
            .and_then(|field| field.strip_prefix("version="))
            .expect("fixture version header")
            .parse::<u8>()
            .expect("fixture version is a u8");
        let size = fields
            .next()
            .and_then(|field| field.strip_prefix("size="))
            .expect("fixture size header")
            .parse::<u16>()
            .expect("fixture size is a u16");
        assert_eq!(fields.next(), None);

        let version = Version::new(version_number).expect("fixture version is valid");
        let matrix = build_function_matrix(version).expect("fixture matrix builds");
        assert_eq!(matrix.size(), size);
        for y in 0..size {
            let row = lines.next().expect("fixture contains every row");
            assert_eq!(row.len(), usize::from(size));
            for (x, glyph) in row.chars().enumerate() {
                assert_eq!(
                    matrix.module(u16::try_from(x).expect("fixture x fits u16"), y),
                    Some(fixture_module(glyph)),
                    "Version {version_number} coordinate ({x}, {y})"
                );
            }
        }
        assert_eq!(lines.next(), Some("end"));
        covered_versions.push(version_number);
    }

    assert_eq!(covered_versions, vec![1, 2, 7, 40]);
}

fn fixture_matrix_text(matrix: &qr_core::matrix::ModuleMatrix) -> String {
    let size = matrix.size();
    let mut text = String::with_capacity(usize::from(size) * usize::from(size + 1));
    for y in 0..size {
        for x in 0..size {
            text.push(fixture_glyph(
                matrix.module(x, y).expect("matrix coordinate is in bounds"),
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
        (ModuleKind::Data, false) => '.',
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
        unexpected => panic!("unexpected classified module {unexpected:?}"),
    }
}

fn fixture_module(glyph: char) -> Module {
    match glyph {
        '.' => Module::new(false, ModuleKind::Data),
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
        unexpected => panic!("unexpected fixture glyph {unexpected:?}"),
    }
}
