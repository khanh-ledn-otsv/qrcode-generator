use std::error::Error;

use qr_core::Version;
use qr_render::{
    CanvasGeometry, GeometryError, ModuleCount, PixelDimensions, ProfileError, SUPPORTED_PROFILES,
};

fn assert_error(error: &(dyn Error + 'static), fragment: &str) {
    assert!(error.to_string().contains(fragment));
}

#[test]
fn every_geometry_and_profile_error_has_stable_context() {
    let version_one = Version::new(1).expect("version 1 is valid");
    let version_two = Version::new(2).expect("version 2 is valid");
    let geometry_errors = [
        GeometryError::InvalidCanvasDimensions,
        GeometryError::InvalidModuleCount,
        GeometryError::DimensionOverflow,
        GeometryError::NoPositiveEvenScale,
        GeometryError::OuterPaddingIsNotIntegral,
        GeometryError::VersionExceedsProfile {
            requested: version_two,
            maximum: version_one,
        },
    ];
    for (error, fragment) in geometry_errors.iter().zip([
        "canvas dimensions",
        "module count",
        "overflowed",
        "positive even",
        "symmetric and integral",
        "exceeds the profile maximum",
    ]) {
        assert_error(error, fragment);
    }

    let profile_errors = [
        ProfileError::DimensionsMustBePositive,
        ProfileError::DimensionsMustBeSquare,
        ProfileError::DimensionOverflow,
        ProfileError::PngDimensionsAreNotTriple {
            base: PixelDimensions::square(90),
            png: PixelDimensions::square(180),
        },
        ProfileError::AdaptiveDimensionsDoNotMatchMaximum {
            expected_base: PixelDimensions::square(740),
            expected_png: PixelDimensions::square(1110),
            base: PixelDimensions::square(180),
            png: PixelDimensions::square(540),
        },
        ProfileError::InvalidGeometry(GeometryError::NoPositiveEvenScale),
        ProfileError::MaximumVersionScaleBelowSix,
    ];
    for (error, fragment) in profile_errors.iter().zip([
        "positive",
        "square",
        "overflowed",
        "exactly three times",
        "adaptive dimensions must match",
        "invalid profile geometry",
        "at least six pixels",
    ]) {
        assert_error(error, fragment);
    }
}

#[test]
fn public_geometry_accessors_expose_the_calculated_contract() {
    assert_eq!(ModuleCount::new(0), Err(GeometryError::InvalidModuleCount));
    assert_eq!(
        CanvasGeometry::calculate(
            PixelDimensions::new(0, 90),
            ModuleCount::new(21).expect("21 modules is valid"),
        ),
        Err(GeometryError::InvalidCanvasDimensions)
    );

    let profile = SUPPORTED_PROFILES[0];
    let geometry = profile
        .geometry(Version::new(1).expect("version 1 is valid"))
        .expect("supported profile geometry calculates");
    assert_eq!(profile.png_dimensions(), geometry.canvas_dimensions());
    assert_eq!(geometry.matrix_modules().get(), 21);
    assert_eq!(
        geometry.rendered_symbol_dimensions().width(),
        geometry.rendered_symbol_dimensions().height()
    );
}
