//! Container image references, registries, tags, and digests.

/// Parsed container image reference components.
///
/// The parser keeps the original input while exposing registry, repository,
/// tag, digest, and local build-stage information for semantic checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReference {
    /// Original unparsed image reference string.
    raw: String,
    /// Optional registry host, such as `docker.io` or `localhost:5000`.
    registry: Option<String>,
    /// Repository path without registry, tag, or digest.
    pub repository: String,
    /// Optional tag suffix after the final `:`, such as `latest` or `3.20`.
    pub tag: Option<String>,
    /// Optional digest suffix after `@`, such as `sha256:...`.
    digest: Option<String>,
    /// Whether the parsed image name matches a known local build stage name.
    local_stage_reference: bool,
}

impl ImageReference {
    /// Parses an image reference into its semantic components.
    ///
    /// Splits `raw` into digest, tag, registry, and repository parts, and marks
    /// the result as local when `raw` matches an entry in `local_stages`.
    ///
    /// ```
    /// use rudolint_image::ImageReference;
    ///
    /// let reference = ImageReference::parse("alpine:3.20", &[]);
    /// assert_eq!(reference.repository, "alpine");
    /// assert_eq!(reference.tag.as_deref(), Some("3.20"));
    /// ```
    pub fn parse(raw: &str, local_stages: &[String]) -> Self {
        let (name, digest) = raw.split_once('@').map_or((raw, None), |(name, digest)| {
            (name, Some(digest.to_string()))
        });
        let (name, tag) = split_tag(name);
        let (registry, repository) = split_registry(name);
        Self {
            raw: raw.to_string(),
            registry: registry.map(str::to_string),
            repository: repository.to_string(),
            tag: tag.map(str::to_string),
            digest,
            local_stage_reference: local_stages.iter().any(|stage| stage == name),
        }
    }
}

fn split_registry(name: &str) -> (Option<&str>, &str) {
    let Some((first, rest)) = name.split_once('/') else {
        return (None, name);
    };
    if first.contains('.') || first.contains(':') || first == "localhost" {
        (Some(first), rest)
    } else {
        (None, name)
    }
}

fn split_tag(name: &str) -> (&str, Option<&str>) {
    let Some((prefix, suffix)) = name.rsplit_once(':') else {
        return (name, None);
    };
    if suffix.contains('/') {
        (name, None)
    } else {
        (prefix, Some(suffix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn snapshots_image_reference_parsing() {
        let local_stages = vec!["build".to_string()];
        let references = [
            "alpine:3.20",
            "docker.io/library/alpine:latest",
            "ghcr.io/kubeply/rudolint@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "localhost:5000/team/image:dev",
            "build",
            "build:v1",
        ]
        .iter()
        .map(|raw| {
            let reference = ImageReference::parse(raw, &local_stages);
            json!({
                "raw": reference.raw,
                "registry": reference.registry,
                "repository": reference.repository,
                "tag": reference.tag,
                "digest": reference.digest,
                "local_stage_reference": reference.local_stage_reference,
            })
        })
        .collect::<Vec<_>>();

        insta::assert_json_snapshot!(references);
    }
}
