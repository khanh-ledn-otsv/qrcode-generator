use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u8,
    decoder: Decoder,
    sample: Sample,
    dots: DotPolicy,
    logo: LogoPolicy,
    quiet_zone_modules: u8,
    decorative_export_borders: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decoder {
    tool: String,
    version: String,
    source_commit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    profiles: usize,
    payload_classes: Vec<String>,
    backgrounds: Vec<String>,
    artifact_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DotPolicy {
    candidate_diameters_thousandths: Vec<u16>,
    function_treatments: Vec<String>,
    outcomes: Vec<DotOutcome>,
    selected_diameter_thousandths: u16,
    selected_function_treatment: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DotOutcome {
    diameter_thousandths: u16,
    function_treatment: String,
    attempted: usize,
    decoded: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogoPolicy {
    ecc: String,
    exact_matrix_centering: bool,
    opaque_white_knockout: bool,
    candidate_minimum_versions: Vec<u8>,
    minimum_version_evaluation: Vec<MinimumVersionRow>,
    selected_version_width_outcomes: Vec<LogoWidthOutcome>,
    profile_outcomes: Vec<LogoProfileOutcome>,
    selected_minimum_version: u8,
    selected_sizes: Vec<LogoRow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogoProfileOutcome {
    profile: String,
    attempted: usize,
    decoded: usize,
    outcome: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogoWidthOutcome {
    version: u8,
    source_width_thousandths: u32,
    attempted: usize,
    decoded: usize,
    outcome: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MinimumVersionRow {
    version: u8,
    maximum_safe_source_width_thousandths: u32,
    source_left_ten_thousandths: u32,
    source_top_ten_thousandths: u32,
    source_height_ten_thousandths: u32,
    knockout: [u32; 4],
    protected_clearance_modules: u32,
    obscured_data_modules: u32,
    obscured_remainder_modules: u32,
    requested_minimum_source_width_thousandths: u32,
    meets_requested_hierarchy: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogoRow {
    version: u8,
    source_left_ten_thousandths: Option<u32>,
    source_top_ten_thousandths: Option<u32>,
    source_width_ten_thousandths: Option<u32>,
    source_height_ten_thousandths: Option<u32>,
    knockout: Option<[u32; 4]>,
    protected_clearance_modules: Option<u32>,
    obscured_data_modules: Option<u32>,
    obscured_remainder_modules: Option<u32>,
    outcome: String,
}

#[test]
fn committed_policy_is_complete_and_selects_only_full_decode_passes() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/generated/branded-geometry-policy.json");
    let bytes = fs::read(path).expect("branded geometry policy must be committed");
    let policy: Policy = serde_json::from_slice(&bytes).expect("policy JSON must be strict");

    assert_eq!(policy.schema_version, 1);
    assert_eq!(policy.decoder.tool, "ZXing-C++ ZXingReader");
    assert_eq!(policy.decoder.version, "3.0.2");
    assert_eq!(
        policy.decoder.source_commit,
        "8dd1cf5c4fd6fb6211bb96713db926ac6f2cf825"
    );
    assert_eq!(policy.sample.profiles, 4);
    assert_eq!(
        policy.sample.payload_classes,
        [
            "short-url",
            "dense-url",
            "numeric",
            "alphanumeric",
            "ascii-byte",
            "utf8-eci26",
        ]
    );
    assert_eq!(policy.sample.backgrounds, ["opaque-white", "transparent"]);
    assert_eq!(
        policy.sample.artifact_paths,
        ["native-png", "rasterized-svg"]
    );

    assert_eq!(
        policy.dots.candidate_diameters_thousandths,
        (450..=600).step_by(10).collect::<Vec<_>>()
    );
    assert_eq!(
        policy.dots.function_treatments,
        ["square-functions", "non-finder-dots"]
    );
    assert_eq!(policy.dots.outcomes.len(), 32);
    for diameter in &policy.dots.candidate_diameters_thousandths {
        for treatment in &policy.dots.function_treatments {
            let outcome = policy
                .dots
                .outcomes
                .iter()
                .find(|outcome| {
                    outcome.diameter_thousandths == *diameter
                        && outcome.function_treatment == *treatment
                })
                .expect("every dot candidate/treatment pair must be recorded");
            assert!(outcome.attempted > 0);
            assert!(outcome.decoded <= outcome.attempted);
        }
    }
    let selected = policy
        .dots
        .outcomes
        .iter()
        .find(|outcome| {
            outcome.diameter_thousandths == policy.dots.selected_diameter_thousandths
                && outcome.function_treatment == policy.dots.selected_function_treatment
        })
        .expect("selected dot policy must be present");
    assert_eq!(selected.decoded, selected.attempted);
    assert_eq!(policy.dots.selected_diameter_thousandths, 450);
    assert_eq!(policy.dots.selected_function_treatment, "non-finder-dots");
    assert!(!policy.dots.outcomes.iter().any(|outcome| {
        outcome.function_treatment == policy.dots.selected_function_treatment
            && outcome.diameter_thousandths < policy.dots.selected_diameter_thousandths
            && outcome.decoded == outcome.attempted
    }));

    assert_eq!(policy.logo.ecc, "H");
    assert!(policy.logo.exact_matrix_centering);
    assert!(policy.logo.opaque_white_knockout);
    assert!(
        policy
            .logo
            .candidate_minimum_versions
            .contains(&policy.logo.selected_minimum_version)
    );
    assert_eq!(policy.logo.minimum_version_evaluation.len(), 3);
    for row in &policy.logo.minimum_version_evaluation {
        assert_eq!(
            row.meets_requested_hierarchy,
            row.maximum_safe_source_width_thousandths
                >= row.requested_minimum_source_width_thousandths
        );
        let matrix_units = u32::from(17 + row.version * 4) * 10_000;
        assert_eq!(
            row.source_left_ten_thousandths * 2 + row.maximum_safe_source_width_thousandths * 10,
            matrix_units
        );
        assert_eq!(
            row.source_top_ten_thousandths * 2 + row.source_height_ten_thousandths,
            matrix_units
        );
        assert!(row.knockout[2] > 0 && row.knockout[3] > 0);
        assert!(row.protected_clearance_modules > 0);
        assert!(row.obscured_data_modules > 0);
        assert_eq!(row.obscured_remainder_modules, 0);
    }
    assert_eq!(
        policy
            .logo
            .minimum_version_evaluation
            .iter()
            .find(|row| row.meets_requested_hierarchy)
            .map(|row| row.version),
        Some(policy.logo.selected_minimum_version)
    );
    assert_eq!(policy.logo.selected_version_width_outcomes.len(), 9);
    assert_eq!(policy.logo.profile_outcomes.len(), 4);
    assert!(policy.logo.profile_outcomes.iter().any(|outcome| {
        outcome.profile == "Inline"
            && outcome.attempted == 0
            && outcome.decoded == 0
            && outcome.outcome == "minimum-version-exceeds-profile-ceiling"
    }));
    assert_eq!(
        policy
            .logo
            .profile_outcomes
            .iter()
            .filter(|outcome| outcome.outcome == "decoded")
            .map(|outcome| {
                assert_eq!(outcome.decoded, outcome.attempted);
                outcome.decoded
            })
            .sum::<usize>(),
        36
    );
    let selected_width = policy
        .logo
        .selected_sizes
        .iter()
        .find(|row| row.version == policy.logo.selected_minimum_version)
        .and_then(|row| row.source_width_ten_thousandths)
        .map(|width| width / 10)
        .expect("selected version must have a logo width");
    for outcome in &policy.logo.selected_version_width_outcomes {
        assert_eq!(outcome.version, policy.logo.selected_minimum_version);
        match outcome.outcome.as_str() {
            "decoded" => {
                assert!(outcome.attempted > 0);
                assert_eq!(outcome.decoded, outcome.attempted);
            }
            "unsafe-geometry" => {
                assert_eq!(outcome.attempted, 0);
                assert_eq!(outcome.decoded, 0);
            }
            unexpected => panic!("unexpected logo width outcome {unexpected}"),
        }
    }
    assert_eq!(
        policy
            .logo
            .selected_version_width_outcomes
            .iter()
            .filter(|outcome| outcome.outcome == "decoded")
            .map(|outcome| outcome.source_width_thousandths)
            .max(),
        Some(selected_width)
    );
    assert_eq!(policy.logo.selected_sizes.len(), 13);
    for (index, row) in policy.logo.selected_sizes.iter().enumerate() {
        assert_eq!(usize::from(row.version), index + 1);
        match row.outcome.as_str() {
            "decoded" => {
                assert!(row.version >= policy.logo.selected_minimum_version);
                let left = row.source_left_ten_thousandths.unwrap();
                let top = row.source_top_ten_thousandths.unwrap();
                let width = row.source_width_ten_thousandths.unwrap();
                let height = row.source_height_ten_thousandths.unwrap();
                let matrix_units = u32::from(17 + row.version * 4) * 10_000;
                assert_eq!(left * 2 + width, matrix_units);
                assert_eq!(top * 2 + height, matrix_units);
                assert!(row.knockout.is_some());
                assert!(row.protected_clearance_modules.is_some());
                assert!(row.obscured_data_modules.is_some());
                assert!(row.obscured_remainder_modules.is_some());
            }
            "below-branded-minimum" | "unsafe-protected-module-intersection" => {
                assert!(row.source_left_ten_thousandths.is_none());
                assert!(row.source_top_ten_thousandths.is_none());
                assert!(row.source_width_ten_thousandths.is_none());
                assert!(row.source_height_ten_thousandths.is_none());
                assert!(row.knockout.is_none());
                assert!(row.protected_clearance_modules.is_none());
                assert!(row.obscured_data_modules.is_none());
                assert!(row.obscured_remainder_modules.is_none());
            }
            unexpected => panic!("unexpected logo outcome {unexpected}"),
        }
    }
    assert_eq!(policy.quiet_zone_modules, 4);
    assert!(!policy.decorative_export_borders);
}
