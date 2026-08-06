use qr_core::tables::ErrorCorrection;
use qr_render::ProfileId;
use qr_web::workflow::{WorkflowFailure, WorkflowState, evaluate_preview};

#[test]
fn payload_entry_preserves_text_and_reports_character_and_byte_counts() {
    let mut state = WorkflowState::new(ProfileId::Inline);

    state
        .set_payload("  café\n".to_owned())
        .expect("revision is available");

    assert_eq!(state.payload(), "  café\n");
    assert_eq!(state.character_count(), 7);
    assert_eq!(state.byte_count(), 8);
}

#[test]
fn textarea_edits_preserve_unchanged_carriage_return_sequences() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    state
        .set_payload("first\r\nsecond\rthird".to_owned())
        .expect("revision is available");

    state
        .set_display_payload_at("first\nsecond\nthird!".to_owned(), 18)
        .expect("display edit maps to raw payload");

    assert_eq!(state.payload(), "first\r\nsecond\rthird!");
    assert_eq!(state.textarea_value(), "first\nsecond\nthird!");
}

#[test]
fn edit_anchor_disambiguates_identical_normalized_line_endings() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    state
        .set_payload("\r\n\r".to_owned())
        .expect("revision is available");

    state
        .set_display_payload_at("\n".to_owned(), 0)
        .expect("first displayed newline is deleted");
    assert_eq!(state.payload(), "\r");

    state
        .set_payload("\r\n\r".to_owned())
        .expect("revision is available");
    state
        .set_display_payload_at("\n".to_owned(), 1)
        .expect("second displayed newline is deleted");
    assert_eq!(state.payload(), "\r\n");
}

#[test]
fn raw_paste_replaces_a_textarea_selection_without_normalizing_line_endings() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    state
        .set_payload("before after".to_owned())
        .expect("revision is available");

    state
        .replace_display_range(7, 7, "one\r\ntwo\r")
        .expect("selection maps to raw payload");

    assert_eq!(state.payload(), "before one\r\ntwo\rafter");
    assert_eq!(state.textarea_value(), "before one\ntwo\nafter");
}

#[test]
fn internal_drag_reads_the_exact_raw_text_for_the_display_selection() {
    let mut state = WorkflowState::new(ProfileId::Content);
    state
        .replace_display_range(0, 0, "a\r\nb\rc")
        .expect("synthetic payload should fit the input limit");

    assert_eq!(
        state.raw_text_for_display_range(1, 3).as_deref(),
        Ok("\r\nb")
    );
}

#[test]
fn safe_payload_fits_at_ecc_m_and_reports_exact_diagnostics() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    let request = state
        .set_payload("hello".to_owned())
        .expect("revision is available");

    assert_eq!(request.ecc(), ErrorCorrection::Medium);
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));

    let preview = state.preview().expect("valid payload has a preview");
    let diagnostics = preview.diagnostics();
    assert_eq!(diagnostics.ecc(), ErrorCorrection::Medium);
    assert_eq!(diagnostics.maximum_version().number(), 5);
    assert_eq!(diagnostics.selected_version().number(), 1);
    assert_eq!(diagnostics.used_data_bits(), 52);
    assert_eq!(diagnostics.available_data_bits(), 128);
    assert_eq!(diagnostics.data_codewords(), 16);
    assert_eq!(diagnostics.matrix_modules(), 21);
    assert_eq!(diagnostics.svg_side_pixels(), 90);
    assert_eq!(diagnostics.png_side_pixels(), 270);
    assert!(state.exports_enabled());
}

#[test]
fn every_profile_derives_its_limit_dimensions_and_guidance() {
    let cases = [
        (ProfileId::Inline, 5, 90, 270, false),
        (ProfileId::Content, 8, 120, 360, false),
        (ProfileId::Landing, 12, 150, 450, false),
        (ProfileId::Print, 13, 160, 480, true),
    ];

    for (profile_id, maximum_version, svg_side, png_side, is_print) in cases {
        let mut state = WorkflowState::new(profile_id);
        let request = state
            .set_payload("hello".to_owned())
            .expect("revision is available");
        assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));
        let diagnostics = state.preview().expect("short payload fits").diagnostics();
        assert_eq!(diagnostics.ecc(), ErrorCorrection::Medium);
        assert_eq!(diagnostics.maximum_version().number(), maximum_version);
        assert_eq!(diagnostics.svg_side_pixels(), svg_side);
        assert_eq!(diagnostics.png_side_pixels(), png_side);
        assert_eq!(
            diagnostics.print_guidance()
                == "Place at 25–30 mm or larger; validate for the actual environment.",
            is_print,
        );
    }
}

#[test]
fn profile_changes_refit_without_changing_safe_ecc() {
    let mut state = WorkflowState::new(ProfileId::Content);
    let payload_request = state
        .set_payload("a".repeat(90))
        .expect("revision is available");
    assert!(state.complete_preview(
        payload_request.revision(),
        evaluate_preview(&payload_request),
    ));
    assert_eq!(
        state
            .preview()
            .expect("content profile fits")
            .diagnostics()
            .selected_version()
            .number(),
        6,
    );

    let inline_request = state
        .select_profile(ProfileId::Inline)
        .expect("revision is available");
    assert_eq!(inline_request.ecc(), ErrorCorrection::Medium);
    assert!(state.complete_preview(inline_request.revision(), evaluate_preview(&inline_request),));
    assert!(!state.exports_enabled());

    let print_request = state
        .select_profile(ProfileId::Print)
        .expect("revision is available");
    assert_eq!(print_request.ecc(), ErrorCorrection::Medium);
    assert!(state.complete_preview(print_request.revision(), evaluate_preview(&print_request),));
    assert_eq!(
        state
            .preview()
            .expect("print profile fits")
            .diagnostics()
            .selected_version()
            .number(),
        6,
    );
}

