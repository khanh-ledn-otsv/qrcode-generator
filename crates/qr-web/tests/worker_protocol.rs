use qr_render::{ForegroundTheme, ProfileId};
use qr_web::worker_protocol::{WorkerRequest, WorkerResponse};
use qr_web::workflow::{ArtifactKind, WorkflowState, evaluate_preview};

#[test]
fn worker_protocol_preserves_revision_payload_diagnostics_and_artifact_bytes() {
    let mut state = WorkflowState::new(ProfileId::PosterPackage);
    state
        .set_logo_enabled(false)
        .expect("fixed output supports unbranded output");
    let preview_request = state
        .set_payload("MIXED-1234-lowercase-世界".to_owned())
        .expect("revision is available");
    let expected = evaluate_preview(&preview_request).expect("representative request renders");

    let encoded_request = WorkerRequest::from_preview(&preview_request)
        .to_json()
        .expect("request serializes");
    assert!(encoded_request.contains("MIXED-1234-lowercase-世界"));

    let response =
        WorkerResponse::evaluate_json(&encoded_request).expect("worker evaluates a valid request");
    let (metadata, png) = response
        .into_message_parts()
        .expect("response separates transferable PNG bytes");
    let (revision, actual) = WorkerResponse::from_message_parts(&metadata, png)
        .expect("response deserializes")
        .into_preview_result();
    let actual = actual.expect("worker result succeeds");

    assert_eq!(revision, preview_request.revision());
    assert_eq!(actual.diagnostics(), expected.diagnostics());
    assert_eq!(actual.svg(), expected.svg());
    assert_eq!(
        actual.artifact(ArtifactKind::Png).bytes(),
        expected.artifact(ArtifactKind::Png).bytes(),
    );
}

#[test]
fn worker_artifacts_match_direct_evaluation_across_approved_request_shapes() {
    let cases = [
        (ProfileId::Small, true, "BRANDED-123".to_owned()),
        (ProfileId::Standard, false, "café 世界".to_owned()),
        (
            ProfileId::PosterPackage,
            false,
            "HELLOworld1234567890".to_owned(),
        ),
    ];

    for (profile, logo_enabled, payload) in cases {
        let mut state = WorkflowState::new(profile);
        state
            .set_logo_enabled(logo_enabled)
            .expect("logo transition produces a revision");
        if profile == ProfileId::Standard {
            state
                .set_foreground_theme(ForegroundTheme::Black)
                .expect("theme transition produces a revision");
        }
        let request = state
            .set_payload(payload)
            .expect("payload transition produces a revision");
        let expected = evaluate_preview(&request).expect("approved request renders directly");
        let request_json = WorkerRequest::from_preview(&request)
            .to_json()
            .expect("request serializes");
        let (metadata, png) = WorkerResponse::evaluate_json(&request_json)
            .expect("worker evaluates request")
            .into_message_parts()
            .expect("worker response separates bytes");
        let (_, actual) = WorkerResponse::from_message_parts(&metadata, png)
            .expect("worker response reconstructs")
            .into_preview_result();
        let actual = actual.expect("worker result succeeds");

        assert_eq!(actual.diagnostics(), expected.diagnostics());
        assert_eq!(actual.svg(), expected.svg());
        assert_eq!(
            actual.artifact(ArtifactKind::Png).bytes(),
            expected.artifact(ArtifactKind::Png).bytes(),
        );
    }
}

#[test]
fn malformed_ready_responses_never_become_exportable_previews() {
    let mut state = WorkflowState::new(ProfileId::Small);
    let request = state
        .set_payload("valid payload".to_owned())
        .expect("payload starts a preview");
    let request_json = WorkerRequest::from_preview(&request)
        .to_json()
        .expect("request serializes");
    let (metadata, png) = WorkerResponse::evaluate_json(&request_json)
        .expect("worker evaluates request")
        .into_message_parts()
        .expect("response separates bytes");

    assert!(WorkerResponse::from_message_parts(&metadata, Vec::new()).is_err());
    assert!(WorkerResponse::from_message_parts(&metadata, b"\x89PNG\r\n\x1a\n".to_vec(),).is_err());

    let mut invalid_svg: serde_json::Value =
        serde_json::from_str(&metadata).expect("metadata is JSON");
    invalid_svg["result"]["value"]["svg"] = "<svg></svg>".into();
    let invalid_svg = serde_json::to_string(&invalid_svg).expect("JSON serializes");
    assert!(WorkerResponse::from_message_parts(&invalid_svg, png.clone()).is_err());

    let mut invalid_enum: serde_json::Value =
        serde_json::from_str(&metadata).expect("metadata is JSON");
    invalid_enum["result"]["value"]["diagnostics"]["safety"] = 2.into();
    let invalid_enum = serde_json::to_string(&invalid_enum).expect("JSON serializes");
    assert!(WorkerResponse::from_message_parts(&invalid_enum, png.clone()).is_err());

    let mut invalid_geometry: serde_json::Value =
        serde_json::from_str(&metadata).expect("metadata is JSON");
    invalid_geometry["result"]["value"]["diagnostics"]["matrix_modules"] = 1.into();
    let invalid_geometry = serde_json::to_string(&invalid_geometry).expect("JSON serializes");
    assert!(WorkerResponse::from_message_parts(&invalid_geometry, png).is_err());
}
