use std::path::Path;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rudolint_config::Config;
use rudolint_dockerfile::parse_dockerfile;
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

// ---------------------------------------------------------------------------
// Parsing benchmarks
// ---------------------------------------------------------------------------

fn bench_parse(c: &mut Criterion) {
    let small = read_corpus("small");
    let medium = read_corpus("medium-multistage");
    let large = read_corpus("large-generated");

    let mut group = c.benchmark_group("parse");
    group.bench_function("small", |b| {
        b.iter(|| parse_dockerfile(black_box(&small)).unwrap());
    });
    group.bench_function("medium_multistage", |b| {
        b.iter(|| parse_dockerfile(black_box(&medium)).unwrap());
    });
    group.bench_function("large_generated", |b| {
        b.iter(|| parse_dockerfile(black_box(&large)).unwrap());
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Linting benchmarks
// ---------------------------------------------------------------------------

fn bench_lint(c: &mut Criterion) {
    let small_src = read_corpus("small");
    let medium_src = read_corpus("medium-multistage");
    let large_src = read_corpus("large-generated");

    let small_doc = parse_dockerfile(&small_src).unwrap();
    let medium_doc = parse_dockerfile(&medium_src).unwrap();
    let large_doc = parse_dockerfile(&large_src).unwrap();

    let engine = RuleEngine::new(Profile::Default, Config::default());

    let mut group = c.benchmark_group("lint");
    group.bench_function("small", |b| {
        b.iter(|| engine.lint(black_box(&small_doc)));
    });
    group.bench_function("medium_multistage", |b| {
        b.iter(|| engine.lint(black_box(&medium_doc)));
    });
    group.bench_function("large_generated", |b| {
        b.iter(|| engine.lint(black_box(&large_doc)));
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// End-to-end benchmarks (parse + lint)
// ---------------------------------------------------------------------------

fn bench_end_to_end(c: &mut Criterion) {
    let small_src = read_corpus("small");
    let medium_src = read_corpus("medium-multistage");
    let large_src = read_corpus("large-generated");

    let mut group = c.benchmark_group("end_to_end");
    group.bench_function("small", |b| {
        b.iter(|| {
            let doc = parse_dockerfile(black_box(&small_src)).unwrap();
            let engine = RuleEngine::new(Profile::Default, Config::default());
            engine.lint(&doc)
        });
    });
    group.bench_function("medium_multistage", |b| {
        b.iter(|| {
            let doc = parse_dockerfile(black_box(&medium_src)).unwrap();
            let engine = RuleEngine::new(Profile::Default, Config::default());
            engine.lint(&doc)
        });
    });
    group.bench_function("large_generated", |b| {
        b.iter(|| {
            let doc = parse_dockerfile(black_box(&large_src)).unwrap();
            let engine = RuleEngine::new(Profile::Default, Config::default());
            engine.lint(&doc)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_parse, bench_lint, bench_end_to_end);
criterion_main!(benches);
