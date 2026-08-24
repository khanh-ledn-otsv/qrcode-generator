#[allow(dead_code)]
#[path = "support/styling.rs"]
mod styling;
#[allow(dead_code)]
#[path = "support/versions.rs"]
mod versions;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use qr_render::{
    APPROVED_FOREGROUND_THEMES, APPROVED_LOGO_STYLES, ForegroundTheme, LogoStyle,
    MAXIMUM_ADAPTIVE_LOGO_VERSION, SUPPORTED_PROFILES,
};

fn matrix_policy() -> serde_json::Value {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    serde_json::from_slice(
        &fs::read(workspace.join("tests/approved-output-matrix-policy.json"))
            .expect("approved-output matrix policy is readable"),
    )
    .expect("approved-output matrix policy is valid JSON")
}

#[test]
fn approved_configuration_lists_cover_the_complete_selectable_surface() {
    let policy = matrix_policy();
    let dimensions = &policy["tuple_dimensions"];
    assert_eq!(dimensions["profiles"], SUPPORTED_PROFILES.len());
    assert_eq!(dimensions["logo_states"], APPROVED_LOGO_STYLES.len());
    assert_eq!(
        dimensions["foreground_themes"],
        APPROVED_FOREGROUND_THEMES.len()
    );
    assert_eq!(APPROVED_LOGO_STYLES, [LogoStyle::None, LogoStyle::Bundled]);
    assert_eq!(
        APPROVED_FOREGROUND_THEMES,
        [ForegroundTheme::Magenta, ForegroundTheme::Black]
    );

    let raw_tuple_count =
        SUPPORTED_PROFILES.len() * APPROVED_LOGO_STYLES.len() * APPROVED_FOREGROUND_THEMES.len();
    assert_eq!(
        raw_tuple_count,
        dimensions
            .as_object()
            .expect("tuple dimensions are an object")
            .values()
            .map(|value| {
                usize::try_from(value.as_u64().expect("tuple dimension is unsigned"))
                    .expect("tuple dimension fits usize")
            })
            .product::<usize>()
    );
}

#[test]
#[ignore = "exhaustive approved matrix runs in release evidence and extended CI"]
fn generated_matrix_records_every_tuple_payload_and_expected_outcome() {
    let records = styling::approved_combination_records().expect("matrix generation succeeds");
    let policy = matrix_policy();
    assert_eq!(policy["schema_version"], 1);
    let expected_rows = &policy["expected_rows"];

    let tuples = styling::approved_style_tuples();
    let required_payload_rows = tuples.len() * styling::REQUIRED_PAYLOAD_CLASSES.len();
    let tuple_version_rows = tuples
        .iter()
        .map(|tuple| usize::from(tuple.profile.maximum_version().number()))
        .sum::<usize>();
    assert_eq!(
        policy["tuple_dimensions"]["profiles"],
        SUPPORTED_PROFILES.len()
    );
    assert_eq!(
        policy["profile_max_versions"],
        serde_json::json!(
            SUPPORTED_PROFILES
                .iter()
                .map(|profile| profile.maximum_version().number())
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        policy["required_payload_classes"],
        serde_json::json!(
            styling::REQUIRED_PAYLOAD_CLASSES
                .iter()
                .map(|class| class.label())
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(expected_rows["required_payload"], required_payload_rows);
    assert_eq!(expected_rows["version_coverage"], tuple_version_rows);
    assert_eq!(expected_rows["total"], records.len());
    assert_eq!(
        expected_rows["decoded"],
        records
            .iter()
            .filter(|record| record.outcome.is_renderable())
            .count()
    );
    assert_eq!(
        expected_rows["expected_invalid"],
        records
            .iter()
            .filter(|record| record.outcome.is_expected_invalid())
            .count()
    );
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
                Some(record.tuple.expected_safety()),
                "{}",
                record.label()
            );
        }
        assert!(
            record.outcome.is_renderable()
                || matches!(
                    record.outcome.expected_error(),
                    Some(qr_render::RenderError::UnsafeLogoGeometry)
                ),
            "{}",
            record.label()
        );
        if record.outcome.is_renderable() && record.tuple.logo == LogoStyle::Bundled {
            let version = record.version.expect("branded rows record a version");
            assert!(
                version == 6
                    || (record.tuple.profile.id() == qr_render::ProfileId::Adaptive
                        && version <= MAXIMUM_ADAPTIVE_LOGO_VERSION.number()),
                "{} branded version",
                record.label()
            );
            let placement = record
                .logo_placement
                .expect("renderable branded rows record geometry");
            assert_eq!(placement.obscured_data_modules(), 105);
            assert_eq!(placement.obscured_remainder_modules(), 0);
            if version == 6 {
                assert_eq!(placement.protected_clearance(), 6);
            }
        } else {
            assert!(
                record.logo_placement.is_none(),
                "{} geometry",
                record.label()
            );
        }
    }
}
