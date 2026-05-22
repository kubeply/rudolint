mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_output::{json, sarif};

fn large_findings() -> Vec<rudolint_diagnostics::Finding> {
    let engine = common::default_engine();
    let docs = common::parsed_corpus();
    let base = docs
        .iter()
        .flat_map(|(_, document)| engine.lint(document))
        .collect::<Vec<_>>();

    (0..128)
        .flat_map(|_| base.iter().cloned())
        .collect::<Vec<_>>()
}

fn bench_output_renderers(c: &mut Criterion) {
    let findings = large_findings();

    let mut group = c.benchmark_group("output");
    group.bench_function("json_large_findings", |b| {
        b.iter(|| json(black_box(&findings)).unwrap());
    });
    group.bench_function("sarif_large_findings", |b| {
        b.iter(|| sarif(black_box(&findings)).unwrap());
    });
    group.finish();
}

criterion_group!(benches, bench_output_renderers);
criterion_main!(benches);
