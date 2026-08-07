#[path = "support/versions.rs"]
mod versions;

use proptest::prelude::*;
use qr_core::matrix::ModuleKind;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_BACKGROUNDS, APPROVED_DATA_MODULE_STYLES, APPROVED_FOREGROUNDS, Background,
    ContrastRatio, DataModuleStyle, FinderStyle, Foreground, FunctionModuleStyle, LogoStyle,
    MAX_RGBA_BUFFER_BYTES, OutputProfile, OutputSafety, ProfileId, RenderError, RenderModel,
    RenderOptions, Rgba, SUPPORTED_PROFILES, Version, render_png, render_svg,
};

#[test]
fn safe_model_preserves_the_encoded_symbol_and_approved_preset() {
    let encoded = encoded_qr("SAFE RENDER MODEL");
    let original = encoded.clone();
    let profile = SUPPORTED_PROFILES[1];
    let options = RenderOptions::safe(profile).unwrap();

    let model = RenderModel::new(&encoded, options).unwrap();

    assert_eq!(encoded, original);
    assert_eq!(model.matrix(), encoded.modules());
    assert_eq!(model.version(), encoded.version());
    assert_eq!(model.ecc(), encoded.ecc());
    assert_eq!(model.mask(), encoded.mask());
    assert_eq!(model.options().foreground(), Rgba::BRAND);
    assert_eq!(
        model.options().background(),
        Background::Opaque(Rgba::WHITE)
    );
    assert_eq!(model.options().data_module_style(), DataModuleStyle::Square);
    assert_eq!(
        model.options().function_module_style(),
        FunctionModuleStyle::Square
    );
    assert_eq!(model.options().finder_style(), FinderStyle::StandardSquare);
    assert_eq!(model.options().logo_style(), LogoStyle::None);
}

#[test]
fn approved_color_and_background_options_have_measurable_safety() {
    let profile = SUPPORTED_PROFILES[1];

    let safe = RenderOptions::approved(profile, Foreground::Brand, Background::Opaque(Rgba::WHITE))
        .unwrap();
    assert_eq!(safe.foreground(), Rgba::BRAND);
    assert_eq!(safe.safety(), OutputSafety::Safe);
    assert_eq!(
        safe.contrast_ratio(),
        Some(ContrastRatio::from_hundredths(604))
    );

    let brand =
        RenderOptions::approved(profile, Foreground::Brand, Background::Opaque(Rgba::WHITE))
            .unwrap();
    assert_eq!(brand.foreground(), Rgba::BRAND);
    assert_eq!(brand.safety(), OutputSafety::Safe);
    assert_eq!(
        brand.contrast_ratio(),
        Some(ContrastRatio::from_hundredths(604))
    );

    let transparent =
        RenderOptions::approved(profile, Foreground::Brand, Background::Transparent).unwrap();
    assert_eq!(transparent.safety(), OutputSafety::Caution);
    assert_eq!(transparent.contrast_ratio(), None);
}

#[test]
fn only_approved_combinations_can_be_rendered_and_unsafe_contrast_is_typed() {
    let profile = SUPPORTED_PROFILES[1];
    let unsafe_gray = Rgba::opaque(119, 119, 119);

    assert_eq!(
        RenderOptions::try_new(profile, unsafe_gray, Background::Opaque(Rgba::WHITE)),
        Err(RenderError::UnsafeContrast {
            actual: ContrastRatio::from_hundredths(447),
            minimum: ContrastRatio::MINIMUM_OPAQUE,
        })
    );
    assert_eq!(
        RenderOptions::try_new(
            profile,
            Rgba::opaque(119, 118, 124),
            Background::Opaque(Rgba::WHITE),
        ),
        Err(RenderError::UnsafeContrast {
            actual: ContrastRatio::from_hundredths(449),
            minimum: ContrastRatio::MINIMUM_OPAQUE,
        })
    );
    assert_eq!(
        RenderOptions::try_new(
            profile,
            Rgba::opaque(0, 96, 0),
            Background::Opaque(Rgba::WHITE),
        ),
        Err(RenderError::UnapprovedColorCombination)
    );
}

