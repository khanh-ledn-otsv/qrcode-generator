#![cfg(target_arch = "wasm32")]

use js_sys::{ArrayBuffer, Function, Promise, Reflect, Uint8Array};
use qr_render::ProfileId;
use qr_web::download::{ObjectUrl, create_blob};
use qr_web::workflow::{ArtifactKind, WorkflowFailure, WorkflowState, evaluate_preview};
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
    let mut state = WorkflowState::new(ProfileId::Inline);
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
    assert_eq!(diagnostics.maximum_version().number(), 6);
    assert_eq!(diagnostics.svg_side_pixels(), 100);
    assert_eq!(diagnostics.png_side_pixels(), 300);
    assert!(diagnostics.branding_increased_version());
    let placement = diagnostics
        .logo_placement()
        .expect("version 6 has reviewed logo geometry");
    assert_eq!(placement.source_bounds().left_ten_thousandths(), 140_000);
    assert_eq!(placement.source_bounds().top_ten_thousandths(), 180_625);
    assert_eq!(placement.source_bounds().width_ten_thousandths(), 130_000);
    assert_eq!(placement.source_bounds().height_ten_thousandths(), 48_750);
    assert_eq!(placement.obscured_data_modules(), 105);
    assert_eq!(placement.obscured_remainder_modules(), 0);
    assert!(state.exports_enabled());
}

#[wasm_bindgen_test]
fn adaptive_long_url_selects_version_ten_and_exports_on_wasm() {
    let payload = "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya";
    let mut state = WorkflowState::new(ProfileId::Adaptive);
    let request = state
        .set_payload(payload.to_owned())
        .expect("revision is available");
    assert_eq!(request.payload(), payload);
    let first = evaluate_preview(&request).expect("adaptive URL renders");
    let second = evaluate_preview(&request).expect("repeated adaptive URL renders");
    assert_eq!(first.svg(), second.svg());
    assert_eq!(
        first.artifact(ArtifactKind::Png).bytes(),
        second.artifact(ArtifactKind::Png).bytes(),
    );
    assert!(state.complete_preview(request.revision(), Ok(first)));

    let diagnostics = state
        .preview()
        .expect("adaptive preview is ready")
        .diagnostics();
    assert_eq!(diagnostics.selected_version().number(), 10);
    assert_eq!(diagnostics.maximum_version().number(), 40);
    assert_eq!(diagnostics.svg_side_pixels(), 260);
    assert_eq!(diagnostics.png_side_pixels(), 390);
    assert_eq!(diagnostics.module_scale(), 6);
    assert_eq!(diagnostics.rendered_symbol_side_pixels(), 390);
    assert_eq!(diagnostics.outer_padding_per_side(), 0);
    let placement = diagnostics.logo_placement().expect("adaptive placement");
    assert_eq!(placement.source_bounds().left_ten_thousandths(), 220_000);
    assert_eq!(placement.source_bounds().top_ten_thousandths(), 200_625);
    assert_eq!(placement.knockout_bounds().left().get(), 21);
    assert_eq!(placement.knockout_bounds().top().get(), 19);
    assert!(state.exports_enabled());
}

#[wasm_bindgen_test]
fn adaptive_version_forty_boundary_is_deterministic_on_wasm() {
    let mut state = WorkflowState::new(ProfileId::Adaptive);
    state
        .set_logo_enabled(false)
        .expect("Adaptive can disable the logo");
    let exact = state
        .set_payload("a".repeat(2_331))
        .expect("revision is available");
    let first = evaluate_preview(&exact).expect("Version 40 boundary renders");
    let second = evaluate_preview(&exact).expect("repeated Version 40 boundary renders");
    assert_eq!(first.svg(), second.svg());
    assert_eq!(
        first.artifact(ArtifactKind::Png).bytes(),
        second.artifact(ArtifactKind::Png).bytes()
    );
    assert_eq!(first.diagnostics().selected_version().number(), 40);
    assert_eq!(first.diagnostics().svg_side_pixels(), 740);
    assert_eq!(first.diagnostics().png_side_pixels(), 1_110);
    assert_eq!(first.diagnostics().module_scale(), 6);

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
