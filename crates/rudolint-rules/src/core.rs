use std::collections::BTreeSet;

use crate::{Rule, RuleInfo, RuleStatus};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Comment, Dockerfile, Instruction};
use rudolint_policy::{LegacySuppression, PolicyProfile};

macro_rules! rule {
    ($name:ident, $code:literal, $severity:expr, $summary:literal, $body:expr) => {
        struct $name;
        impl Rule for $name {
            fn info(&self) -> RuleInfo {
                RuleInfo {
                    code: $code,
                    severity: $severity,
                    summary: $summary,
                    status: RuleStatus::Implemented,
                }
            }

            fn check(&self, document: &Dockerfile) -> Vec<Finding> {
                $body(document)
            }
        }
    };
}

pub fn implemented_rules(profile: PolicyProfile) -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = vec![
        Box::new(InlineIgnore),
        Box::new(AbsoluteWorkdir),
        Box::new(LastUserNotRoot),
        Box::new(ExplicitFromTag),
        Box::new(NoLatestTag),
        Box::new(ValidExposePort),
        Box::new(SingleHealthcheck),
        Box::new(PreferCopy),
        Box::new(UniqueStageNames),
        Box::new(JsonEntrypoints),
        Box::new(DeprecatedMaintainer),
        Box::new(SingleCmd),
        Box::new(SingleEntrypoint),
    ];

    if profile.includes_buildkit_native_rules() {
        rules.extend([
            Box::new(BuildkitSyntaxWhenFeaturesUsed) as Box<dyn Rule>,
            Box::new(SecretLikeArgOrEnv),
            Box::new(SecretInRun),
            Box::new(CacheMountForPackageInstall),
        ]);
    }

    rules
}

pub fn catalog(profile: PolicyProfile) -> Vec<RuleInfo> {
    let mut rules = implemented_rules(profile)
        .into_iter()
        .map(|rule| rule.info())
        .collect::<Vec<_>>();

    if profile.includes_compatibility_rules() {
        rules.extend(planned_compat_rules().into_iter().map(|code| RuleInfo {
            code,
            severity: Severity::Warning,
            summary: "tracked for compatibility parity",
            status: RuleStatus::Planned,
        }));
    }

    if profile.includes_shell_catalog() {
        rules.extend(shell_rule_catalog().into_iter().map(|code| RuleInfo {
            code,
            severity: Severity::Warning,
            summary: "shell diagnostics delegated to the shell-analysis layer",
            status: RuleStatus::External,
        }));
    }

    rules.sort_by_key(|rule| rule.code);
    rules.dedup_by_key(|rule| rule.code);
    rules
}

fn diagnostic(
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    instruction: &Instruction,
) -> Finding {
    Finding::new(code, severity, message, instruction.line, 1)
}

rule!(
    InlineIgnore,
    "RDL1001",
    Severity::Warning,
    "warn on legacy external linter suppression comments",
    |doc: &Dockerfile| {
        doc.comments
            .iter()
            .filter_map(legacy_suppression_comment)
            .collect()
    }
);

fn legacy_suppression_comment(comment: &Comment) -> Option<Finding> {
    LegacySuppression::parse_comment(comment.line, &comment.text)?;
    Some(Finding::with_span(
        "RDL1001",
        Severity::Warning,
        "prefer native rudolint suppression comments over legacy external suppressions",
        comment.span,
    ))
}