#[test]
fn generated_approved_color_background_profile_matrix_is_complete() {
    let combinations = SUPPORTED_PROFILES.len()
        * APPROVED_FOREGROUNDS.len()
        * APPROVED_BACKGROUNDS.len()
        * APPROVED_DATA_MODULE_STYLES.len();
    assert_eq!(combinations, 16);

    for profile in SUPPORTED_PROFILES {
        for foreground in APPROVED_FOREGROUNDS {
            for background in APPROVED_BACKGROUNDS {
                for data_module_style in APPROVED_DATA_MODULE_STYLES {
                    let options = RenderOptions::approved_with_data_style(
                        profile,
                        foreground,
                        background,
                        data_module_style,
                    )
                    .unwrap();
                    assert_eq!(options.data_module_style(), data_module_style);
                    assert_eq!(options.function_module_style(), FunctionModuleStyle::Square);
                    assert_eq!(options.finder_style(), FinderStyle::StandardSquare);
                }
            }
        }
    }
}

#[test]
fn rounded_data_style_changes_no_encoded_symbol_decisions() {
    let encoded = encoded_qr("ROUNDED DATA MODULES");
    let original = encoded.clone();
    let options = RenderOptions::approved_with_data_style(
        SUPPORTED_PROFILES[1],
        Foreground::Brand,
        Background::Opaque(Rgba::WHITE),
        DataModuleStyle::Rounded,
    )
    .unwrap();
    let model = RenderModel::new(&encoded, options).unwrap();

    assert_eq!(encoded, original);
    assert_eq!(model.matrix(), original.modules());
    assert_eq!(model.version(), original.version());
    assert_eq!(model.ecc(), original.ecc());
    assert_eq!(model.mask(), original.mask());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn approved_appearance_preserves_encoding_and_deterministic_artifacts(
        payload in "[A-Za-z0-9:/._-]{1,80}",
        profile_index in 0_usize..SUPPORTED_PROFILES.len(),
        foreground_index in 0_usize..APPROVED_FOREGROUNDS.len(),
        background_index in 0_usize..APPROVED_BACKGROUNDS.len(),
    ) {
        let profile = SUPPORTED_PROFILES[profile_index];
        let encoded = encode(EncodeRequest {
            text: &payload,
            ecc: ErrorCorrection::Medium,
            max_version: profile.maximum_version(),
        }).unwrap();
        let original = encoded.clone();
        let options = RenderOptions::approved(
            profile,
            APPROVED_FOREGROUNDS[foreground_index],
            APPROVED_BACKGROUNDS[background_index],
        ).unwrap();
        let model = RenderModel::new(&encoded, options).unwrap();

        prop_assert_eq!(&encoded, &original);
        prop_assert_eq!(model.matrix(), original.modules());
        prop_assert_eq!(model.version(), original.version());
        prop_assert_eq!(model.ecc(), original.ecc());
        prop_assert_eq!(model.mask(), original.mask());
        prop_assert_eq!(render_svg(&model).unwrap(), render_svg(&model).unwrap());
        prop_assert_eq!(render_png(&model).unwrap(), render_png(&model).unwrap());
    }
}

#[test]
fn shared_symbol_geometry_drives_tight_svg_and_fixed_canvas_png_placement() {
    let encoded = encoded_qr("A");
    assert_eq!(encoded.version().number(), 1);
    let profile = SUPPORTED_PROFILES[0];
    let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();

    assert_eq!(model.symbol().matrix_modules().get(), 21);
    assert_eq!(model.symbol().quiet_zone_modules_per_side().get(), 4);
    assert_eq!(model.symbol().extent_modules().get(), 29);

    let svg = model.svg_placement();
    assert_eq!(svg.output_dimensions(), profile.svg_dimensions());
    assert_eq!(svg.view_box().width().get(), 29);
    assert_eq!(svg.view_box().height().get(), 29);
    assert_eq!(svg.matrix_origin().x().get(), 4);
    assert_eq!(svg.matrix_origin().y().get(), 4);

    let png = model.png_placement();
    assert_eq!(png.canvas_dimensions(), profile.png_dimensions());
    assert_eq!(png.symbol(), model.symbol());
    assert_eq!(png.module_scale().get(), 8);
    assert_eq!(png.outer_padding().left.get(), 19);
    assert_eq!(png.outer_padding().right.get(), 19);
    assert_eq!(png.outer_padding().top.get(), 19);
    assert_eq!(png.outer_padding().bottom.get(), 19);
    assert_eq!(png.matrix_origin().x().get(), 51);
    assert_eq!(png.matrix_origin().y().get(), 51);
}

