#![no_main]

use std::path::Path;

use fixture_tool::FixtureManifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = FixtureManifest::parse_and_verify(data, Path::new("."));
});
