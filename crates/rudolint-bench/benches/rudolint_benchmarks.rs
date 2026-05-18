use std::path::Path;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_config::Config;
use rudolint_dockerfile::{Dockerfile, parse_dockerfile};
use rudolint_output::{json, sarif};
use rudolint_rules::{Profile, RuleEngine};

fn workspace_root() -> &'static Path {
    // Two levels up from crates/rudolint-bench to reach the workspace root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should exist")
}

fn read_corpus(name: &str) -> String {
    let path = workspace_root()
        .join("fixtures/corpus")
        .join(name)
        .join("Dockerfile");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("corpus fixture {} should be readable", path.display()))
}

fn corpus_sources() -> [(&'static str, String); 4] {
    [
        ("small", read_corpus("small")),
        ("medium_multistage", read_corpus("medium-multistage")),
        ("large_generated", read_corpus("large-generated")),
        ("buildkit_heavy", read_corpus("buildkit-heavy")),
    ]
}

fn parsed_corpus() -> Vec<(&'static str, Dockerfile)> {
    corpus_sources()
        .into_iter()
        .map(|(name, source)| (name, parse_dockerfile(&source).unwrap()))
        .collect()
}

// ---------------------------------------------------------------------------
// Parsing benchmarks
// ---------------------------------------------------------------------------

fn bench_parse(c: &mut Criterion) {
    let sources = corpus_sources();

    let mut group = c.benchmark_group("parse");
    for (name, source) in &sources {
        group.bench_function(*name, |b| {
            b.iter(|| parse_dockerfile(black_box(source)).unwrap());
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Linting benchmarks
// ---------------------------------------------------------------------------

fn bench_lint(c: &mut Criterion) {
    let docs = parsed_corpus();
    let engine = RuleEngine::new(Profile::Default, Config::default());

    let mut group = c.benchmark_group("lint");
    for (name, doc) in &docs {
        group.bench_function(*name, |b| {
            b.iter(|| engine.lint(black_box(doc)));
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// End-to-end benchmarks (parse + lint)
// ---------------------------------------------------------------------------

fn bench_end_to_end(c: &mut Criterion) {
    let sources = corpus_sources();
    let engine = RuleEngine::new(Profile::Default, Config::default());

    let mut group = c.benchmark_group("end_to_end");
    for (name, source) in &sources {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let doc = parse_dockerfile(black_box(source)).unwrap();
                engine.lint(&doc)
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Batch and output benchmarks
// ---------------------------------------------------------------------------

fn repeated_sources(count: usize) -> Vec<String> {
    let sources = corpus_sources();
    (0..count)
        .map(|index| sources[index % sources.len()].1.clone())
        .collect()
}

fn bench_many_files(c: &mut Criterion) {
    let sources = repeated_sources(1_000);
    let engine = RuleEngine::new(Profile::Default, Config::default());

    let mut group = c.benchmark_group("many_files");
    group.bench_function("parse_lint_1000", |b| {
        b.iter(|| {
            sources
                .iter()
                .map(|source| {
                    let doc = parse_dockerfile(black_box(source)).unwrap();
                    engine.lint(&doc)
                })
                .collect::<Vec<_>>()
        });
    });
    group.finish();
}

fn bench_output(c: &mut Criterion) {
    let engine = RuleEngine::new(Profile::Default, Config::default());
    let docs = parsed_corpus();
    let findings = docs
        .iter()
        .flat_map(|(_, doc)| engine.lint(doc))
        .collect::<Vec<_>>();

    let mut group = c.benchmark_group("output");
    group.bench_function("json", |b| {
        b.iter(|| json(black_box(&findings)).unwrap());
    });
    group.bench_function("sarif", |b| {
        b.iter(|| sarif(black_box(&findings)).unwrap());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_lint,
    bench_end_to_end,
    bench_many_files,
    bench_output
);
criterion_main!(benches);
