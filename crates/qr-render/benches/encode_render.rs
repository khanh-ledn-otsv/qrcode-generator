use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use qr_core::codeword_stream::{CodewordStreamRequest, construct};
use qr_core::encoding;
use qr_core::matrix::{MaskId, build_function_matrix, finalize_information, place_data};
use qr_core::penalty::penalty_score;
use qr_core::selection::select_mask;
use qr_core::tables::ErrorCorrection;
use qr_core::{EncodeRequest, Version, encode};
use qr_render::{
    APPROVED_DATA_MODULE_STYLES, Background, Foreground, LogoStyle, RenderModel, RenderOptions,
    Rgba, SUPPORTED_PROFILES, render_png, render_svg,
};

fn encoding_benchmarks(criterion: &mut Criterion) {
    let cases = [
        ("version-1", "hello".to_owned(), ErrorCorrection::Medium, 1),
        (
            "profile-ceiling",
            "a".repeat(300),
            ErrorCorrection::Medium,
            13,
        ),
        ("version-40", "a".repeat(2_900), ErrorCorrection::Low, 40),
    ];
    let mut group = criterion.benchmark_group("encoding");
    for (name, payload, ecc, maximum_version) in cases {
        let version = Version::new(maximum_version).expect("benchmark version is valid");
        let selected = encode(EncodeRequest {
            text: &payload,
            ecc,
            max_version: version,
        })
        .expect("benchmark payload fits");
        assert_eq!(
            selected.version(),
            version,
            "benchmark case must select its named version"
        );
        group.throughput(Throughput::Bytes(payload.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("request", name),
            &payload,
            |bencher, text| {
                bencher.iter(|| {
                    encode(black_box(EncodeRequest {
                        text,
                        ecc,
                        max_version: version,
                    }))
                    .expect("benchmark payload fits")
                });
            },
        );
    }
    group.finish();
}

fn mask_benchmarks(criterion: &mut Criterion) {
    let data = encoding::encode(EncodeRequest {
        text: &"a".repeat(180),
        ecc: ErrorCorrection::Medium,
        max_version: Version::new(13).expect("benchmark version is valid"),
    })
    .expect("benchmark payload fits");
    let stream = construct(CodewordStreamRequest {
        version: data.version(),
        ecc: data.ecc(),
        data_codewords: data.data_codewords(),
    })
    .expect("benchmark stream is valid");
    let mut group = criterion.benchmark_group("mask-selection");
    for mask_number in MaskId::MIN..=MaskId::MAX {
        let mask = MaskId::new(mask_number).expect("benchmark mask is valid");
        group.bench_function(BenchmarkId::new("candidate", mask_number), |bencher| {
            bencher.iter(|| {
                let matrix = place_data(
                    build_function_matrix(stream.version()).expect("matrix builds"),
                    black_box(&stream),
                    mask,
                )
                .expect("data places");
                let matrix = finalize_information(matrix).expect("information finalizes");
                black_box(penalty_score(&matrix))
            });
        });
    }
    group.bench_function("all-candidates", |bencher| {
        bencher.iter(|| select_mask(black_box(&stream)).expect("mask selection succeeds"));
    });
    group.finish();
}

fn rendering_benchmarks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("rendering");
    for (profile_index, profile) in SUPPORTED_PROFILES.into_iter().enumerate() {
        let payload = "a".repeat(if profile_index == 0 { 40 } else { 70 });
        let encoded = encode(EncodeRequest {
            text: &payload,
            ecc: ErrorCorrection::Medium,
            max_version: profile.maximum_version(),
        })
        .expect("benchmark payload fits");
        for style in APPROVED_DATA_MODULE_STYLES {
            let options = RenderOptions::approved_with_data_style(
                profile,
                Foreground::Brand,
                Background::Opaque(Rgba::WHITE),
                style,
            )
            .expect("benchmark options are approved");
            let model = RenderModel::new(&encoded, options).expect("benchmark model is valid");
            group.bench_function(
                BenchmarkId::new("png", format!("{profile_index}-{style:?}")),
                |bencher| {
                    bencher.iter(|| render_png(black_box(&model)).expect("PNG renders"));
                },
            );
        }
        let model = RenderModel::new(
            &encoded,
            RenderOptions::safe(profile).expect("safe options are valid"),
        )
        .expect("benchmark model is valid");
        group.bench_function(BenchmarkId::new("svg", profile_index), |bencher| {
            bencher.iter(|| render_svg(black_box(&model)).expect("SVG renders"));
        });
    }

    let logo_payload = "logo overlap benchmark";
    let logo_encoded = encode(EncodeRequest {
        text: logo_payload,
        ecc: ErrorCorrection::High,
        max_version: SUPPORTED_PROFILES[3].maximum_version(),
    })
    .expect("logo benchmark payload fits");
    let logo_options = RenderOptions::safe(SUPPORTED_PROFILES[3])
        .expect("safe options are valid")
        .with_logo(LogoStyle::Bundled)
        .expect("logo options are valid");
    group.bench_function("full-request-to-logo-artifacts", |bencher| {
        bencher.iter(|| {
            let model = RenderModel::new(black_box(&logo_encoded), logo_options)
                .expect("logo model is valid");
            black_box((
                render_svg(&model).expect("SVG renders"),
                render_png(&model).expect("PNG renders"),
            ))
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    encoding_benchmarks,
    mask_benchmarks,
    rendering_benchmarks
);
criterion_main!(benches);
