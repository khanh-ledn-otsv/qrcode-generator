#[path = "support/high_versions.rs"]
mod high_versions;

use qr_core::matrix::ModuleKind;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    Background, Foreground, LogoStyle, OutputSafety, ProfileId, RenderError, RenderModel,
    RenderOptions, Rgba, SUPPORTED_PROFILES,
};

const LONG_ONE_URL: &str = "https://www.one-line.com/en/news/notice-mandatory-advance-cargo-declaration-acd-reference-number-imports-kenya";

#[test]
fn version_six_uses_the_exact_decode_backed_centered_logo_placement() {
    let version_six = Version::new(6).unwrap();
    let encoded = encode(EncodeRequest::with_version_range(
        "logo",
        ErrorCorrection::High,
        version_six,
        version_six,
    ))
    .unwrap();
    let options = RenderOptions::safe(SUPPORTED_PROFILES[1])
        .unwrap()
        .with_logo(LogoStyle::Bundled)
        .unwrap();
    let placement = RenderModel::new(&encoded, options)
        .unwrap()
        .logo_placement()
        .unwrap();
    let source = placement.source_bounds();
    let knockout = placement.knockout_bounds();

    assert_eq!(source.left_ten_thousandths(), 140_000);
    assert_eq!(source.top_ten_thousandths(), 180_625);
    assert_eq!(source.width_ten_thousandths(), 130_000);
    assert_eq!(source.height_ten_thousandths(), 48_750);
    assert_eq!(
        (
            knockout.left().get(),
            knockout.top().get(),
            knockout.width().get(),
            knockout.height().get(),
        ),
        (13, 17, 15, 7),
    );
    assert_eq!(placement.protected_clearance(), 6);
    assert_eq!(placement.obscured_data_modules(), 105);
    assert_eq!(placement.obscured_remainder_modules(), 0);
}

#[test]
fn adaptive_branded_version_ten_uses_the_nearest_function_safe_logo_placement() {
    let version_ten = Version::new(10).unwrap();
    let encoded = encode(EncodeRequest::with_version_range(
        LONG_ONE_URL,
        ErrorCorrection::High,
        version_ten,
        version_ten,
    ))
    .unwrap();
    let profile = SUPPORTED_PROFILES
        .into_iter()
        .find(|profile| profile.id() == ProfileId::AdaptiveBranded)
        .unwrap();
    let options = RenderOptions::safe(profile)
        .unwrap()
        .with_logo(LogoStyle::Bundled)
        .unwrap();
    let placement = RenderModel::new(&encoded, options)
        .unwrap()
        .logo_placement()
        .unwrap();
    let source = placement.source_bounds();
    let knockout = placement.knockout_bounds();

    assert_eq!(source.left_ten_thousandths(), 220_000);
    assert_eq!(source.top_ten_thousandths(), 200_625);
    assert_eq!(source.width_ten_thousandths(), 130_000);
    assert_eq!(source.height_ten_thousandths(), 48_750);
    assert_eq!(
        (
            knockout.left().get(),
            knockout.top().get(),
            knockout.width().get(),
            knockout.height().get(),
        ),
        (21, 19, 15, 7),
    );
    assert_eq!(placement.obscured_modules(), 105);

    for y in knockout.top().get()..knockout.top().get() + knockout.height().get() {
        for x in knockout.left().get()..knockout.left().get() + knockout.width().get() {
            assert!(matches!(
                encoded
                    .modules()
                    .module(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())
                    .unwrap()
                    .kind(),
                ModuleKind::Data | ModuleKind::Remainder
            ));
        }
    }
}

