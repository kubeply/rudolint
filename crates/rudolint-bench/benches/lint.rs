mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_dockerfile::parse_dockerfile;

fn bench_lint_one_file(c: &mut Criterion) {
    let docs = common::parsed_corpus();
    let engine = common::default_engine();

    let mut group = c.benchmark_group("lint_one_file");
    for (name, doc) in &docs {
        group.bench_function(*name, |b| {
            b.iter(|| engine.lint(black_box(doc)));
        });
    }
    group.finish();
}

fn bench_parse_and_lint(c: &mut Criterion) {
    let sources = common::corpus_sources();
    let engine = common::default_engine();

    let mut group = c.benchmark_group("parse_and_lint");
    for (name, source) in &sources {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let document = parse_dockerfile(black_box(source)).unwrap();
                engine.lint(&document)
            });
        });
    }
    group.finish();
}

fn bench_many_files(c: &mut Criterion) {
    let sources = common::repeated_sources(1_000);
    let engine = common::default_engine();

    let mut group = c.benchmark_group("many_files");
    group.bench_function("parse_lint_1000", |b| {
        b.iter(|| {
            sources
                .iter()
                .map(|source| {
                    let document = parse_dockerfile(black_box(source)).unwrap();
                    engine.lint(&document)
                })
                .collect::<Vec<_>>()
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_lint_one_file,
    bench_parse_and_lint,
    bench_many_files
);
criterion_main!(benches);
