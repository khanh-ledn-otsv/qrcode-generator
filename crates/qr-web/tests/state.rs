use qr_core::tables::{DataMode, ErrorCorrection};
use qr_render::{Background, ContrastRatio, Foreground, OutputSafety, ProfileId, Rgba};
use qr_web::workflow::{ArtifactKind, WorkflowFailure, WorkflowState, evaluate_preview};

fn state_without_logo(profile_id: ProfileId) -> WorkflowState {
    let mut state = WorkflowState::new(profile_id);
    state
        .set_logo_enabled(false)
        .expect("initial logo selection can be disabled");
    state
}

#[test]
fn bundled_logo_is_selected_by_default() {
    let state = WorkflowState::new(ProfileId::Content);

    assert!(state.logo_enabled());
    assert_eq!(state.background(), Background::Opaque(Rgba::WHITE));
}

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
    let mut state = state_without_logo(ProfileId::Inline);
    let request = state
        .set_payload("hello".to_owned())
        .expect("revision is available");

    assert_eq!(request.ecc(), ErrorCorrection::Medium);
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));

    let preview = state.preview().expect("valid payload has a preview");
    let diagnostics = preview.diagnostics();
    assert_eq!(diagnostics.ecc(), ErrorCorrection::Medium);
    assert_eq!(diagnostics.maximum_version().number(), 6);
    assert_eq!(diagnostics.selected_version().number(), 1);
    assert_eq!(diagnostics.used_data_bits(), 52);
    assert_eq!(diagnostics.available_data_bits(), 128);
    assert_eq!(diagnostics.data_codewords(), 16);
    assert_eq!(diagnostics.matrix_modules(), 21);
    assert_eq!(diagnostics.svg_side_pixels(), 100);
    assert_eq!(diagnostics.png_side_pixels(), 300);
    assert_eq!(diagnostics.foreground(), Foreground::Brand);
    assert_eq!(diagnostics.background(), Background::Opaque(Rgba::WHITE));
    assert_eq!(diagnostics.safety(), OutputSafety::Safe);
    assert_eq!(
        diagnostics.contrast_ratio(),
        Some(ContrastRatio::from_hundredths(604))
    );
    assert!(state.exports_enabled());
}

#[test]
fn approved_brand_and_transparency_selections_update_artifacts_and_safety() {
    let mut state = state_without_logo(ProfileId::Content);
    let payload = state
        .set_payload("approved appearance".to_owned())
        .expect("revision is available");
    assert!(state.complete_preview(payload.revision(), evaluate_preview(&payload)));
    let original = state.preview().unwrap().diagnostics();

    let brand = state
        .select_foreground(Foreground::Brand)
        .expect("revision is available");
    assert!(state.complete_preview(brand.revision(), evaluate_preview(&brand)));
    let brand_preview = state.preview().unwrap();
    assert!(brand_preview.svg().contains("fill=\"#bd0f72\""));
    assert_eq!(brand_preview.diagnostics().foreground(), Foreground::Brand);
    assert_eq!(
        brand_preview.diagnostics().contrast_ratio(),
        Some(ContrastRatio::from_hundredths(604))
    );
    assert_eq!(
        brand_preview.diagnostics().selected_version(),
        original.selected_version()
    );
    assert_eq!(brand_preview.diagnostics().mask(), original.mask());

    let transparent = state
        .select_background(Background::Transparent)
        .expect("revision is available");
    assert!(state.complete_preview(transparent.revision(), evaluate_preview(&transparent),));
    let transparent_preview = state.preview().unwrap();
    assert!(!transparent_preview.svg().contains("<rect"));
    assert_eq!(
        transparent_preview.diagnostics().safety(),
        OutputSafety::Caution
    );
    assert_eq!(transparent_preview.diagnostics().contrast_ratio(), None);
    assert_eq!(
        state.caution().as_deref(),
        Some(
            "Transparent output has unknown effective contrast. Check it on white, light-gray, dark, and patterned placement surfaces."
        )
    );
    assert!(state.exports_enabled());
}

