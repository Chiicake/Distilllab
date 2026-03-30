use crate::contracts::{DistillRunStepPreview, RunHandoffPreview};

pub fn build_import_and_distill_handoff_preview(
    primary_object_type: Option<String>,
    primary_object_id: Option<String>,
) -> RunHandoffPreview {
    RunHandoffPreview {
        run_type: "import_and_distill".to_string(),
        primary_object_type,
        primary_object_id,
        summary: "Previewing the import-and-distill workflow for this work material.".to_string(),
        planned_steps: vec![
            DistillRunStepPreview {
                step_key: "materialize_sources".to_string(),
                summary: "Materialize the current work material into one or more sources."
                    .to_string(),
            },
            DistillRunStepPreview {
                step_key: "chunk_sources".to_string(),
                summary: "Chunk the source material into retrieval and extraction units."
                    .to_string(),
            },
            DistillRunStepPreview {
                step_key: "extract_work_items".to_string(),
                summary: "Extract structured work items from the chunked material.".to_string(),
            },
        ],
    }
}
