#![cfg(target_arch = "wasm32")]

use js_sys::{ArrayBuffer, Function, Promise, Reflect, Uint8Array};
use qr_render::ProfileId;
use qr_web::download::{ObjectUrl, create_blob};
use qr_web::workflow::{ArtifactKind, WorkflowState, evaluate_preview};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::Response;

#[wasm_bindgen_test(async)]
async fn blob_has_exact_artifact_bytes_mime_type_and_revocable_url() {
    let mut state = WorkflowState::new(ProfileId::Inline);
    state
        .set_logo_enabled(false)
        .expect("Inline can use ordinary no-logo fitting");
    let request = state
        .set_payload("browser artifact".to_owned())
        .expect("revision is available");
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));
    let preview = state.preview().expect("valid payload has artifacts");
    let artifact = preview.artifact(ArtifactKind::Png);

    let blob = create_blob(artifact).expect("valid bytes create a Blob");
    assert_eq!(blob.type_(), "image/png");
    assert_eq!(blob.size(), artifact.bytes().len() as f64);

    let object_url = ObjectUrl::new(&blob).expect("browser creates an object URL");
    let url = object_url.as_str().to_owned();
    assert_eq!(read_url(&url).await, artifact.bytes());
    drop(object_url);
    assert!(fetch(&url).await.is_err(), "dropped URL must be revoked");
}

#[wasm_bindgen_test]
fn logo_mode_selects_the_branded_minimum_and_keeps_exports_available_on_wasm() {
    let mut state = WorkflowState::new(ProfileId::Content);
    let request = state
        .set_payload("browser logo".to_owned())
        .expect("revision is available");
    assert_eq!(request.minimum_version().number(), 6);
    assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));

    let diagnostics = state
        .preview()
        .expect("logo preview is ready")
        .diagnostics();
    assert_eq!(diagnostics.selected_version().number(), 6);
    assert!(diagnostics.branding_increased_version());
    assert!(state.exports_enabled());
}

#[wasm_bindgen_test(async)]
async fn repeated_object_urls_are_revoked_instead_of_retained() {
    let mut state = WorkflowState::new(ProfileId::Content);
    let mut revoked = Vec::new();
    for generation in 0..32 {
        let payload = format!("bounded generation {generation}");
        let request = state
            .set_payload(payload.clone())
            .expect("revision is available");
        assert!(state.complete_preview(request.revision(), evaluate_preview(&request)));
        assert_eq!(state.payload(), payload);
        let artifact = state
            .preview()
            .expect("valid payload has artifacts")
            .artifact(ArtifactKind::Svg);
        let blob = create_blob(artifact).expect("valid bytes create a Blob");
        let object_url = ObjectUrl::new(&blob).expect("browser creates an object URL");
        revoked.push(object_url.as_str().to_owned());
        drop(object_url);
    }
    for url in revoked {
        assert!(fetch(&url).await.is_err(), "every released URL is revoked");
    }
}

#[wasm_bindgen_test]
fn control_character_payload_renders_both_artifacts_on_wasm() {
    let mut state = WorkflowState::new(ProfileId::Content);
    let request = state
        .set_payload("line one\nline two".to_owned())
        .expect("revision is available");
    let preview = evaluate_preview(&request).expect("control characters remain renderable");

    assert!(
        preview
            .artifact(ArtifactKind::Svg)
            .bytes()
            .starts_with(b"<svg")
    );
    assert!(
        preview
            .artifact(ArtifactKind::Png)
            .bytes()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
}

async fn read_url(url: &str) -> Vec<u8> {
    let response = fetch(url).await.expect("owned URL is readable");
    let buffer = JsFuture::from(response.array_buffer().expect("response exposes bytes"))
        .await
        .expect("array buffer resolves")
        .unchecked_into::<ArrayBuffer>();
    Uint8Array::new(&buffer).to_vec()
}

async fn fetch(url: &str) -> Result<Response, wasm_bindgen::JsValue> {
    let global = js_sys::global();
    let fetch =
        Reflect::get(&global, &wasm_bindgen::JsValue::from_str("fetch"))?.dyn_into::<Function>()?;
    let promise: Promise = fetch
        .call1(&global, &wasm_bindgen::JsValue::from_str(url))?
        .dyn_into()?;
    JsFuture::from(promise)
        .await
        .map(|value| value.unchecked_into::<Response>())
}