#[test]
fn ready_preview_exposes_safe_artifacts_complete_diagnostics_and_accessible_text() {
    let sensitive = "private!";
    let mut state = state_without_logo(ProfileId::Inline);
    let request = state
        .set_payload(sensitive.to_owned())
        .expect("revision is available");
    assert_eq!(
        state.export_disabled_reason().as_deref(),
        Some("QR preview is updating.")
    );
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));

    let preview = state.preview().expect("valid payload has a preview");
    let svg = preview.artifact(ArtifactKind::Svg);
    assert_eq!(svg.filename(), "qr-code.svg");
    assert_eq!(svg.mime_type(), "image/svg+xml");
    assert_eq!(svg.bytes(), preview.svg().as_bytes());
    let png = preview.artifact(ArtifactKind::Png);
    assert_eq!(png.filename(), "qr-code.png");
    assert_eq!(png.mime_type(), "image/png");
    assert!(png.bytes().starts_with(b"\x89PNG\r\n\x1a\n"));

    let diagnostics = preview.diagnostics();
    assert_eq!(diagnostics.mode(), DataMode::Byte);
    assert!(diagnostics.mask().number() <= 7);
    assert_eq!(diagnostics.quiet_zone_modules(), 4);
    assert_eq!(diagnostics.module_scale(), 10);
    assert_eq!(diagnostics.rendered_symbol_side_pixels(), 290);
    assert_eq!(diagnostics.outer_padding_per_side(), 5);
    assert_eq!(state.export_disabled_reason(), None);

    let label = preview.accessible_label();
    assert_eq!(
        label,
        "Generated QR code preview: Byte mode, version 1, ECC M."
    );
    assert!(!label.contains(sensitive));
    assert!(!svg.filename().contains(sensitive));
    assert!(!png.filename().contains(sensitive));
    assert!(!preview.svg().contains(sensitive));
}

