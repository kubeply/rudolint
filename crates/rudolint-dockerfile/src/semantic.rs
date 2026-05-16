use std::ops::Range;

use crate::Dockerfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub index: usize,
    pub alias: Option<String>,
    pub base_image: String,
    pub platform: Option<String>,
    pub instruction_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgScopes {
    pub global: Vec<ArgValue>,
    pub stages: Vec<StageArgs>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageArgs {
    pub stage_index: usize,
    pub args: Vec<ArgValue>,
    pub inherited: Vec<ArgValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgValue {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvScopes {
    pub stages: Vec<StageEnv>,
    pub final_env: Vec<EnvValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageEnv {
    pub stage_index: usize,
    pub vars: Vec<EnvValue>,
    pub effective: Vec<EnvValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyGraph {
    pub operations: Vec<CopyOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyOperation {
    pub instruction_index: usize,
    pub kind: crate::CopyKind,
    pub sources: Vec<String>,
    pub destination: Option<String>,
    pub from: Option<String>,
}

/// Multi-platform build configuration extracted from a Dockerfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPlatformFacts {
    /// Global `TARGETPLATFORM` ARG default, if declared before the first stage.
    pub targetplatform: Option<String>,
    /// Global `BUILDPLATFORM` ARG default, if declared before the first stage.
    pub buildplatform: Option<String>,
    /// Global `TARGETARCH` ARG value, if declared or inferred.
    pub targetarch: Option<String>,
    /// Global `TARGETOS` ARG value, if declared or inferred.
    pub targetos: Option<String>,
    /// Per-stage platform overrides from `FROM --platform=...` declarations.
    pub stage_platforms: Vec<StagePlatform>,
}

/// Platform override declared for a Dockerfile stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagePlatform {
    /// Zero-based Dockerfile stage number.
    pub stage_index: usize,
    /// Platform string passed to the `FROM --platform=` flag.
    pub platform: String,
}

pub fn stages(document: &Dockerfile) -> Vec<Stage> {
    let from_indexes = document
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, instruction)| instruction.keyword == "FROM")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    from_indexes
        .iter()
        .enumerate()
        .filter_map(|(stage_index, instruction_index)| {
            let instruction = &document.instructions[*instruction_index];
            let from = instruction.from.as_ref()?;
            let end = from_indexes
                .get(stage_index + 1)
                .copied()
                .unwrap_or(document.instructions.len());
            Some(Stage {
                index: stage_index,
                alias: from.alias.clone(),
                base_image: from.image.clone(),
                platform: from.platform.clone(),
                instruction_range: *instruction_index..end,
            })
        })
        .collect()
}

pub fn arg_scopes(document: &Dockerfile) -> ArgScopes {
    let stages = stages(document);
    let first_stage_start = stages
        .first()
        .map(|stage| stage.instruction_range.start)
        .unwrap_or(document.instructions.len());
    let global = document.instructions[..first_stage_start]
        .iter()
        .filter_map(|instruction| instruction.arg.as_ref())
        .map(arg_value)
        .collect::<Vec<_>>();

    let stage_args = stages
        .iter()
        .map(|stage| {
            let args = document.instructions[stage.instruction_range.clone()]
                .iter()
                .filter_map(|instruction| instruction.arg.as_ref())
                .map(arg_value)
                .collect::<Vec<_>>();
            StageArgs {
                stage_index: stage.index,
                args,
                inherited: global.clone(),
            }
        })
        .collect();

    ArgScopes {
        global,
        stages: stage_args,
    }
}

fn arg_value(arg: &crate::ArgInstruction) -> ArgValue {
    ArgValue {
        name: arg.name.clone(),
        default: arg.default.clone(),
    }
}

pub fn env_scopes(document: &Dockerfile) -> EnvScopes {
    let stage_values = stages(document)
        .iter()
        .map(|stage| {
            let vars = document.instructions[stage.instruction_range.clone()]
                .iter()
                .filter_map(|instruction| instruction.env.as_ref())
                .flat_map(|env| env.assignments.iter())
                .map(|assignment| EnvValue {
                    name: assignment.name.clone(),
                    value: assignment.value.clone(),
                })
                .collect::<Vec<_>>();
            StageEnv {
                stage_index: stage.index,
                effective: collapse_env(&vars),
                vars,
            }
        })
        .collect::<Vec<_>>();
    let final_env = stage_values
        .last()
        .map(|stage| stage.effective.clone())
        .unwrap_or_default();

    EnvScopes {
        stages: stage_values,
        final_env,
    }
}

fn collapse_env(values: &[EnvValue]) -> Vec<EnvValue> {
    let mut collapsed = Vec::<EnvValue>::new();
    for value in values {
        if let Some(existing) = collapsed.iter_mut().find(|item| item.name == value.name) {
            existing.value.clone_from(&value.value);
        } else {
            collapsed.push(value.clone());
        }
    }
    collapsed
}

pub fn copy_graph(document: &Dockerfile) -> CopyGraph {
    let operations = document
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(index, instruction)| {
            let copy = instruction.copy.as_ref()?;
            Some(CopyOperation {
                instruction_index: index,
                kind: copy.kind,
                sources: copy.sources.clone(),
                destination: copy.destination.clone(),
                from: copy.from.clone(),
            })
        })
        .collect();

    CopyGraph { operations }
}

pub fn multi_platform_facts(document: &Dockerfile) -> MultiPlatformFacts {
    let arg_scopes = arg_scopes(document);
    let lookup = |name: &str| {
        arg_scopes
            .global
            .iter()
            .rev()
            .find(|arg| arg.name == name)
            .and_then(|arg| arg.default.clone())
    };
    let stage_platforms = stages(document)
        .into_iter()
        .filter_map(|stage| {
            let platform = stage.platform?;
            Some(StagePlatform {
                stage_index: stage.index,
                platform,
            })
        })
        .collect();

    MultiPlatformFacts {
        targetplatform: lookup("TARGETPLATFORM"),
        buildplatform: lookup("BUILDPLATFORM"),
        targetarch: lookup("TARGETARCH"),
        targetos: lookup("TARGETOS"),
        stage_platforms,
    }
}
