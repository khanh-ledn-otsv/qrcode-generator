use qr_core::Version;
use qr_core::tables::{
    DataMode, ErrorCorrection, TableLookupError, character_count_bits,
    character_count_bits_for_version_number, lookup, lookup_version_number, version_info,
    version_info_for_number,
};
use std::str::Split;

#[derive(Debug)]
struct FixtureGroup {
    block_count: u8,
    data_codewords_per_block: u16,
}

#[derive(Debug)]
struct FixtureRow {
    version: u8,
    ecc: ErrorCorrection,
    total_codewords: u16,
    data_codewords: u16,
    ecc_codewords_per_block: u8,
    groups: [Option<FixtureGroup>; 2],
    remainder_bits: u8,
    alignment_centers: Vec<u8>,
    character_count_bits: [u8; 4],
}

fn next_field<'a>(fields: &mut Split<'a, char>, name: &str) -> &'a str {
    fields
        .next()
        .unwrap_or_else(|| panic!("fixture row is missing {name}"))
}

fn next_u16(fields: &mut Split<'_, char>, name: &str) -> u16 {
    next_field(fields, name)
        .parse()
        .unwrap_or_else(|error| panic!("fixture {name} is not a u16: {error}"))
}

fn next_u8(fields: &mut Split<'_, char>, name: &str) -> u8 {
    u8::try_from(next_u16(fields, name))
        .unwrap_or_else(|error| panic!("fixture {name} is not a u8: {error}"))
}

fn parse_fixture_row(line: &str) -> FixtureRow {
    let mut fields = line.split(',');
    let version = next_u8(&mut fields, "version");
    let ecc = match next_field(&mut fields, "ECC") {
        "L" => ErrorCorrection::Low,
        "M" => ErrorCorrection::Medium,
        "Q" => ErrorCorrection::Quartile,
        "H" => ErrorCorrection::High,
        other => panic!("unexpected fixture ECC {other}"),
    };
    let total_codewords = next_u16(&mut fields, "total codewords");
    let data_codewords = next_u16(&mut fields, "data codewords");
    let ecc_codewords_per_block = next_u8(&mut fields, "ECC codewords per block");
    let first_group = FixtureGroup {
        block_count: next_u8(&mut fields, "first-group block count"),
        data_codewords_per_block: next_u16(&mut fields, "first-group data codewords"),
    };
    let second_count = next_u8(&mut fields, "second-group block count");
    let second_data = next_u16(&mut fields, "second-group data codewords");
    let second_group = (second_count != 0).then_some(FixtureGroup {
        block_count: second_count,
        data_codewords_per_block: second_data,
    });
    let remainder_bits = next_u8(&mut fields, "remainder bits");
    let alignment_centers = next_field(&mut fields, "alignment centers");
    let alignment_centers = if alignment_centers.is_empty() {
        Vec::new()
    } else {
        alignment_centers
            .split(';')
            .map(|value| {
                value
                    .parse()
                    .unwrap_or_else(|error| panic!("invalid alignment center: {error}"))
            })
            .collect()
    };
    let character_count_bits = [
        next_u8(&mut fields, "numeric character-count bits"),
        next_u8(&mut fields, "alphanumeric character-count bits"),
        next_u8(&mut fields, "byte character-count bits"),
        next_u8(&mut fields, "Kanji character-count bits"),
    ];
    assert!(
        fields.next().is_none(),
        "fixture row has unexpected columns"
    );
    FixtureRow {
        version,
        ecc,
        total_codewords,
        data_codewords,
        ecc_codewords_per_block,
        groups: [Some(first_group), second_group],
        remainder_bits,
        alignment_centers,
        character_count_bits,
    }
}

