use rudolint_dockerfile::{
    CopyKind, Dockerfile, EnvForm, Instruction, InstructionForm, RecoveryKind, parse_dockerfile,
};
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
        ("windows_escape", "parser/windows-escape/Dockerfile"),
        ("json_form", "parser/json-form/Dockerfile"),
        ("invalid_json_form", "parser/invalid-json-form/Dockerfile"),
        ("arg", "parser/arg/Dockerfile"),
        ("env", "parser/env/Dockerfile"),
        ("label", "parser/label/Dockerfile"),
        ("expose", "parser/expose/Dockerfile"),
        ("recovery", "parser/recovery/Dockerfile"),
        ("healthcheck", "parser/healthcheck/Dockerfile"),
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
        "escape": document.escape.as_ref().map(|escape| {
            json!({
                "character": escape.character.to_string(),
                "line": escape.line,
            })
        }),
        "checks": document.checks.iter().map(|check| {
            json!({
                "value": check.value,
                "line": check.line,
            })
        }).collect::<Vec<_>>(),
        "comments": document.comments.iter().map(|comment| {
            json!({
                "text": comment.text,
                "line": comment.line,
                "span": comment.span,
            })
        }).collect::<Vec<_>>(),
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
        "keyword_span": instruction.keyword_span,
        "args": instruction.args,
        "args_span": instruction.args_span,
        "form": instruction_form_json(&instruction.form),
        "continuations": instruction.continuations.iter().map(|continuation| {
            json!({
                "line": continuation.line,
                "escape": continuation.escape.to_string(),
                "span": continuation.span,
            })
        }).collect::<Vec<_>>(),
        "flags": instruction.flags,
        "mounts": instruction.mounts.iter().map(|mount| {
            json!({
                "type": mount.mount_type,
                "options": mount.options,
            })
        }).collect::<Vec<_>>(),
        "heredocs": instruction.heredocs.iter().map(|heredoc| {
            json!({
                "delimiter": heredoc.delimiter,
                "quoted": heredoc.quoted,
                "target_instruction": heredoc.target_instruction,
                "body": heredoc.body,
                "body_span": heredoc.body_span,
            })
        }).collect::<Vec<_>>(),
        "from": instruction.from.as_ref().map(|from| {
            json!({
                "image": from.image,
                "digest": from.digest,
                "alias": from.alias,
                "platform": from.platform,
                "flags": from.flags,
            })
        }),
        "run": instruction.run.as_ref().map(|run| {
            json!({
                "flags": run.flags,
                "mounts": run.mounts.iter().map(|mount| {
                    json!({
                        "type": mount.mount_type,
                        "options": mount.options,
                    })
                }).collect::<Vec<_>>(),
                "network": run.network,
                "security": run.security,
                "shell": run.shell.as_ref().map(|shell| {
                    json!({
                        "text": shell.text,
                        "span": shell.span,
                    })
                }),
            })
        }),
        "copy": instruction.copy.as_ref().map(|copy| {
            json!({
                "kind": match copy.kind {
                    CopyKind::Copy => "copy",
                    CopyKind::Add => "add",
                },
                "flags": copy.flags,
                "from": copy.from,
                "chown": copy.chown,
                "chmod": copy.chmod,
                "sources": copy.sources,
                "destination": copy.destination,
            })
        }),
        "healthcheck": instruction.healthcheck.as_ref().map(|healthcheck| {
            json!({
                "flags": healthcheck.flags,
                "command": healthcheck.command.as_ref().map(|command| {
                    json!({
                        "text": command.text,
                        "span": command.span,
                    })
                }),
            })
        }),
        "arg": instruction.arg.as_ref().map(|arg| {
            json!({
                "name": arg.name,
                "default": arg.default,
            })
        }),
        "env": instruction.env.as_ref().map(|env| {
            json!({
                "form": match env.form {
                    EnvForm::KeyValue => "key_value",
                    EnvForm::LegacyPair => "legacy_pair",
                },
                "assignments": env.assignments.iter().map(|assignment| {
                    json!({
                        "name": assignment.name,
                        "value": assignment.value,
                    })
                }).collect::<Vec<_>>(),
            })
        }),
        "label": instruction.label.as_ref().map(|label| {
            json!({
                "pairs": label.pairs.iter().map(|pair| {
                    json!({
                        "key": pair.key,
                        "value": pair.value,
                    })
                }).collect::<Vec<_>>(),
            })
        }),
        "expose": instruction.expose.as_ref().map(|expose| {
            json!({
                "ports": expose.ports.iter().map(|port| {
                    json!({
                        "port": port.port,
                        "protocol": port.protocol,
                    })
                }).collect::<Vec<_>>(),
            })
        }),
        "recovery": instruction.recovery.as_ref().map(|recovery| {
            json!({
                "kind": match recovery.kind {
                    RecoveryKind::UnknownInstruction => "unknown_instruction",
                    RecoveryKind::MalformedInstruction => "malformed_instruction",
                },
                "message": recovery.message,
                "span": recovery.span,
            })
        }),
        "line": instruction.line,
        "raw_span": instruction.raw_span,
    })
}

fn instruction_form_json(form: &InstructionForm) -> Value {
    match form {
        InstructionForm::Empty => json!({ "kind": "empty" }),
        InstructionForm::Json(values) => json!({
            "kind": "json",
            "values": values,
        }),
        InstructionForm::InvalidJson { raw, error } => json!({
            "kind": "invalid_json",
            "raw": raw,
            "error": error,
        }),
        InstructionForm::Shell { text, span } => json!({
            "kind": "shell",
            "text": text,
            "span": span,
        }),
    }
}
