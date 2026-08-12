use qr_render::ProfileId;
use qr_web::worker_protocol::{WorkerRequest, WorkerResponse};
use qr_web::workflow::{ArtifactKind, WorkflowState, evaluate_preview};

#[test]
fn worker_protocol_preserves_revision_payload_diagnostics_and_artifact_bytes() {
    let mut state = WorkflowState::new(ProfileId::Adaptive);
    state
        .set_logo_enabled(false)
        .expect("Adaptive supports unbranded output");
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
        (ProfileId::Inline, true, "BRANDED-123".to_owned()),
        (ProfileId::Content, false, "café 世界".to_owned()),
        (
            ProfileId::Adaptive,
            false,
            "HELLOworld1234567890".to_owned(),
        ),
        (ProfileId::Adaptive, false, "a".repeat(2_331)),
    ];

    for (profile, logo_enabled, payload) in cases {
        let mut state = WorkflowState::new(profile);
        state
            .set_logo_enabled(logo_enabled)
            .expect("logo transition produces a revision");
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
