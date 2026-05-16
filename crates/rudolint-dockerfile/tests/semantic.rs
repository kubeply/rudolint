use rudolint_dockerfile::multi_platform_facts;
use rudolint_dockerfile::{CopyKind, arg_scopes, copy_graph, env_scopes, parse_dockerfile, stages};
use rudolint_test::read_fixture;
use serde_json::json;

#[test]
fn snapshots_stage_model() {
    let source = read_fixture("parser/buildkit-basics/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let values = stages(&document)
        .iter()
        .map(|stage| {
            json!({
                "index": stage.index,
                "alias": stage.alias,
                "base_image": stage.base_image,
                "platform": stage.platform,
                "instruction_range": {
                    "start": stage.instruction_range.start,
                    "end": stage.instruction_range.end,
                },
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(values);
}

#[test]
fn snapshots_arg_scope_model() {
    let source = read_fixture("parser/arg/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let scopes = arg_scopes(&document);

    insta::assert_json_snapshot!(json!({
        "global": scopes.global.iter().map(|arg| {
            json!({
                "name": arg.name,
                "default": arg.default,
            })
        }).collect::<Vec<_>>(),
        "stages": scopes.stages.iter().map(|stage| {
            json!({
                "stage_index": stage.stage_index,
                "args": stage.args.iter().map(|arg| {
                    json!({
                        "name": arg.name,
                        "default": arg.default,
                    })
                }).collect::<Vec<_>>(),
                "inherited": stage.inherited.iter().map(|arg| {
                    json!({
                        "name": arg.name,
                        "default": arg.default,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    }));
}

#[test]
fn snapshots_env_model() {
    let source = read_fixture("parser/env/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let scopes = env_scopes(&document);

    insta::assert_json_snapshot!(json!({
        "stages": scopes.stages.iter().map(|stage| {
            json!({
                "stage_index": stage.stage_index,
                "vars": env_values_json(&stage.vars),
                "effective": env_values_json(&stage.effective),
            })
        }).collect::<Vec<_>>(),
        "final_env": env_values_json(&scopes.final_env),
    }));
}

fn env_values_json(values: &[rudolint_dockerfile::EnvValue]) -> Vec<serde_json::Value> {
    values
        .iter()
        .map(|value| {
            json!({
                "name": value.name,
                "value": value.value,
            })
        })
        .collect()
}

#[test]
fn snapshots_copy_graph() {
    let source = read_fixture("parser/copy-from/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let graph = copy_graph(&document);

    insta::assert_json_snapshot!(
        graph
            .operations
            .iter()
            .map(|operation| {
                json!({
                    "instruction_index": operation.instruction_index,
                    "kind": match operation.kind {
                        CopyKind::Copy => "copy",
                        CopyKind::Add => "add",
                    },
                    "sources": operation.sources,
                    "destination": operation.destination,
                    "from": operation.from,
                })
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn snapshots_multi_platform_facts() {
    let source = read_fixture("parser/from-platform/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let facts = multi_platform_facts(&document);

    insta::assert_json_snapshot!(json!({
        "targetplatform": facts.targetplatform,
        "buildplatform": facts.buildplatform,
        "targetarch": facts.targetarch,
        "targetos": facts.targetos,
        "stage_platforms": facts.stage_platforms.iter().map(|platform| {
            json!({
                "stage_index": platform.stage_index,
                "platform": platform.platform,
            })
        }).collect::<Vec<_>>(),
    }));
}
