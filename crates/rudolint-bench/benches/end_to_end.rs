mod common;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_dockerfile::parse_dockerfile;

fn bench_parse_and_lint(c: &mut Criterion) {
    let sources = common::corpus_sources();
    let engine = common::default_engine();

    let mut group = c.benchmark_group("parse_and_lint");
    for (name, source) in &sources {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let document = parse_dockerfile(black_box(source)).expect(
                    "parse_dockerfile only errors on regex compilation \
                    (heredoc_delimiters/parse_heredocs), not on fixture content",
                );
                engine.lint(&document)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_parse_and_lint);
criterion_main!(benches);
