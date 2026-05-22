mod common;

use std::process::Command;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn run_rudolint(args: &[&str]) -> Result<(), String> {
    let binary = common::release_binary_path();
    if !binary.exists() {
        return Err(format!("{} is not available", binary.display()));
    }

    let output = Command::new(&binary)
        .args(args)
        .output()
        .map_err(|error| format!("{} should run: {error}", binary.display()))?;

    if !output.status.success() {
        return Err(format!(
            "{} failed with status {}: {}",
            binary.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    black_box(output.stdout);
    Ok(())
}

fn should_register_benchmark(args: &[&str], label: &str) -> bool {
    match run_rudolint(args) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("skipping {label}: {error}");
            false
        }
    }
}

fn bench_cold_start(c: &mut Criterion) {
    if !should_register_benchmark(&["--version"], "cli_cold_start/version") {
        return;
    }

    let mut group = c.benchmark_group("cli_cold_start");
    group.bench_function("version", |b| {
        b.iter(|| {
            if let Err(error) = run_rudolint(&["--version"]) {
                black_box(error);
            }
        });
    });
    group.finish();
}

fn bench_cli_lint(c: &mut Criterion) {
    let small = common::fixture_path("fixtures/corpus/small/Dockerfile");
    let directory = common::fixture_path("fixtures/corpus/directory-tree");
    let Some(small_path) = small.to_str() else {
        eprintln!(
            "skipping cli_lint: non-UTF-8 fixture path {}",
            small.display()
        );
        return;
    };
    let Some(directory_path) = directory.to_str() else {
        eprintln!(
            "skipping cli_lint: non-UTF-8 fixture path {}",
            directory.display()
        );
        return;
    };

    let mut group = c.benchmark_group("cli_lint");

    let one_file_args = [
        "check",
        small_path,
        "--format",
        "json",
        "--failure-threshold",
        "ignore",
    ];
    if should_register_benchmark(&one_file_args, "cli_lint/one_file_json") {
        group.bench_function("one_file_json", |b| {
            b.iter(|| {
                if let Err(error) = run_rudolint(&one_file_args) {
                    black_box(error);
                }
            });
        });
    }

    let directory_args = [
        "check",
        directory_path,
        "--format",
        "json",
        "--failure-threshold",
        "ignore",
    ];
    if should_register_benchmark(&directory_args, "cli_lint/recursive_directory_json") {
        group.bench_function("recursive_directory_json", |b| {
            b.iter(|| {
                if let Err(error) = run_rudolint(&directory_args) {
                    black_box(error);
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_cold_start, bench_cli_lint);
criterion_main!(benches);