#[test]
fn logo_transition_uses_ecc_h_before_fitting_and_disabling_restores_m() {
    let mut state = WorkflowState::new(ProfileId::Print);
    let safe_request = state
        .set_payload("a".repeat(70))
        .expect("revision is available");
    assert!(state.complete_preview(safe_request.revision(), evaluate_preview(&safe_request),));
    assert_eq!(
        state
            .preview()
            .expect("safe preview fits")
            .diagnostics()
            .selected_version()
            .number(),
        5,
    );

    let logo_request = state.set_logo_enabled(true).expect("revision is available");
    assert_eq!(logo_request.ecc(), ErrorCorrection::High);
    assert!(state.complete_preview(logo_request.revision(), evaluate_preview(&logo_request),));
    let logo_diagnostics = state.preview().expect("logo preview fits").diagnostics();
    assert_eq!(logo_diagnostics.ecc(), ErrorCorrection::High);
    assert_eq!(logo_diagnostics.selected_version().number(), 8);

    let restored_request = state
        .set_logo_enabled(false)
        .expect("revision is available");
    assert_eq!(restored_request.ecc(), ErrorCorrection::Medium);
    assert!(state.complete_preview(
        restored_request.revision(),
        evaluate_preview(&restored_request),
    ));
    let restored = state
        .preview()
        .expect("safe preview is restored")
        .diagnostics();
    assert_eq!(restored.ecc(), ErrorCorrection::Medium);
    assert_eq!(restored.selected_version().number(), 5);
}

#[test]
fn invalid_and_internal_results_have_associated_messages_and_disable_exports() {
    let mut state = WorkflowState::new(ProfileId::Inline);

    let empty = state
        .set_payload(String::new())
        .expect("revision is available");
    assert!(state.complete_preview(empty.revision(), evaluate_preview(&empty)));
    assert_eq!(
        state.validation_message().as_deref(),
        Some("Enter text to generate a QR code."),
    );
    assert!(!state.exports_enabled());

    let over_limit = state
        .set_payload("x".repeat(4097))
        .expect("revision is available");
    assert!(state.complete_preview(over_limit.revision(), evaluate_preview(&over_limit)));
    assert_eq!(
        state.validation_message().as_deref(),
        Some("The payload is 4097 bytes; the input limit is 4096 bytes."),
    );
    assert!(!state.exports_enabled());

    let over_capacity = state
        .set_payload("x".repeat(100))
        .expect("revision is available");
    assert!(state.complete_preview(over_capacity.revision(), evaluate_preview(&over_capacity),));
    assert_eq!(
        state.validation_message().as_deref(),
        Some("The payload does not fit this profile's maximum QR version 5."),
    );
    assert!(!state.exports_enabled());

    let internal = state
        .set_payload("valid".to_owned())
        .expect("revision is available");
    assert!(state.complete_preview(internal.revision(), Err(WorkflowFailure::Internal)));
    assert_eq!(
        state.validation_message().as_deref(),
        Some("QR generation failed unexpectedly. Change the input and try again."),
    );
    assert!(!state.exports_enabled());
}

#[test]
fn textarea_mapping_failure_invalidates_pending_work_and_exports() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    let pending = state
        .set_payload("valid".to_owned())
        .expect("revision is available");

    assert!(state.replace_display_range(99, 99, "x").is_err());
    assert_eq!(
        state.validation_message().as_deref(),
        Some("QR generation failed unexpectedly. Change the input and try again."),
    );
    assert!(!state.exports_enabled());
    assert!(!state.complete_preview(pending.revision(), evaluate_preview(&pending)));
}

#[test]
fn control_characters_add_a_deterministic_caution_without_changing_valid_text() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    let request = state
        .set_payload("line one\nline two\t".to_owned())
        .expect("revision is available");
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));

    assert_eq!(state.payload(), "line one\nline two\t");
    assert_eq!(
        state.caution(),
        Some("This payload contains control characters. Confirm that they are intentional."),
    );
    assert!(state.exports_enabled());
}

#[test]
fn stale_preview_results_cannot_replace_the_latest_value() {
    let mut state = WorkflowState::new(ProfileId::Print);
    let stale = state
        .set_payload("old".to_owned())
        .expect("revision is available");
    let latest = state
        .set_payload("n".repeat(200))
        .expect("revision is available");
    assert!(!state.exports_enabled());

    assert!(state.complete_preview(latest.revision(), evaluate_preview(&latest)));
    let selected_version = state
        .preview()
        .expect("latest value fits")
        .diagnostics()
        .selected_version();
    assert!(!state.complete_preview(stale.revision(), evaluate_preview(&stale)));
    assert_eq!(
        state
            .preview()
            .expect("stale result did not clear preview")
            .diagnostics()
            .selected_version(),
        selected_version,
    );
}
