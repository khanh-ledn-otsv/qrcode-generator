#![cfg(target_arch = "wasm32")]

use qr_web::wasm_api::{capacity_limit, generate_preview};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn adapter_returns_binary_artifacts_and_raw_diagnostics() {
    let preview =
        generate_preview("browser logo", "small", "magenta", true).expect("valid request renders");

    assert!(preview.svg().starts_with("<svg"));
    assert!(preview.png().starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(preview.ecc(), 3);
    assert_eq!(preview.selected_version(), 6);
    assert_eq!(preview.maximum_version(), 6);
    assert!(preview.rendered_logo());
    assert_eq!(preview.obscured_data_modules(), 105);
}

#[wasm_bindgen_test]
fn adapter_is_deterministic_and_reports_capacity_errors() {
    let payload = "a".repeat(capacity_limit("poster-package", false));
    let first = generate_preview(&payload, "poster-package", "black", false).unwrap();
    let second = generate_preview(&payload, "poster-package", "black", false).unwrap();

    assert_eq!(first.svg(), second.svg());
    assert_eq!(first.png(), second.png());
    assert!(generate_preview(&(payload + "a"), "poster-package", "black", false).is_err());
}
