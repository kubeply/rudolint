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
