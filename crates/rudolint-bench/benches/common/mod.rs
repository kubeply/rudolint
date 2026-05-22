#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use rudolint_config::Config;
use rudolint_dockerfile::{Dockerfile, parse_dockerfile};
use rudolint_rules::{Profile, RuleEngine};

pub fn workspace_root() -> &'static Path {
    // Two levels up from crates/rudolint-bench to reach the workspace root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root should exist")
}

pub fn fixture_path(path: &str) -> PathBuf {
    workspace_root().join(path)
}

pub fn read_corpus(name: &str) -> String {
    let path = fixture_path(&format!("fixtures/corpus/{name}/Dockerfile"));
    fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("corpus fixture {} should be readable", path.display()))
}

pub fn corpus_sources() -> [(&'static str, String); 4] {
    [
        ("small", read_corpus("small")),
        ("medium_multistage", read_corpus("medium-multistage")),
        ("large_generated", read_corpus("large-generated")),
        ("buildkit_heavy", read_corpus("buildkit-heavy")),
    ]
}

pub fn parsed_corpus() -> Vec<(&'static str, Dockerfile)> {
    corpus_sources()
        .into_iter()
        .map(|(name, source)| (name, parse_dockerfile(&source).unwrap()))
        .collect()
}

pub fn default_engine() -> RuleEngine {
    RuleEngine::new(Profile::Default, Config::default())
}

pub fn repeated_sources(count: usize) -> Vec<String> {
    let sources = corpus_sources();
    (0..count)
        .map(|index| sources[index % sources.len()].1.clone())
        .collect()
}

pub fn release_binary_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUDOLINT_BENCH_BIN") {
        return PathBuf::from(path);
    }

    let executable = if cfg!(windows) {
        "rudolint.exe"
    } else {
        "rudolint"
    };
    workspace_root().join("target/release").join(executable)
}
