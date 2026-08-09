#[allow(dead_code)]
#[path = "support/styling.rs"]
mod styling;
#[allow(dead_code)]
#[path = "support/versions.rs"]
mod versions;

use std::collections::HashSet;

use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_FINDERS, APPROVED_FOREGROUNDS, APPROVED_LOGO_STYLES,
    APPROVED_MODULE_STYLES, Background, FinderStyle, LogoStyle, ModuleStyle, Rgba,
    SUPPORTED_PROFILES,
};

#[test]
fn approved_configuration_lists_cover_the_complete_selectable_surface() {
    assert_eq!(SUPPORTED_PROFILES.len(), 4);
    assert_eq!(APPROVED_FOREGROUNDS.len(), 1);
    assert_eq!(
        APPROVED_BACKGROUNDS,
        [Background::Opaque(Rgba::WHITE), Background::Transparent]
    );
    assert_eq!(APPROVED_MODULE_STYLES, [ModuleStyle::CompactDots]);
    assert_eq!(APPROVED_FINDERS, [FinderStyle::StandardSquare]);
    assert_eq!(APPROVED_LOGO_STYLES, [LogoStyle::None, LogoStyle::Bundled]);

    let raw_tuple_count = SUPPORTED_PROFILES.len()
        * APPROVED_FOREGROUNDS.len()
        * APPROVED_BACKGROUNDS.len()
        * APPROVED_MODULE_STYLES.len()
        * APPROVED_FINDERS.len()
        * APPROVED_LOGO_STYLES.len();
    assert_eq!(raw_tuple_count, 16);
}

#[test]
fn generated_matrix_records_every_tuple_payload_and_expected_outcome() {
    let records = styling::approved_combination_records().expect("matrix generation succeeds");

    let required_payload_rows = 16 * styling::REQUIRED_PAYLOAD_CLASSES.len();
    let tuple_version_rows = SUPPORTED_PROFILES
        .iter()
        .map(|profile| usize::from(profile.maximum_version().number()) * 4)
        .sum::<usize>();
    assert_eq!(required_payload_rows, 96);
    assert_eq!(tuple_version_rows, 152);
    assert_eq!(records.len(), required_payload_rows + tuple_version_rows);
    assert_eq!(
        records
            .iter()
            .map(styling::ApprovedCombinationRecord::label)
            .collect::<HashSet<_>>()
            .len(),
        records.len(),
        "every matrix scenario needs a stable unique evidence ID"
    );

    for tuple in styling::approved_style_tuples() {
        let tuple_records = records
            .iter()
            .filter(|record| record.tuple.label() == tuple.label())
            .collect::<Vec<_>>();
        let payload_classes = tuple_records
            .iter()
            .filter(|record| record.case_kind == styling::MatrixCaseKind::RequiredPayload)
            .map(|record| record.payload_class)
            .collect::<HashSet<_>>();
        assert_eq!(
            payload_classes,
            styling::REQUIRED_PAYLOAD_CLASSES.into_iter().collect(),
            "{} required payload coverage",
            tuple.label()
        );
        let versions = tuple_records
            .iter()
            .filter(|record| record.case_kind == styling::MatrixCaseKind::VersionCoverage)
            .map(|record| record.version.expect("version rows identify their version"))
            .collect::<HashSet<_>>();
        assert_eq!(
            versions,
            (1..=tuple.profile.maximum_version().number()).collect(),
            "{} version coverage",
            tuple.label()
        );
    }

    for record in &records {
        if record.outcome.is_renderable() {
            assert_eq!(
                record.outcome.safety(),
                record.tuple.expected_safety(),
                "{}",
                record.label()
            );
        }
        assert!(
            record.outcome.is_renderable()
                || matches!(
                    record.outcome.expected_error(),
                    Some(
                        qr_render::RenderError::LogoRequiresOpaqueWhite
                            | qr_render::RenderError::UnsafeLogoGeometry
                    )
                ),
            "{}",
            record.label()
        );
        if record.outcome.is_renderable() && record.tuple.logo == LogoStyle::Bundled {
            assert_eq!(
                record.version,
                Some(6),
                "{} branded version",
                record.label()
            );
            let placement = record
                .logo_placement
                .expect("renderable branded rows record geometry");
            assert_eq!(placement.obscured_data_modules(), 105);
            assert_eq!(placement.obscured_remainder_modules(), 0);
            assert_eq!(placement.protected_clearance(), 6);
        } else {
            assert!(
                record.logo_placement.is_none(),
                "{} geometry",
                record.label()
            );
        }
    }
}
