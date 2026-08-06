#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

#[path = "support/png_fixture.rs"]
mod png_fixture;

#[wasm_bindgen_test]
fn png_bytes_match_the_native_artifact_fixture() {
    let first = png_fixture::artifact();
    let second = png_fixture::artifact();

    assert_eq!(first, second);
    assert_eq!(png_fixture::sha256_hex(&first), png_fixture::SHA256);
}
