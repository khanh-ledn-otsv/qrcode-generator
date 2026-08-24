#[path = "support/inline_version_six_artifact_fixture.rs"]
mod inline_version_six_artifact_fixture;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use qr_render::{
    CanvasGeometry, GeometryError, ModuleCount, OutputProfile, PaddingContent, PixelDimensions,
    ProfileError, ProfileId, SUPPORTED_PROFILES, Version,
};

#[test]
fn unbranded_inline_version_six_hashes_are_pinned_on_native() {
    assert_eq!(
        inline_version_six_artifact_fixture::SHA256,
        inline_version_six_artifact_fixture::provenance_hashes()
    );
    assert_eq!(
        inline_version_six_artifact_fixture::hashes(),
        inline_version_six_artifact_fixture::SHA256
    );
}

#[test]
#[ignore = "explicitly emits golden hashes for reviewed fixture refreshes"]
fn print_unbranded_inline_version_six_hashes_for_fixture_refresh() {
    let [svg_sha256, png_sha256] = inline_version_six_artifact_fixture::hashes();
    println!("svg_sha256={svg_sha256}");
    println!("png_sha256={png_sha256}");
}

#[test]
fn supported_profiles_match_the_approved_output_contract() {
    let expected = [
        (ProfileId::Small, 100, 300, 5, 6),
        (ProfileId::Standard, 120, 360, 5, 8),
        (ProfileId::PrimaryCta, 160, 480, 5, 12),
        (ProfileId::HeroCampaign, 200, 600, 8, 12),
        (ProfileId::BusinessCard, 148, 444, 5, 12),
        (ProfileId::FlyerBrochure, 177, 531, 5, 12),
        (ProfileId::PosterPackage, 236, 708, 5, 12),
    ];

    assert_eq!(SUPPORTED_PROFILES.len(), expected.len());

    for (profile, (id, base_side, png_side, minimum_version, maximum_version)) in
        SUPPORTED_PROFILES.iter().zip(expected)
    {
        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(profile.id(), id);
        assert_eq!(profile.svg_dimensions(), PixelDimensions::square(base_side));
        assert_eq!(
            profile.base_dimensions(),
            PixelDimensions::square(base_side)
        );
        assert_eq!(profile.png_dimensions(), PixelDimensions::square(png_side));
        assert_eq!(profile.minimum_version().number(), minimum_version);
        assert_eq!(profile.maximum_version().number(), maximum_version);
    }
}

#[test]
fn every_supported_version_has_centered_even_integer_geometry() {
    for profile in SUPPORTED_PROFILES {
        for version_number in
            profile.minimum_version().number()..=profile.maximum_version().number()
        {
            let version = Version::try_from(version_number).unwrap();
            let geometry = profile.geometry(version).unwrap();

            assert_eq!(geometry.quiet_zone_modules(), ModuleCount::new(4).unwrap());
            assert!(geometry.module_scale().get() > 0);
            assert_eq!(
                geometry.outer_padding().left,
                geometry.outer_padding().right
            );
            assert_eq!(
                geometry.outer_padding().top,
                geometry.outer_padding().bottom
            );
            assert_eq!(
                geometry.canvas_dimensions(),
                profile.png_dimensions_for(version).unwrap()
            );
            assert_eq!(
                geometry.outer_padding().content,
                PaddingContent::BackgroundOnly
            );

            let next_scale = geometry.module_scale().get() + 1;
            let next_width = geometry.symbol_modules().get() * next_scale;
            let canvas_side = profile.png_dimensions_for(version).unwrap().width().get();
            assert!(
                next_width > canvas_side || (canvas_side - next_width) % 2 != 0,
                "the selected scale is the largest symmetrically centered integer pitch"
            );
        }

        let maximum_geometry = profile.geometry(profile.maximum_version()).unwrap();
        assert!(maximum_geometry.module_scale().get() >= 6);
    }
}

#[test]
fn profile_ceiling_geometry_meets_the_approved_module_pitch_rule() {
    for profile in SUPPORTED_PROFILES {
        let geometry = profile.geometry(profile.maximum_version()).unwrap();
        assert!(geometry.module_scale().get() >= 6);
        assert_eq!(
            geometry.outer_padding().left,
            geometry.outer_padding().right
        );
        assert_eq!(
            geometry.outer_padding().top,
            geometry.outer_padding().bottom
        );
    }
}

#[test]
fn module_scale_does_not_increase_within_each_fixed_profile_range() {
    for profile in SUPPORTED_PROFILES {
        let actual: Vec<u32> = (profile.minimum_version().number()
            ..=profile.maximum_version().number())
            .map(|number| {
                profile
                    .geometry(Version::try_from(number).unwrap())
                    .unwrap()
                    .module_scale()
                    .get()
            })
            .collect();
        assert!(actual.windows(2).all(|pair| pair[0] >= pair[1]));
    }
}

