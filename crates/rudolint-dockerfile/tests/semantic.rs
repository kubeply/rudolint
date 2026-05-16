use rudolint_dockerfile::{arg_scopes, env_scopes, parse_dockerfile, stages};
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