rule!(
    AbsoluteWorkdir,
    "RDL3000",
    Severity::Error,
    "require absolute WORKDIR paths",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "WORKDIR")
            .filter(|instruction| {
                let path = instruction.args.trim_matches('"').trim_matches('\'');
                !(path.starts_with('/') || path.starts_with('$') || is_windows_absolute(path))
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3000",
                    Severity::Error,
                    "WORKDIR should be absolute",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    LastUserNotRoot,
    "RDL3002",
    Severity::Warning,
    "require the final USER to be non-root",
    |doc: &Dockerfile| {
        let Some(last_user) = doc
            .instructions
            .iter()
            .rev()
            .find(|instruction| instruction.keyword == "USER")
        else {
            return Vec::new();
        };
        let user = last_user.args.trim();
        if matches!(user, "root" | "0" | "0:0") {
            vec![diagnostic(
                "RDL3002",
                Severity::Warning,
                "the final image user should not be root",
                last_user,
            )]
        } else {
            Vec::new()
        }
    }
);

rule!(
    ExplicitFromTag,
    "RDL3006",
    Severity::Warning,
    "require explicit image tags in FROM",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "FROM")
            .filter(|instruction| {
                instruction.base_image().is_some_and(|image| {
                    !image.contains('@') && !image.rsplit('/').next().unwrap_or("").contains(':')
                })
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3006",
                    Severity::Warning,
                    "base image should use an explicit tag or digest",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    NoLatestTag,
    "RDL3007",
    Severity::Warning,
    "reject latest base image tags",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "FROM")
            .filter(|instruction| {
                instruction.base_image().is_some_and(|image| {
                    image.rsplit('/').next().unwrap_or("").ends_with(":latest")
                })
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3007",
                    Severity::Warning,
                    "avoid mutable latest base image tags",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    ValidExposePort,
    "RDL3011",
    Severity::Error,
    "validate EXPOSE port numbers",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "EXPOSE")
            .flat_map(|instruction| {
                instruction
                    .args
                    .split_whitespace()
                    .filter_map(|port| {
                        let port = port.split('/').next().unwrap_or(port);
                        let valid = port.parse::<u32>().is_ok_and(|value| value <= 65535);
                        (!valid).then(|| {
                            diagnostic(
                                "RDL3011",
                                Severity::Error,
                                format!("invalid exposed port `{port}`"),
                                instruction,
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
);

rule!(
    SingleHealthcheck,
    "RDL3012",
    Severity::Error,
    "allow only one HEALTHCHECK instruction",
    |doc: &Dockerfile| duplicates(
        doc,
        "HEALTHCHECK",
        "RDL3012",
        Severity::Error,
        "only one HEALTHCHECK is allowed"
    )
);

rule!(
    PreferCopy,
    "RDL3020",
    Severity::Error,
    "prefer COPY for plain local files",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "ADD")
            .filter(|instruction| {
                let source = instruction.args.split_whitespace().next().unwrap_or("");
                !(source.starts_with("http://")
                    || source.starts_with("https://")
                    || source.ends_with(".tar")
                    || source.ends_with(".tar.gz")
                    || source.ends_with(".tgz"))
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3020",
                    Severity::Error,
                    "use COPY for local files unless archive extraction or remote fetch is intended",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    UniqueStageNames,
    "RDL3024",
    Severity::Error,
    "require unique multi-stage aliases",
    |doc: &Dockerfile| {
        let mut seen = BTreeSet::new();
        let mut findings = Vec::new();
        for instruction in &doc.instructions {
            let Some(alias) = instruction.stage_alias() else {
                continue;
            };
            if !seen.insert(alias.clone()) {
                findings.push(diagnostic(
                    "RDL3024",
                    Severity::Error,
                    format!("stage alias `{alias}` is defined more than once"),
                    instruction,
                ));
            }
        }
        findings
    }
);

rule!(
    JsonEntrypoints,
    "RDL3025",
    Severity::Warning,
    "prefer JSON form for CMD and ENTRYPOINT",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| matches!(instruction.keyword.as_str(), "CMD" | "ENTRYPOINT"))
            .filter(|instruction| !instruction.args.trim_start().starts_with('['))
            .map(|instruction| {
                diagnostic(
                    "RDL3025",
                    Severity::Warning,
                    "use exec/JSON form for CMD and ENTRYPOINT",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    DeprecatedMaintainer,
    "RDL4000",
    Severity::Error,
    "reject deprecated MAINTAINER instructions",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "MAINTAINER")
            .map(|instruction| {
                diagnostic(
                    "RDL4000",
                    Severity::Error,
                    "use OCI labels instead of MAINTAINER",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    SingleCmd,
    "RDL4003",
    Severity::Warning,
    "allow only one CMD instruction",
    |doc: &Dockerfile| duplicates(
        doc,
        "CMD",
        "RDL4003",
        Severity::Warning,
        "only the final CMD is used"
    )
);

rule!(
    SingleEntrypoint,
    "RDL4004",
    Severity::Error,
    "allow only one ENTRYPOINT instruction",
    |doc: &Dockerfile| duplicates(
        doc,
        "ENTRYPOINT",
        "RDL4004",
        Severity::Error,
        "only the final ENTRYPOINT is used"
    )
);

rule!(
    BuildkitSyntaxWhenFeaturesUsed,
    "RDK1000",
    Severity::Info,
    "require explicit syntax directive for BuildKit-only features",
    |doc: &Dockerfile| {
        if doc.has_buildkit_features && doc.syntax.is_none() {
            let Some(instruction) = doc
                .instructions
                .iter()
                .find(|instruction| instruction.has_buildkit_features())
            else {
                return Vec::new();
            };
            vec![diagnostic(
                "RDK1000",
                Severity::Info,
                "BuildKit features are used without an explicit # syntax directive",
                instruction,
            )]
        } else {
            Vec::new()
        }
    }
);

rule!(
    SecretLikeArgOrEnv,
    "RDK1001",
    Severity::Warning,
    "reject secret-like ARG and ENV names",
    |doc: &Dockerfile| {
        let secret_words = ["SECRET", "TOKEN", "PASSWORD", "PRIVATE_KEY", "ACCESS_KEY"];
        doc.instructions
            .iter()
            .filter(|instruction| matches!(instruction.keyword.as_str(), "ARG" | "ENV"))
            .filter(|instruction| {
                let upper = instruction.args.to_ascii_uppercase();
                secret_words.iter().any(|word| upper.contains(word))
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1001",
                    Severity::Warning,
                    "secret-like values should use BuildKit secret mounts instead of ARG or ENV",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    SecretInRun,
    "RDK1002",
    Severity::Warning,
    "prefer BuildKit secret mounts for secret-consuming RUN steps",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                let upper = instruction.args.to_ascii_uppercase();
                upper.contains("TOKEN=") || upper.contains("PASSWORD=") || upper.contains("SECRET=")
            })
            .filter(|instruction| {
                !instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.mount_type == "secret")
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1002",
                    Severity::Warning,
                    "RUN appears to pass a secret without a type=secret mount",
                    instruction,
                )
            })
            .collect()
    }
);

rule!(
    CacheMountForPackageInstall,
    "RDK1003",
    Severity::Info,
    "prefer BuildKit cache mounts for package-manager caches",
    |doc: &Dockerfile| {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                instruction.args.contains("apt-get install")
                    || instruction.args.contains("apk add")
                    || instruction.args.contains("dnf install")
                    || instruction.args.contains("yum install")
            })
            .filter(|instruction| {
                !instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.mount_type == "cache")
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1003",
                    Severity::Info,
                    "package install step can use a BuildKit cache mount for repeat builds",
                    instruction,
                )
            })
            .collect()
    }
);

fn duplicates(
    doc: &Dockerfile,
    keyword: &str,
    code: &'static str,
    severity: Severity,
    message: &'static str,
) -> Vec<Finding> {
    let mut seen = false;
    let mut findings = Vec::new();
    for instruction in doc
        .instructions
        .iter()
        .filter(|instruction| instruction.keyword == keyword)
    {
        if seen {
            findings.push(diagnostic(code, severity, message, instruction));
        }
        seen = true;
    }
    findings
}

fn is_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\'
}

fn planned_compat_rules() -> Vec<&'static str> {
    vec![
        "RDL3001", "RDL3003", "RDL3004", "RDL3008", "RDL3009", "RDL3010", "RDL3013", "RDL3014",
        "RDL3015", "RDL3016", "RDL3018", "RDL3019", "RDL3021", "RDL3022", "RDL3023", "RDL3026",
        "RDL3027", "RDL3028", "RDL3029", "RDL3030", "RDL3032", "RDL3033", "RDL3034", "RDL3035",
        "RDL3036", "RDL3037", "RDL3038", "RDL3040", "RDL3041", "RDL3042", "RDL3043", "RDL3044",
        "RDL3045", "RDL3046", "RDL3047", "RDL3048", "RDL3049", "RDL3050", "RDL3051", "RDL3052",
        "RDL3053", "RDL3054", "RDL3055", "RDL3056", "RDL3057", "RDL3058", "RDL3059", "RDL3060",
        "RDL3061", "RDL3062", "RDL3063", "RDL4001", "RDL4005", "RDL4006",
    ]
}

fn shell_rule_catalog() -> Vec<&'static str> {
    vec![
        "RSC1000", "RSC1001", "RSC1007", "RSC1010", "RSC1018", "RSC1035", "RSC1045", "RSC1065",
        "RSC1066", "RSC1077", "RSC1078", "RSC1079", "RSC1081", "RSC1083", "RSC1086", "RSC1095",
        "RSC2002", "RSC2015", "RSC2026", "RSC2035", "RSC2046", "RSC2086", "RSC2140", "RSC2154",
        "RSC2155", "RSC2164", "RSC2181", "RSC2196",
    ]
}
