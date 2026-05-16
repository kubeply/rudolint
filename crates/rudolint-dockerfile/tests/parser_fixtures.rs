use rudolint_dockerfile::{Dockerfile, Instruction, parse_dockerfile};
use rudolint_test::read_fixture;
use serde_json::{Value, json};

#[test]
fn snapshots_buildkit_basics() {
    snapshot_parser_fixture("buildkit_basics", "parser/buildkit-basics/Dockerfile");
}

#[test]
fn snapshots_parser_matrix() {
    for (snapshot, fixture) in [
        (
            "simple_instructions",
            "parser/simple-instructions/Dockerfile",
        ),
        ("comments", "parser/comments/Dockerfile"),
        ("directives", "parser/directives/Dockerfile"),
        ("continuations", "parser/continuations/Dockerfile"),
        ("heredocs", "parser/heredocs/Dockerfile"),
        ("run_mount", "parser/run-mount/Dockerfile"),
        ("from_platform", "parser/from-platform/Dockerfile"),
        ("copy_from", "parser/copy-from/Dockerfile"),
    ] {
        snapshot_parser_fixture(snapshot, fixture);
    }
}

fn snapshot_parser_fixture(snapshot: &str, fixture: &str) {
    let source = read_fixture(fixture);
    let document = parse_dockerfile(&source).expect("fixture should parse");
    insta::assert_json_snapshot!(snapshot, document_json(&document));
}

fn document_json(document: &Dockerfile) -> Value {
    json!({
        "syntax": document.syntax.as_ref().map(|syntax| {
            json!({
                "image": syntax.image,
                "line": syntax.line,
            })
        }),
        "has_buildkit_features": document.has_buildkit_features,
        "instructions": document
            .instructions
            .iter()
            .map(instruction_json)
            .collect::<Vec<_>>(),
    })
}

fn instruction_json(instruction: &Instruction) -> Value {
    json!({
        "keyword": instruction.keyword,
        "args": instruction.args,
        "flags": instruction.flags,
        "mounts": instruction.mounts.iter().map(|mount| {
            json!({
                "type": mount.mount_type,
                "options": mount.options,
            })
        }).collect::<Vec<_>>(),
        "heredocs": instruction.heredocs,
        "line": instruction.line,
    })
}