fn fixture_rows() -> Vec<FixtureRow> {
    include_str!("../../../tests/fixtures/qr_tables.csv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .skip(1)
        .map(parse_fixture_row)
        .collect()
}

#[test]
fn every_oracle_row_matches_the_public_lookup() {
    let rows = fixture_rows();
    assert_eq!(rows.len(), 160);

    for fixture in rows {
        let version = Version::new(fixture.version).expect("fixture version is valid");
        let row = lookup(version, fixture.ecc).expect("fixture lookup succeeds");
        assert_eq!(row.version().number(), fixture.version);
        assert_eq!(row.error_correction(), fixture.ecc);
        assert_eq!(row.total_codewords(), fixture.total_codewords);
        assert_eq!(row.data_codewords(), fixture.data_codewords);
        assert_eq!(
            row.ecc_codewords_per_block(),
            fixture.ecc_codewords_per_block
        );
        assert_eq!(row.remainder_bits(), fixture.remainder_bits);

        let mut groups = row.block_groups();
        for expected in fixture.groups.into_iter().flatten() {
            let actual = groups.next().expect("expected block group exists");
            assert_eq!(actual.block_count(), expected.block_count);
            assert_eq!(
                actual.data_codewords_per_block(),
                expected.data_codewords_per_block
            );
        }
        assert!(
            groups.next().is_none(),
            "lookup returned an extra block group"
        );

        let expanded_data = row
            .block_groups()
            .map(|group| u16::from(group.block_count()) * group.data_codewords_per_block())
            .sum::<u16>();
        let block_count = row
            .block_groups()
            .map(|group| u16::from(group.block_count()))
            .sum::<u16>();
        assert_eq!(expanded_data, row.data_codewords());
        assert_eq!(
            expanded_data + block_count * u16::from(row.ecc_codewords_per_block()),
            row.total_codewords()
        );
        assert_eq!(row.ecc_codewords(), row.total_codewords() - expanded_data);
    }
}

#[test]
fn version_metadata_matches_every_oracle_row_and_matrix_accounting() {
    for fixture in fixture_rows() {
        let version = Version::new(fixture.version).expect("fixture version is valid");
        let info = version_info(version).expect("fixture version exists");
        assert_eq!(info.alignment_pattern_centers(), fixture.alignment_centers);
        assert_eq!(info.remainder_bits(), fixture.remainder_bits);
        assert_eq!(info.symbol_size(), 17 + u16::from(fixture.version) * 4);

        let centers = info.alignment_pattern_centers();
        assert!(centers.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            centers
                .iter()
                .all(|center| u16::from(*center) < info.symbol_size())
        );

        let positions = info.alignment_pattern_positions().collect::<Vec<_>>();
        if let (Some(&first), Some(&last)) = (centers.first(), centers.last()) {
            assert_eq!(positions.len(), centers.len() * centers.len() - 3);
            assert!(positions.iter().all(|&(x, y)| {
                !((x == first && (y == first || y == last)) || (x == last && y == first))
            }));
            assert!(
                positions
                    .iter()
                    .all(|(x, y)| centers.contains(x) && centers.contains(y))
            );
        } else {
            assert!(positions.is_empty());
        }

        let version = u32::from(fixture.version);
        let mut raw_modules = (16 * version + 128) * version + 64;
        if version >= 2 {
            let alignment_count = u32::try_from(centers.len()).expect("small center count");
            raw_modules -= (25 * alignment_count - 10) * alignment_count - 55;
            if version >= 7 {
                raw_modules -= 36;
            }
        }
        assert_eq!(raw_modules / 8, u32::from(fixture.total_codewords));
        assert_eq!(raw_modules % 8, u32::from(info.remainder_bits()));
    }
}

#[test]
fn character_count_bands_and_profile_ceiling_versions_are_explicit() {
    let modes = [
        DataMode::Numeric,
        DataMode::Alphanumeric,
        DataMode::Byte,
        DataMode::Kanji,
    ];
    for version in [1, 5, 8, 9, 10, 12, 13, 26, 27, 40] {
        let fixture = fixture_rows()
            .into_iter()
            .find(|row| row.version == version)
            .expect("regression version exists");
        for (mode, expected) in modes.into_iter().zip(fixture.character_count_bits) {
            assert_eq!(
                character_count_bits(
                    Version::new(version).expect("regression version is valid"),
                    mode,
                ),
                expected
            );
        }
    }
}

#[test]
fn invalid_versions_return_typed_errors() {
    for version in [0, 41, u8::MAX] {
        assert!(matches!(
            lookup_version_number(version, ErrorCorrection::Low),
            Err(TableLookupError::InvalidVersion(error)) if error.number() == version
        ));
        assert!(matches!(
            version_info_for_number(version),
            Err(TableLookupError::InvalidVersion(error)) if error.number() == version
        ));
        assert!(matches!(
            character_count_bits_for_version_number(version, DataMode::Byte),
            Err(TableLookupError::InvalidVersion(error)) if error.number() == version
        ));
    }
}
