mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_dockerfile::parse_dockerfile;

fn bench_many_files(c: &mut Criterion) {
    let sources = common::repeated_sources(300);
    let engine = common::default_engine();

    let mut group = c.benchmark_group("many_files");
    group.bench_function("parse_lint_300", |b| {
        b.iter(|| {
            sources
                .iter()
                .map(|source| match parse_dockerfile(black_box(source)) {
                    Ok(document) => engine.lint(&document),
                    Err(_) => Vec::new(),
                })
                .collect::<Vec<_>>()
        });
    });
    group.finish();
}

criterion_group!(benches, bench_many_files);
criterion_main!(benches);
