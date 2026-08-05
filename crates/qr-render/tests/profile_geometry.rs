use qr_render::{
    CanvasGeometry, GeometryError, ModuleCount, OutputProfile, PixelDimensions, ProfileError,
    ProfileId, QrVersion, SUPPORTED_PROFILES,
};

#[test]
fn supported_profiles_match_the_approved_output_contract() {
    let expected = [
        (ProfileId::Inline, 90, 270, 5),
        (ProfileId::Content, 120, 360, 8),
        (ProfileId::Landing, 150, 450, 12),
        (ProfileId::Print, 160, 480, 13),
    ];

    assert_eq!(SUPPORTED_PROFILES.len(), expected.len());

    for (profile, (id, base_side, png_side, maximum_version)) in
        SUPPORTED_PROFILES.iter().zip(expected)
    {
        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(profile.id(), id);
        assert_eq!(
            profile.base_dimensions(),
            PixelDimensions::square(base_side)
        );
        assert_eq!(profile.png_dimensions(), PixelDimensions::square(png_side));
        assert_eq!(profile.maximum_version().number(), maximum_version);
    }
}

#[test]
fn every_supported_version_has_centered_even_integer_geometry() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let version = QrVersion::try_from(version_number).unwrap();
            let geometry = profile.geometry(version).unwrap();

            assert_eq!(geometry.quiet_zone_modules(), ModuleCount::new(4).unwrap());
            assert!(geometry.module_scale().get() > 0);
            assert_eq!(geometry.module_scale().get() % 2, 0);
            assert_eq!(
                geometry.outer_padding().left,
                geometry.outer_padding().right
            );
            assert_eq!(
                geometry.outer_padding().top,
                geometry.outer_padding().bottom
            );
            assert_eq!(geometry.canvas_dimensions(), profile.png_dimensions());

            let next_even_scale = geometry.module_scale().get() + 2;
            let next_width = geometry.symbol_modules().get() * next_even_scale;
            assert!(next_width > profile.png_dimensions().width().get());
        }

        let maximum_geometry = profile.geometry(profile.maximum_version()).unwrap();
        assert!(maximum_geometry.module_scale().get() >= 6);
    }
}

#[test]
fn profile_ceiling_geometry_matches_the_approved_worked_examples() {
    let expected = [
        (ProfileId::Inline, 45, 6, 0),
        (ProfileId::Content, 57, 6, 9),
        (ProfileId::Landing, 73, 6, 6),
        (ProfileId::Print, 77, 6, 9),
    ];

    for (profile, (id, symbol_modules, scale, padding)) in SUPPORTED_PROFILES.iter().zip(expected) {
        let geometry = profile.geometry(profile.maximum_version()).unwrap();
        assert_eq!(profile.id(), id);
        assert_eq!(geometry.symbol_modules().get(), symbol_modules);
        assert_eq!(geometry.module_scale().get(), scale);
        assert_eq!(geometry.outer_padding().left.get(), padding);
    }
}

#[test]
fn scale_transitions_are_exercised_for_each_profile() {
    let expected_scales: [&[u32]; 4] = [
        &[8, 8, 6, 6, 6],
        &[12, 10, 8, 8, 8, 6, 6, 6],
        &[14, 12, 12, 10, 10, 8, 8, 6, 6, 6, 6, 6],
        &[16, 14, 12, 10, 10, 8, 8, 8, 6, 6, 6, 6, 6],
    ];

    for (profile, expected) in SUPPORTED_PROFILES.iter().zip(expected_scales) {
        let actual: Vec<u32> = (1..=profile.maximum_version().number())
            .map(|number| {
                profile
                    .geometry(QrVersion::try_from(number).unwrap())
                    .unwrap()
                    .module_scale()
                    .get()
            })
            .collect();
        assert_eq!(actual, expected);
    }
}

#[test]
fn invalid_profiles_return_specific_errors() {
    let base = PixelDimensions::square(90);
    let incorrect_png = PixelDimensions::square(269);

    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Inline,
            base,
            incorrect_png,
            QrVersion::try_from(5).unwrap(),
        ),
        Err(ProfileError::PngDimensionsAreNotTriple {
            base,
            png: incorrect_png,
        })
    );

    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Inline,
            PixelDimensions::square(u32::MAX),
            PixelDimensions::square(270),
            QrVersion::try_from(5).unwrap(),
        ),
        Err(ProfileError::DimensionOverflow)
    );
}

#[test]
fn impossible_asymmetric_and_overflowing_geometry_return_typed_errors() {
    assert_eq!(
        CanvasGeometry::calculate(PixelDimensions::square(2), ModuleCount::new(21).unwrap()),
        Err(GeometryError::NoPositiveEvenScale)
    );
    assert_eq!(
        CanvasGeometry::calculate(PixelDimensions::square(271), ModuleCount::new(37).unwrap()),
        Err(GeometryError::OuterPaddingIsNotIntegral)
    );
    assert_eq!(
        CanvasGeometry::calculate(
            PixelDimensions::square(u32::MAX),
            ModuleCount::new(u32::MAX).unwrap()
        ),
        Err(GeometryError::DimensionOverflow)
    );
}

#[test]
fn a_version_above_the_profile_ceiling_is_rejected() {
    let inline = SUPPORTED_PROFILES[0];
    assert_eq!(
        inline.geometry(QrVersion::try_from(6).unwrap()),
        Err(GeometryError::VersionExceedsProfile {
            requested: QrVersion::try_from(6).unwrap(),
            maximum: QrVersion::try_from(5).unwrap(),
        })
    );
}
