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