#[test]
fn every_enabled_fixed_and_adaptive_logo_placement_is_function_safe() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let text = high_versions::payload_for_high_version(version_number).unwrap();
            let encoded = encode(EncodeRequest::first_fit(
                &text,
                ErrorCorrection::High,
                Version::new(version_number).unwrap(),
            ))
            .unwrap();
            assert_eq!(encoded.version().number(), version_number);

            let options = RenderOptions::safe(profile)
                .unwrap()
                .with_logo(LogoStyle::Bundled)
                .unwrap();
            let model = RenderModel::new(&encoded, options);
            let adaptive = profile.id() == ProfileId::AdaptiveBranded;
            let enabled = version_number == 6 || (adaptive && version_number > 6);
            if !enabled {
                assert_eq!(model.unwrap_err(), RenderError::UnsafeLogoGeometry);
                continue;
            }
            let model = model.unwrap_or_else(|error| {
                panic!("version {version_number} should have centered logo geometry: {error}")
            });
            let placement = model.logo_placement().expect("logo placement");
            let knockout = placement.knockout_bounds();
            let source = placement.source_bounds();
            let matrix_width = u32::from(encoded.version().symbol_size());

            assert_eq!(
                source.left_ten_thousandths() * 2 + source.width_ten_thousandths(),
                matrix_width * 10_000,
                "version {version_number} logo is not horizontally centered",
            );
            if version_number == 6 {
                assert_eq!(
                    source.top_ten_thousandths() * 2 + source.height_ten_thousandths(),
                    matrix_width * 10_000,
                    "version {version_number} logo is not vertically centered",
                );
            } else {
                assert_eq!(
                    source.top_ten_thousandths() * 2 + source.height_ten_thousandths() + 120_000,
                    matrix_width * 10_000,
                    "version {version_number} logo is not shifted six modules upward",
                );
            }

            assert!(knockout.width().get() * 5 <= matrix_width * 2);
            assert!(knockout.height().get() * 5 <= matrix_width * 2);
            assert!(source.width_ten_thousandths() * 100 >= (matrix_width + 8) * 10_000 * 17);
            let padding = 10_000;
            assert!(source.left_ten_thousandths() >= knockout.left().get() * 10_000 + padding);
            assert!(source.top_ten_thousandths() >= knockout.top().get() * 10_000 + padding);
            assert!(
                source.right_ten_thousandths()
                    <= (knockout.left().get() + knockout.width().get()) * 10_000 - padding
            );
            assert!(
                source.bottom_ten_thousandths()
                    <= (knockout.top().get() + knockout.height().get()) * 10_000 - padding
            );
            assert_eq!(
                source.width_ten_thousandths() * 240,
                source.height_ten_thousandths() * 640
            );

            let mut obscured = 0_u32;
            for y in knockout.top().get()..knockout.top().get() + knockout.height().get() {
                for x in knockout.left().get()..knockout.left().get() + knockout.width().get() {
                    let module = encoded
                        .modules()
                        .module(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())
                        .unwrap();
                    assert!(matches!(
                        module.kind(),
                        ModuleKind::Data | ModuleKind::Remainder
                    ));
                    obscured += 1;
                }
            }
            assert_eq!(placement.obscured_modules(), obscured);
            assert_eq!(placement.obscured_data_modules(), 105);
            assert_eq!(placement.obscured_remainder_modules(), 0);
            assert_eq!(options.safety(), OutputSafety::Caution);
        }
    }
}

#[test]
fn logo_requires_high_ecc_and_opaque_white() {
    let profile = SUPPORTED_PROFILES[0];
    let encoded = encode(EncodeRequest::first_fit(
        "logo",
        ErrorCorrection::Medium,
        profile.maximum_version(),
    ))
    .unwrap();
    let options = RenderOptions::safe(profile)
        .unwrap()
        .with_logo(LogoStyle::Bundled)
        .unwrap();
    assert!(RenderModel::new(&encoded, options).is_err());

    assert!(
        RenderOptions::approved(profile, Foreground::Brand, Background::Transparent,)
            .unwrap()
            .with_logo(LogoStyle::Bundled)
            .is_err()
    );
    assert_eq!(options.background(), Background::Opaque(Rgba::WHITE));
}
