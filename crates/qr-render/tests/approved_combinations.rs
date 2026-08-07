#[allow(dead_code)]
#[path = "support/styling.rs"]
mod styling;
#[allow(dead_code)]
#[path = "support/versions.rs"]
mod versions;

use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_DATA_MODULE_STYLES, APPROVED_FINDERS, APPROVED_FOREGROUNDS,
    APPROVED_FUNCTION_MODULE_STYLES, APPROVED_LOGO_STYLES, Background, FinderStyle,
    FunctionModuleStyle, LogoStyle, Rgba, SUPPORTED_PROFILES,
};

#[test]
fn approved_configuration_lists_cover_the_complete_selectable_surface() {
    assert_eq!(SUPPORTED_PROFILES.len(), 4);
    assert_eq!(APPROVED_FOREGROUNDS.len(), 1);
    assert_eq!(
        APPROVED_BACKGROUNDS,
        [Background::Opaque(Rgba::WHITE), Background::Transparent]
    );
    assert_eq!(APPROVED_DATA_MODULE_STYLES.len(), 2);
    assert_eq!(
        APPROVED_FUNCTION_MODULE_STYLES,
        [FunctionModuleStyle::Square]
    );
    assert_eq!(APPROVED_FINDERS, [FinderStyle::StandardSquare]);
    assert_eq!(APPROVED_LOGO_STYLES, [LogoStyle::None, LogoStyle::Bundled]);

    let raw_tuple_count = SUPPORTED_PROFILES.len()
        * APPROVED_FOREGROUNDS.len()
        * APPROVED_BACKGROUNDS.len()
        * APPROVED_DATA_MODULE_STYLES.len()
        * APPROVED_FUNCTION_MODULE_STYLES.len()
        * APPROVED_FINDERS.len()
        * APPROVED_LOGO_STYLES.len();
    assert_eq!(raw_tuple_count, 32);
}

#[test]
fn generated_matrix_records_every_tuple_payload_and_expected_outcome() {
    let records = styling::approved_combination_records().expect("matrix generation succeeds");

    assert_eq!(records.len(), 32 * styling::REQUIRED_PAYLOAD_CLASSES.len());
    assert_eq!(
        records
            .iter()
            .filter(|record| record.outcome.is_renderable())
            .count(),
        24 * styling::REQUIRED_PAYLOAD_CLASSES.len()
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| record.outcome.is_expected_invalid())
            .count(),
        8 * styling::REQUIRED_PAYLOAD_CLASSES.len()
    );

    for record in records {
        assert_eq!(
            record.outcome.safety(),
            record.tuple.expected_safety(),
            "{}",
            record.label()
        );
        assert!(
            record.outcome.is_renderable()
                || record.outcome.expected_error()
                    == Some(qr_render::RenderError::LogoRequiresOpaqueWhite),
            "{}",
            record.label()
        );
    }
}
