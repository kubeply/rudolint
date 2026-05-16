use rudolint_dockerfile::{parse_dockerfile, stages};
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