#[test]
fn invalid_profiles_return_specific_errors() {
    let base = PixelDimensions::square(90);
    let incorrect_png = PixelDimensions::square(269);

    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Small,
            PixelDimensions::square(0),
            PixelDimensions::square(0),
            Version::try_from(1).unwrap(),
            Version::try_from(1).unwrap(),
        ),
        Err(ProfileError::DimensionsMustBePositive)
    );
    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Small,
            PixelDimensions::new(90, 91),
            PixelDimensions::square(270),
            Version::try_from(1).unwrap(),
            Version::try_from(1).unwrap(),
        ),
        Err(ProfileError::DimensionsMustBeSquare)
    );
    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Small,
            PixelDimensions::square(40),
            PixelDimensions::square(120),
            Version::try_from(1).unwrap(),
            Version::try_from(1).unwrap(),
        ),
        Err(ProfileError::MaximumVersionScaleBelowSix)
    );

    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Small,
            base,
            incorrect_png,
            Version::try_from(5).unwrap(),
            Version::try_from(5).unwrap(),
        ),
        Err(ProfileError::PngDimensionsAreNotTriple {
            base,
            png: incorrect_png,
        })
    );

    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Small,
            PixelDimensions::square(100),
            PixelDimensions::square(300),
            Version::try_from(7).unwrap(),
            Version::try_from(6).unwrap(),
        ),
        Err(ProfileError::InvertedVersionRange {
            minimum: Version::try_from(7).unwrap(),
            maximum: Version::try_from(6).unwrap(),
        })
    );

    assert_eq!(
        OutputProfile::try_new(
            ProfileId::Small,
            PixelDimensions::square(u32::MAX),
            PixelDimensions::square(270),
            Version::try_from(5).unwrap(),
            Version::try_from(5).unwrap(),
        ),
        Err(ProfileError::DimensionOverflow)
    );
}

#[test]
fn impossible_asymmetric_and_overflowing_geometry_return_typed_errors() {
    assert_eq!(
        CanvasGeometry::calculate(PixelDimensions::square(2), ModuleCount::new(21).unwrap()),
        Err(GeometryError::NoPositiveModuleScale)
    );
    assert!(
        CanvasGeometry::calculate(PixelDimensions::square(271), ModuleCount::new(37).unwrap())
            .is_ok()
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
    let small = SUPPORTED_PROFILES[0];
    assert_eq!(
        small.geometry(Version::try_from(7).unwrap()),
        Err(GeometryError::VersionExceedsProfile {
            requested: Version::try_from(7).unwrap(),
            maximum: Version::try_from(6).unwrap(),
        })
    );
}

fn supported_profile_and_version() -> impl Strategy<Value = (OutputProfile, Version)> {
    prop::sample::select(SUPPORTED_PROFILES.to_vec()).prop_flat_map(|profile| {
        let minimum = profile.minimum_version().number();
        let maximum = profile.maximum_version().number();
        (Just(profile), minimum..=maximum).prop_map(|(profile, number)| {
            (
                profile,
                Version::try_from(number).expect("strategy only generates valid versions"),
            )
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        rng_seed: RngSeed::Fixed(0x5152_5046_4745_4f4d),
        ..ProptestConfig::default()
    })]

    #[test]
    fn supported_profile_geometry_always_preserves_fixed_canvas_invariants(
        (profile, version) in supported_profile_and_version()
    ) {
        let geometry = profile.geometry(version).unwrap();
        let scale = geometry.module_scale().get();
        let symbol_side = geometry.symbol_modules().get();

        prop_assert_eq!(geometry.canvas_dimensions(), profile.png_dimensions_for(version).unwrap());
        prop_assert!(scale > 0);
        prop_assert_eq!(geometry.outer_padding().left, geometry.outer_padding().right);
        prop_assert_eq!(geometry.outer_padding().top, geometry.outer_padding().bottom);
        prop_assert_eq!(geometry.outer_padding().content, PaddingContent::BackgroundOnly);
        prop_assert!(symbol_side * (scale + 2) > profile.png_dimensions_for(version).unwrap().width().get());
    }

    #[test]
    fn malformed_png_dimensions_are_rejected_without_panicking(
        base_side in 1_u32..=1_000_000,
        maximum_version in 1_u8..=Version::MAX,
    ) {
        let base = PixelDimensions::square(base_side);
        let incorrect_png = PixelDimensions::square(base_side * 3 + 1);
        let result = OutputProfile::try_new(
            ProfileId::Small,
            base,
            incorrect_png,
            Version::try_from(1).unwrap(),
            Version::try_from(maximum_version).unwrap(),
        );

        prop_assert_eq!(
            result,
            Err(ProfileError::PngDimensionsAreNotTriple {
                base,
                png: incorrect_png,
            })
        );
    }
}
