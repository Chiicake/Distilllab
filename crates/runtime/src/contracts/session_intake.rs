#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillRunStepPreview {
    pub step_key: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHandoffPreview {
    pub run_type: String,
    pub primary_object_type: Option<String>,
    pub primary_object_id: Option<String>,
    pub summary: String,
    pub planned_steps: Vec<DistillRunStepPreview>,
}

#[derive(Debug, Clone)]
pub struct SessionIntakePreview {
    pub decision: agent::SessionAgentDecision,
    pub run_handoff_preview: Option<RunHandoffPreview>,
}
