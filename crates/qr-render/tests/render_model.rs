#[path = "support/versions.rs"]
mod versions;

use proptest::prelude::*;
use qr_core::matrix::ModuleKind;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, EncodedQr, encode};
use qr_render::{
    APPROVED_FOREGROUND_THEMES, ContrastRatio, ForegroundTheme, GlyphOwnership, LogoStyle,
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
    assert_eq!(model.options().background(), Rgba::WHITE);
    assert_eq!(model.options().logo_style(), LogoStyle::None);
}

#[test]
fn fixed_opaque_appearance_has_measurable_safety() {
    let profile = SUPPORTED_PROFILES[1];
    let safe = RenderOptions::safe(profile).unwrap();
    let black = safe.with_foreground_theme(ForegroundTheme::Black).unwrap();
    assert_eq!(
        APPROVED_FOREGROUND_THEMES,
        [ForegroundTheme::Magenta, ForegroundTheme::Black]
    );
    assert_eq!(safe.foreground(), Rgba::BRAND);
    assert_eq!(black.foreground(), Rgba::BLACK);
    assert_eq!(safe.background(), Rgba::WHITE);
    assert_eq!(black.background(), Rgba::WHITE);
    assert_eq!(safe.safety(), OutputSafety::Safe);
    assert_eq!(black.safety(), OutputSafety::Safe);
    assert_eq!(safe.contrast_ratio(), ContrastRatio::from_hundredths(604));
    assert_eq!(black.contrast_ratio(), ContrastRatio::from_hundredths(2100));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn approved_appearance_preserves_encoding_and_deterministic_artifacts(
        payload in "[A-Za-z0-9:/._-]{1,80}",
        profile_index in 0_usize..SUPPORTED_PROFILES.len(),
    ) {
        let profile = SUPPORTED_PROFILES[profile_index];
        let encoded = encode(EncodeRequest::first_fit(&payload, ErrorCorrection::Medium, profile.maximum_version())).unwrap();
        let original = encoded.clone();
        let options = RenderOptions::safe(profile).unwrap();
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
    assert_eq!(png.module_scale().get(), 10);
    assert_eq!(png.outer_padding().left.get(), 5);
    assert_eq!(png.outer_padding().right.get(), 5);
    assert_eq!(png.outer_padding().top.get(), 5);
    assert_eq!(png.outer_padding().bottom.get(), 5);
    assert_eq!(png.matrix_origin().x().get(), 45);
    assert_eq!(png.matrix_origin().y().get(), 45);
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
            assert_eq!(
                svg.output_dimensions(),
                profile.svg_dimensions_for(encoded.version()).unwrap()
            );
            assert_eq!(svg.view_box().width(), symbol.extent_modules());
            assert_eq!(svg.view_box().height(), symbol.extent_modules());
            assert_eq!(svg.matrix_origin().x().get(), 4);
            assert_eq!(svg.matrix_origin().y().get(), 4);
            assert_eq!(png.symbol(), symbol);
            assert_eq!(
                png.canvas_dimensions(),
                profile.png_dimensions_for(encoded.version()).unwrap()
            );
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
                    u64::from(
                        profile
                            .png_dimensions_for(encoded.version())
                            .unwrap()
                            .width()
                            .get(),
                    ) * u64::from(
                        profile
                            .png_dimensions_for(encoded.version())
                            .unwrap()
                            .height()
                            .get(),
                    ) * 4
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
fn visible_symbol_glyphs_are_row_major_and_retain_module_ownership() {
    let encoded = encoded_qr("CLASSIFIED GLYPHS");
    let model = RenderModel::new(
        &encoded,
        RenderOptions::safe(SUPPORTED_PROFILES[1]).unwrap(),
    )
    .unwrap();
    let glyphs = model.glyphs().collect::<Vec<_>>();

    assert_eq!(
        glyphs.len(),
        model.cells().filter(|cell| cell.module().is_dark()).count()
    );
    assert!(glyphs.windows(2).all(|pair| {
        let first = pair[0];
        let second = pair[1];
        (first.y(), first.x()) < (second.y(), second.x())
    }));

    for glyph in &glyphs {
        let module = model.matrix().module(glyph.x(), glyph.y()).unwrap();
        assert!(module.is_dark());
        assert_eq!(
            glyph.ownership(),
            match module.kind() {
                ModuleKind::Finder => GlyphOwnership::Finder,
                ModuleKind::Separator => GlyphOwnership::Separator,
                ModuleKind::Data => GlyphOwnership::Data,
                ModuleKind::Remainder => GlyphOwnership::Remainder,
                ModuleKind::Timing
                | ModuleKind::Alignment
                | ModuleKind::Format
                | ModuleKind::Version
                | ModuleKind::Dark => GlyphOwnership::OtherFunction,
            }
        );
    }

    assert!(model.cells().any(|cell| {
        cell.module().kind() == ModuleKind::Separator
            && cell.ownership() == GlyphOwnership::Separator
            && cell.glyph().is_none()
    }));
    assert_eq!(
        model
            .cells()
            .filter_map(|cell| cell.glyph())
            .collect::<Vec<_>>(),
        glyphs
    );
}

#[test]
fn logo_knockout_is_applied_before_visible_glyphs_reach_artifact_adapters() {
    let version_six = Version::try_from(6).unwrap();
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
    let model = RenderModel::new(&encoded, options).unwrap();
    let knockout = model.logo_placement().unwrap().knockout_bounds();
    let is_knocked_out = |x: u16, y: u16| {
        let x = u32::from(x);
        let y = u32::from(y);
        x >= knockout.left().get()
            && y >= knockout.top().get()
            && x < knockout.left().get() + knockout.width().get()
            && y < knockout.top().get() + knockout.height().get()
    };

    assert!(
        model
            .glyphs()
            .all(|glyph| !is_knocked_out(glyph.x(), glyph.y()))
    );
    assert_eq!(
        model.glyphs().count(),
        model
            .cells()
            .filter(|cell| cell.module().is_dark() && !is_knocked_out(cell.x(), cell.y()))
            .count()
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
    encode(EncodeRequest::first_fit(
        text,
        ErrorCorrection::Medium,
        Version::try_from(8).unwrap(),
    ))
    .unwrap()
}

fn encoded_qr_at_version(version: u8) -> EncodedQr {
    let length = versions::first_byte_length(version);
    let text = "a".repeat(length);
    encode(EncodeRequest::first_fit(
        &text,
        ErrorCorrection::Medium,
        Version::try_from(version).unwrap(),
    ))
    .unwrap()
}