#[test]
fn every_supported_profile_version_has_complete_composed_placement() {
    for profile in SUPPORTED_PROFILES {
        for version_number in 1..=profile.maximum_version().number() {
            let encoded = encoded_qr_at_version(version_number);
            let model = RenderModel::new(&encoded, RenderOptions::safe(profile).unwrap()).unwrap();
            let symbol = model.symbol();
            let svg = model.svg_placement();
            let png = model.png_placement();

            assert_eq!(encoded.version().number(), version_number);
            assert_eq!(
                symbol.matrix_modules().get(),
                u32::from(encoded.modules().size())
            );
            assert_eq!(symbol.quiet_zone_modules_per_side().get(), 4);
            assert_eq!(
                symbol.extent_modules().get(),
                u32::from(encoded.modules().size()) + 8
            );
            assert_eq!(svg.output_dimensions(), profile.svg_dimensions());
            assert_eq!(svg.view_box().width(), symbol.extent_modules());
            assert_eq!(svg.view_box().height(), symbol.extent_modules());
            assert_eq!(svg.matrix_origin().x().get(), 4);
            assert_eq!(svg.matrix_origin().y().get(), 4);
            assert_eq!(png.symbol(), symbol);
            assert_eq!(png.canvas_dimensions(), profile.png_dimensions());
            assert_eq!(png.outer_padding().left, png.outer_padding().right);
            assert_eq!(png.outer_padding().top, png.outer_padding().bottom);
            assert_eq!(
                png.matrix_origin().x().get(),
                png.outer_padding().left.get() + 4 * png.module_scale().get()
            );
            assert_eq!(
                png.matrix_origin().y().get(),
                png.outer_padding().top.get() + 4 * png.module_scale().get()
            );
            assert_eq!(
                png.rgba_buffer_len(),
                usize::try_from(
                    u64::from(profile.png_dimensions().width().get())
                        * u64::from(profile.png_dimensions().height().get())
                        * 4
                )
                .unwrap()
            );
        }
    }
}

#[test]
fn function_module_ownership_is_preserved_as_protected_geometry() {
    let encoded = encoded_qr("PROTECT FUNCTION MODULES");
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap(),
    )
    .unwrap();

    let mut brandable_count = 0;
    for cell in model.cells() {
        let expected_protection = !matches!(
            cell.module().kind(),
            ModuleKind::Data | ModuleKind::Remainder
        );
        assert_eq!(cell.is_protected(), expected_protection);
        assert_eq!(
            model.matrix().module(cell.x(), cell.y()),
            Some(cell.module())
        );
    }

    for cell in model.brandable_cells() {
        assert!(matches!(
            cell.module().kind(),
            ModuleKind::Data | ModuleKind::Remainder
        ));
        brandable_count += 1;
    }
    assert_eq!(
        brandable_count,
        model.cells().filter(|cell| !cell.is_protected()).count()
    );
}

#[test]
fn repeated_construction_is_deterministic() {
    let encoded = encoded_qr("DETERMINISTIC");
    let options = RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap();

    let first = RenderModel::new(&encoded, options).unwrap();
    let second = RenderModel::new(&encoded, options).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        first.cells().collect::<Vec<_>>(),
        second.cells().collect::<Vec<_>>()
    );
}

#[test]
fn impractical_allocations_have_a_target_independent_typed_failure() {
    let encoded = encoded_qr("BOUNDS");
    let impractical = OutputProfile::try_new(
        ProfileId::Inline,
        qr_render::PixelDimensions::square(50_000),
        qr_render::PixelDimensions::square(150_000),
        Version::try_from(1).unwrap(),
    )
    .unwrap();

    assert_eq!(
        RenderModel::new(&encoded, RenderOptions::safe(impractical).unwrap()),
        Err(RenderError::AllocationTooLarge {
            required_bytes: 90_000_000_000,
            maximum_bytes: MAX_RGBA_BUFFER_BYTES,
        })
    );
}

fn encoded_qr(text: &str) -> EncodedQr {
    encode(EncodeRequest {
        text,
        ecc: ErrorCorrection::Medium,
        max_version: Version::try_from(8).unwrap(),
    })
    .unwrap()
}

fn encoded_qr_at_version(version: u8) -> EncodedQr {
    let length = versions::first_byte_length(version);
    let text = "a".repeat(length);
    encode(EncodeRequest {
        text: &text,
        ecc: ErrorCorrection::Medium,
        max_version: Version::try_from(version).unwrap(),
    })
    .unwrap()
}
