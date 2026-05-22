mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

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

criterion_group!(benches, bench_lint_one_file);
criterion_main!(benches);
