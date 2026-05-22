mod common;

use std::process::Command;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn run_rudolint(args: &[&str]) {
    let binary = common::release_binary_path();
    let output = Command::new(&binary)
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("{} should run", binary.display()));

    if !output.status.success() {
        panic!(
            "{} failed with status {}: {}",
            binary.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    black_box(output.stdout);
}

fn bench_cold_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_cold_start");
    group.bench_function("version", |b| {
        b.iter(|| run_rudolint(&["--version"]));
    });
    group.finish();
}

fn bench_cli_lint(c: &mut Criterion) {
    let small = common::fixture_path("fixtures/corpus/small/Dockerfile");
    let directory = common::fixture_path("fixtures/corpus/directory-tree");

    let mut group = c.benchmark_group("cli_lint");
    group.bench_function("one_file_json", |b| {
        b.iter(|| {
            run_rudolint(&[
                "check",
                small.to_str().expect("fixture path should be UTF-8"),
                "--format",
                "json",
                "--failure-threshold",
                "ignore",
            ]);
        });
    });
    group.bench_function("recursive_directory_json", |b| {
        b.iter(|| {
            run_rudolint(&[
                "check",
                directory.to_str().expect("fixture path should be UTF-8"),
                "--format",
                "json",
                "--failure-threshold",
                "ignore",
            ]);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_cold_start, bench_cli_lint);
criterion_main!(benches);
