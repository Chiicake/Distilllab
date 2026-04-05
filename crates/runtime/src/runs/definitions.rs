#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunStepDefinition {
    pub step_key: &'static str,
    pub summary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunDefinition {
    pub run_type: &'static str,
    pub title: &'static str,
    pub primary_output_type: &'static str,
    pub steps: &'static [RunStepDefinition],
}

const IMPORT_AND_DISTILL_STEPS: [RunStepDefinition; 4] = [
    RunStepDefinition {
        step_key: "materialize_sources",
        summary: "Materialize the current work material into one or more sources.",
    },
    RunStepDefinition {
        step_key: "chunk_sources",
        summary: "Chunk the source material into retrieval and extraction units.",
    },
    RunStepDefinition {
        step_key: "extract_work_items",
        summary: "Extract structured work items from the chunked material.",
    },
    RunStepDefinition {
        step_key: "extract_assets",
        summary: "Distill the extracted material into final insight assets.",
    },
];

const IMPORT_AND_DISTILL: RunDefinition = RunDefinition {
    run_type: "import_and_distill",
    title: "Import and Distill",
    primary_output_type: "asset",
    steps: &IMPORT_AND_DISTILL_STEPS,
};

pub fn import_and_distill_definition() -> &'static RunDefinition {
    &IMPORT_AND_DISTILL
}

pub fn import_and_distill_step_definitions() -> &'static [RunStepDefinition] {
    import_and_distill_definition().steps
}