#[test]
fn every_profile_derives_its_limit_dimensions_and_guidance() {
    let cases = [
        (ProfileId::Inline, 6, 100, 300, false),
        (ProfileId::Content, 8, 120, 360, false),
        (ProfileId::Landing, 12, 150, 450, false),
        (ProfileId::Print, 13, 160, 480, true),
        (ProfileId::Adaptive, 40, 116, 174, false),
    ];

    for (profile_id, maximum_version, svg_side, png_side, is_print) in cases {
        let mut state = state_without_logo(profile_id);
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
fn adaptive_preserves_the_long_url_and_exports_version_ten_at_ecc_h() {
    let payload = "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya";
    let mut state = WorkflowState::new(ProfileId::Adaptive);
    let request = state
        .set_payload(payload.to_owned())
        .expect("revision is available");

    assert_eq!(request.payload(), payload);
    assert_eq!(request.ecc(), ErrorCorrection::High);
    assert_eq!(request.minimum_version().number(), 6);
    let first = evaluate_preview(&request).expect("adaptive URL renders");
    let second = evaluate_preview(&request).expect("repeated adaptive URL renders");
    assert_eq!(first.svg(), second.svg());
    assert_eq!(
        first.artifact(ArtifactKind::Png).bytes(),
        second.artifact(ArtifactKind::Png).bytes(),
    );
    assert!(state.complete_preview(request.revision(), Ok(first)));

    let preview = state.preview().expect("adaptive URL fits");
    let diagnostics = preview.diagnostics();
    assert_eq!(diagnostics.selected_version().number(), 10);
    assert_eq!(diagnostics.maximum_version().number(), 40);
    assert!(!diagnostics.branding_increased_version());
    assert_eq!(diagnostics.svg_side_pixels(), 260);
    assert_eq!(diagnostics.png_side_pixels(), 390);
    assert_eq!(diagnostics.module_scale(), 6);
    assert_eq!(diagnostics.rendered_symbol_side_pixels(), 390);
    assert_eq!(diagnostics.outer_padding_per_side(), 0);
    let placement = diagnostics
        .logo_placement()
        .expect("adaptive logo placement");
    assert_eq!(placement.source_bounds().left_ten_thousandths(), 220_000);
    assert_eq!(placement.source_bounds().top_ten_thousandths(), 200_625);
    assert_eq!(placement.knockout_bounds().left().get(), 21);
    assert_eq!(placement.knockout_bounds().top().get(), 19);
    assert!(state.exports_enabled());
}

#[test]
fn adaptive_grows_past_version_ten_for_a_long_branded_url() {
    let payload = format!("https://example.test/{}", "a".repeat(105));
    let mut state = WorkflowState::new(ProfileId::Adaptive);
    let request = state
        .set_payload(payload.clone())
        .expect("revision is available");

    let preview = evaluate_preview(&request).expect("Version 11 branded URL renders");
    let diagnostics = preview.diagnostics();
    assert_eq!(request.payload(), payload);
    assert_eq!(diagnostics.ecc(), ErrorCorrection::High);
    assert_eq!(diagnostics.selected_version().number(), 11);
    assert_eq!(diagnostics.maximum_version().number(), 40);
    assert_eq!(diagnostics.svg_side_pixels(), 276);
    assert_eq!(diagnostics.png_side_pixels(), 414);
    assert_eq!(diagnostics.module_scale(), 6);
    assert!(diagnostics.logo_placement().is_some());
    assert!(state.complete_preview(request.revision(), Ok(preview)));
    assert!(state.exports_enabled());
}

#[test]
fn adaptive_url_crosses_the_exact_version_ten_byte_boundary() {
    let prefix = "https://example.test/";
    let exact_payload = format!("{prefix}{}", "a".repeat(98));
    let one_over_payload = format!("{prefix}{}", "a".repeat(99));

    let exact = evaluate_preview(
        &WorkflowState::new(ProfileId::Adaptive)
            .set_payload(exact_payload)
            .expect("revision is available"),
    )
    .expect("119-byte URL fits Version 10-H exactly");
    let one_over = evaluate_preview(
        &WorkflowState::new(ProfileId::Adaptive)
            .set_payload(one_over_payload)
            .expect("revision is available"),
    )
    .expect("120-byte URL advances to Version 11-H");

    assert_eq!(exact.diagnostics().selected_version().number(), 10);
    assert_eq!(exact.diagnostics().used_data_bits(), 972);
    assert_eq!(exact.diagnostics().available_data_bits(), 976);
    assert_eq!(one_over.diagnostics().selected_version().number(), 11);
}

#[test]
fn adaptive_rejects_unreviewed_higher_version_branding_without_losing_unbranded_output() {
    let payload = format!("https://example.test/{}", "a".repeat(120));
    let mut state = WorkflowState::new(ProfileId::Adaptive);
    let branded = state
        .set_payload(payload.clone())
        .expect("revision is available");

    assert_eq!(branded.ecc(), ErrorCorrection::High);
    let result = evaluate_preview(&branded);
    assert_eq!(
        result,
        Err(WorkflowFailure::UnsafeLogoGeometry {
            adaptive_recommended: false,
        })
    );
    assert!(state.complete_preview(branded.revision(), result));
    assert_eq!(
        state.validation_message().as_deref(),
        Some(
            "Adaptive logo placement is approved only through QR Version 11; disable the logo to keep this exact payload."
        ),
    );

    let unbranded = state
        .set_logo_enabled(false)
        .expect("revision is available");
    let preview = evaluate_preview(&unbranded).expect("unbranded Adaptive remains available");
    assert_eq!(unbranded.payload(), payload);
    assert_eq!(preview.diagnostics().ecc(), ErrorCorrection::Medium);
    assert!(preview.diagnostics().selected_version().number() <= 40);
}

#[test]
fn adaptive_supports_the_version_forty_byte_boundary_without_a_logo() {
    let mut state = state_without_logo(ProfileId::Adaptive);
    let exact = state
        .set_payload("a".repeat(2_331))
        .expect("revision is available");
    let preview = evaluate_preview(&exact).expect("Version 40 ECC-M byte boundary fits");
    let diagnostics = preview.diagnostics();
    assert_eq!(diagnostics.ecc(), ErrorCorrection::Medium);
    assert_eq!(diagnostics.selected_version().number(), 40);
    assert_eq!(diagnostics.maximum_version().number(), 40);
    assert_eq!(diagnostics.svg_side_pixels(), 740);
    assert_eq!(diagnostics.png_side_pixels(), 1_110);
    assert_eq!(diagnostics.module_scale(), 6);

    let one_over = state
        .set_payload("a".repeat(2_332))
        .expect("revision is available");
    assert_eq!(
        evaluate_preview(&one_over),
        Err(WorkflowFailure::OverCapacity {
            maximum_version: qr_core::Version::new(40).unwrap(),
            adaptive_recommended: false,
        })
    );
}

#[test]
fn profile_changes_refit_without_changing_safe_ecc() {
    let mut state = state_without_logo(ProfileId::Content);
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
    assert!(state.exports_enabled());
    assert_eq!(
        state
            .preview()
            .expect("Inline now admits Version 6")
            .diagnostics()
            .selected_version()
            .number(),
        6,
    );

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
    let mut state = state_without_logo(ProfileId::Print);
    let safe_request = state
        .set_payload("a".repeat(30))
        .expect("revision is available");
    assert!(state.complete_preview(safe_request.revision(), evaluate_preview(&safe_request),));
    assert_eq!(
        state
            .preview()
            .expect("safe preview fits")
            .diagnostics()
            .selected_version()
            .number(),
        3,
    );

    let logo_request = state.set_logo_enabled(true).expect("revision is available");
    assert_eq!(logo_request.ecc(), ErrorCorrection::High);
    assert_eq!(logo_request.minimum_version().number(), 6);
    assert!(state.complete_preview(logo_request.revision(), evaluate_preview(&logo_request),));
    let logo_diagnostics = state.preview().expect("logo preview fits").diagnostics();
    assert_eq!(logo_diagnostics.ecc(), ErrorCorrection::High);
    assert_eq!(logo_diagnostics.minimum_version().number(), 6);
    assert_eq!(logo_diagnostics.selected_version().number(), 6);
    assert!(logo_diagnostics.branding_increased_version());
    assert_eq!(logo_diagnostics.used_data_bits(), 252);
    assert_eq!(logo_diagnostics.available_data_bits(), 480);
    assert_eq!(logo_diagnostics.data_codewords(), 60);
    assert_eq!(logo_diagnostics.matrix_modules(), 41);
    assert_eq!(logo_diagnostics.logo_style(), qr_render::LogoStyle::Bundled);
    let placement = logo_diagnostics
        .logo_placement()
        .expect("version 6 has reviewed logo geometry");
    assert_eq!(placement.obscured_data_modules(), 105);
    assert_eq!(placement.obscured_remainder_modules(), 0);
    assert_eq!(placement.protected_clearance(), 6);
    assert_eq!(logo_diagnostics.safety(), OutputSafety::Caution);
    assert!(
        state
            .preview()
            .unwrap()
            .svg()
            .contains("data-role=\"bundled-logo\"")
    );
    assert!(state.caution().unwrap().contains("bundled logo obscures"));
    assert!(state.exports_enabled());

    let restored_request = state
        .set_logo_enabled(false)
        .expect("revision is available");
    assert_eq!(restored_request.ecc(), ErrorCorrection::Medium);
    assert_eq!(restored_request.minimum_version().number(), 1);
    assert!(state.complete_preview(
        restored_request.revision(),
        evaluate_preview(&restored_request),
    ));
    let restored = state
        .preview()
        .expect("safe preview is restored")
        .diagnostics();
    assert_eq!(restored.ecc(), ErrorCorrection::Medium);
    assert_eq!(restored.selected_version().number(), 3);
    assert!(!restored.branding_increased_version());
}

#[test]
fn inline_logo_mode_admits_the_branded_minimum_and_enables_exports() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    let request = state
        .set_payload("small payload".to_owned())
        .expect("revision is available");

    assert_eq!(request.ecc(), ErrorCorrection::High);
    assert_eq!(request.minimum_version().number(), 6);
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));
    assert_eq!(state.validation_message(), None);
    let diagnostics = state
        .preview()
        .expect("Inline logo preview is valid")
        .diagnostics();
    assert_eq!(diagnostics.selected_version().number(), 6);
    assert_eq!(diagnostics.maximum_version().number(), 6);
    assert_eq!(diagnostics.svg_side_pixels(), 100);
    assert_eq!(diagnostics.png_side_pixels(), 300);
    assert_eq!(diagnostics.module_scale(), 6);
    assert_eq!(diagnostics.outer_padding_per_side(), 3);
    assert!(diagnostics.logo_placement().is_some());
    assert!(state.exports_enabled());
}

