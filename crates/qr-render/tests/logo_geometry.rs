#[path = "support/high_versions.rs"]
mod high_versions;

use qr_core::matrix::ModuleKind;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    Background, Foreground, LogoStyle, OutputSafety, RenderError, RenderModel, RenderOptions, Rgba,
    SUPPORTED_PROFILES,
};

#[test]
fn bundled_logo_geometry_is_version_aware_bounded_and_function_safe() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let text = high_versions::payload_for_high_version(version_number).unwrap();
            let encoded = encode(EncodeRequest {
                text: &text,
                ecc: ErrorCorrection::High,
                max_version: Version::new(version_number).unwrap(),
            })
            .unwrap();
            assert_eq!(encoded.version().number(), version_number);

            let options = RenderOptions::safe(profile)
                .unwrap()
                .with_logo(LogoStyle::Bundled)
                .unwrap();
            let model = RenderModel::new(&encoded, options);
            if version_number >= 7 {
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
                source.left_thousandths() * 2 + source.width_thousandths(),
                matrix_width * 1_000,
                "version {version_number} logo is not horizontally centered",
            );
            assert_eq!(
                source.top_thousandths() * 2 + source.height_thousandths(),
                matrix_width * 1_000,
                "version {version_number} logo is not vertically centered",
            );

            assert!(knockout.width().get() * 5 <= matrix_width * 2);
            assert!(knockout.height().get() * 5 <= matrix_width * 2);
            assert!(source.width_thousandths() * 100 >= (matrix_width + 8) * 1_000 * 17);
            let padding = if version_number == 1 { 0 } else { 1_000 };
            assert!(source.left_thousandths() >= knockout.left().get() * 1_000 + padding);
            assert!(source.top_thousandths() >= knockout.top().get() * 1_000 + padding);
            assert!(
                source.right_thousandths()
                    <= (knockout.left().get() + knockout.width().get()) * 1_000 - padding
            );
            assert!(
                source.bottom_thousandths()
                    <= (knockout.top().get() + knockout.height().get()) * 1_000 - padding
            );
            assert_eq!(
                source.width_thousandths() * 240,
                source.height_thousandths() * 640
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
            assert_eq!(options.safety(), OutputSafety::Caution);
        }
    }
}

#[test]
fn logo_requires_high_ecc_and_opaque_white() {
    let profile = SUPPORTED_PROFILES[0];
    let encoded = encode(EncodeRequest {
        text: "logo",
        ecc: ErrorCorrection::Medium,
        max_version: profile.maximum_version(),
    })
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
