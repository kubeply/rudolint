//! Benchmark harness support and corpus metadata.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkCorpus {
    pub name: String,
    pub kind: CorpusKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusKind {
    SingleFile,
    Directory,
}

pub fn benchmark_corpora(root: impl AsRef<Path>) -> Vec<BenchmarkCorpus> {
    let root = root.as_ref().join("fixtures/corpus");
    vec![
        BenchmarkCorpus {
            name: "small".to_string(),
            kind: CorpusKind::SingleFile,
            path: root.join("small/Dockerfile"),
        },
        BenchmarkCorpus {
            name: "medium-multistage".to_string(),
            kind: CorpusKind::SingleFile,
            path: root.join("medium-multistage/Dockerfile"),
        },
        BenchmarkCorpus {
            name: "large-generated".to_string(),
            kind: CorpusKind::SingleFile,
            path: root.join("large-generated/Dockerfile"),
        },
        BenchmarkCorpus {
            name: "directory-tree".to_string(),
            kind: CorpusKind::Directory,
            path: root.join("directory-tree"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_metadata_is_stable() {
        let corpora = benchmark_corpora("/workspace");

        assert_eq!(
            corpora
                .iter()
                .map(|corpus| corpus.name.as_str())
                .collect::<Vec<_>>(),
            [
                "small",
                "medium-multistage",
                "large-generated",
                "directory-tree"
            ]
        );
        assert_eq!(corpora[0].kind, CorpusKind::SingleFile);
        assert_eq!(corpora[3].kind, CorpusKind::Directory);
    }
}