#[test]
fn logo_mode_rejects_a_naturally_larger_version_without_reviewed_centered_geometry() {
    let mut state = WorkflowState::new(ProfileId::Print);
    let request = state
        .set_payload("a".repeat(59))
        .expect("revision is available");

    assert_eq!(request.ecc(), ErrorCorrection::High);
    assert_eq!(request.minimum_version().number(), 6);
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));
    assert_eq!(
        state.validation_message().as_deref(),
        Some(
            "Logo mode is unavailable because no safe placement exists for this QR version. Try Adaptive for long payloads and version-aware logo placement."
        ),
    );
    assert!(!state.exports_enabled());
}

#[test]
fn invalid_and_internal_results_have_associated_messages_and_disable_exports() {
    let mut state = state_without_logo(ProfileId::Inline);

    let empty = state
        .set_payload(String::new())
        .expect("revision is available");
    assert!(state.complete_preview(empty.revision(), evaluate_preview(&empty)));
    assert_eq!(
        state.validation_message().as_deref(),
        Some("Enter text to generate a QR code."),
    );
    assert!(!state.exports_enabled());

    state
        .set_payload("valid".to_owned())
        .expect("revision is available");
    let unsafe_contrast = state
        .select_background(Background::Opaque(Rgba::opaque(116, 116, 116)))
        .expect("revision is available");
    assert!(state.complete_preview(
        unsafe_contrast.revision(),
        evaluate_preview(&unsafe_contrast),
    ));
    assert_eq!(
        state.validation_message().as_deref(),
        Some("Opaque QR contrast is 1.29:1; at least 4.50:1 is required."),
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
        .set_payload("x".repeat(107))
        .expect("revision is available");
    assert!(state.complete_preview(over_capacity.revision(), evaluate_preview(&over_capacity),));
    assert_eq!(
        state.validation_message().as_deref(),
        Some(
            "The payload does not fit this profile's maximum QR version 6. Try Adaptive for long payloads and version-aware logo placement."
        ),
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
    let mut state = state_without_logo(ProfileId::Inline);
    let request = state
        .set_payload("line one\nline two\t".to_owned())
        .expect("revision is available");
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));

    assert_eq!(state.payload(), "line one\nline two\t");
    assert_eq!(
        state.caution().as_deref(),
        Some("This payload contains control characters. Confirm that they are intentional."),
    );
    assert!(state.exports_enabled());
}

#[test]
fn stale_preview_results_cannot_replace_the_latest_value() {
    let mut state = state_without_logo(ProfileId::Print);
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
