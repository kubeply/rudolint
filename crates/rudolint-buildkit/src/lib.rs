//! BuildKit frontend, mount, entitlement, and Buildx semantic analysis.

mod semantics;

#[cfg(test)]
use rudolint_dockerfile::{Dockerfile, Mount};

pub use semantics::{
    FrontendRequirement, FrontendVersion, TARGET_PLATFORM_VARIABLES,
    final_stage_uses_build_platform, frontend_requirements, frontend_version_is_too_old,
    has_multi_platform_intent, has_secret_like_arg_or_env_name, is_official_dockerfile_frontend,
    missing_buildkit_entitlements, parse_pinned_frontend_version, run_copies_secret_mount,
    run_uses_host_architecture_probe, run_uses_lock_based_package_manager_with_shared_cache,
    ssh_mount_scope_is_broad,
};

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Frontend {
    image: String,
    version: Option<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BuildkitFeatures {
    frontend: Option<Frontend>,
    mounts: Vec<Mount>,
    heredoc_count: usize,
    network_modes: Vec<String>,
    security_modes: Vec<String>,
}

#[cfg(test)]
fn analyze(document: &Dockerfile) -> BuildkitFeatures {
    let frontend = document.syntax.as_ref().map(|syntax| Frontend {
        image: syntax.image.clone(),
        version: syntax
            .image
            .rsplit_once(':')
            .map(|(_, version)| version.to_string()),
    });
    let mut mounts = Vec::new();
    let mut heredoc_count = 0;
    let mut network_modes = Vec::new();
    let mut security_modes = Vec::new();

    for instruction in &document.instructions {
        mounts.extend(instruction.mounts.clone());
        heredoc_count += instruction.heredocs.len();
        if let Some(run) = &instruction.run {
            if let Some(network) = &run.network {
                network_modes.push(network.clone());
            }
            if let Some(security) = &run.security {
                security_modes.push(security.clone());
            }
        }
    }

    BuildkitFeatures {
        frontend,
        mounts,
        heredoc_count,
        network_modes,
        security_modes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rudolint_dockerfile::parse_dockerfile;
    use rudolint_test::read_fixture;
    use serde_json::json;

    #[test]
    fn snapshots_buildkit_features() {
        let source = read_fixture("parser/buildkit-basics/Dockerfile");
        let document = parse_dockerfile(&source).expect("fixture should parse");
        let features = analyze(&document);

        insta::assert_json_snapshot!(json!({
            "frontend": features.frontend.as_ref().map(|frontend| {
                json!({
                    "image": frontend.image,
                    "version": frontend.version,
                })
            }),
            "mounts": features.mounts.iter().map(|mount| {
                json!({
                    "type": mount.mount_type,
                    "options": mount.options,
                })
            }).collect::<Vec<_>>(),
            "heredoc_count": features.heredoc_count,
            "network_modes": features.network_modes,
            "security_modes": features.security_modes,
        }));
    }
}
