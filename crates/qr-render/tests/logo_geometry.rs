use qr_core::matrix::ModuleKind;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    Background, Foreground, LogoStyle, OutputSafety, RenderModel, RenderOptions, Rgba,
    SUPPORTED_PROFILES,
};

#[test]
fn bundled_logo_geometry_is_version_aware_bounded_and_function_safe() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let text = payload_for_high_version(version_number);
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
            let model = RenderModel::new(&encoded, options).unwrap();
            let placement = model.logo_placement().expect("logo placement");
            let knockout = placement.knockout_bounds();
            let source = placement.source_bounds();
            let matrix_width = u32::from(encoded.version().symbol_size());

            assert!(knockout.width().get() * 5 <= matrix_width);
            assert!(knockout.height().get() * 5 <= matrix_width);
            assert!(source.left_thousandths() >= knockout.left().get() * 1_000 + 1_000);
            assert!(source.top_thousandths() >= knockout.top().get() * 1_000 + 1_000);
            assert!(
                source.right_thousandths()
                    <= (knockout.left().get() + knockout.width().get()) * 1_000 - 1_000
            );
            assert!(
                source.bottom_thousandths()
                    <= (knockout.top().get() + knockout.height().get()) * 1_000 - 1_000
            );
            assert_eq!(
                source.width_thousandths() * 602,
                source.height_thousandths() * 1_000
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

fn payload_for_high_version(version_number: u8) -> String {
    for length in 1..=1_000 {
        let text = "a".repeat(length);
        if encode(EncodeRequest {
            text: &text,
            ecc: ErrorCorrection::High,
            max_version: Version::new(version_number).unwrap(),
        })
        .is_ok_and(|encoded| encoded.version().number() == version_number)
        {
            return text;
        }
    }
    panic!("no byte payload selected version {version_number}");
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
