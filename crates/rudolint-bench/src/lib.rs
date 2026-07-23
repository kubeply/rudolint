//! Benchmark harness support and corpus metadata.

#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

/// Metadata for a built-in benchmark corpus fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkCorpus {
    /// Stable human-readable corpus identifier.
    name: String,
    /// Corpus shape used to decide how benchmark runners discover inputs.
    kind: CorpusKind,
    /// Filesystem location of the corpus fixture.
    path: PathBuf,
}

/// Distinguishes single-file and directory-tree benchmark corpora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusKind {
    /// One Dockerfile at the corpus path.
    SingleFile,
    /// A directory that can contain multiple Dockerfile inputs.
    Directory,
}

/// Returns built-in [`BenchmarkCorpus`] metadata under `root/fixtures/corpus`.
///
/// `root` is the repository or workspace root used to resolve the fixture
/// directory. The returned list is deterministic and includes each corpus name,
/// [`CorpusKind`], and [`PathBuf`].
#[cfg(test)]
fn benchmark_corpora(root: impl AsRef<Path>) -> Vec<BenchmarkCorpus> {
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
        assert_eq!(
            corpora.iter().map(|corpus| corpus.kind).collect::<Vec<_>>(),
            [
                CorpusKind::SingleFile,
                CorpusKind::SingleFile,
                CorpusKind::SingleFile,
                CorpusKind::Directory,
            ]
        );
        assert!(
            corpora[0]
                .path
                .ends_with("fixtures/corpus/small/Dockerfile")
        );
        assert!(
            corpora[1]
                .path
                .ends_with("fixtures/corpus/medium-multistage/Dockerfile")
        );
        assert!(
            corpora[2]
                .path
                .ends_with("fixtures/corpus/large-generated/Dockerfile")
        );
        assert!(corpora[3].path.ends_with("fixtures/corpus/directory-tree"));
    }
}
