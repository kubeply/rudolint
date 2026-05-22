mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_dockerfile::parse_dockerfile;

fn bench_parse(c: &mut Criterion) {
    let sources = common::corpus_sources();

    let mut group = c.benchmark_group("parse");
    for (name, source) in &sources {
        group.bench_function(*name, |b| {
            b.iter(|| parse_dockerfile(black_box(source)).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
