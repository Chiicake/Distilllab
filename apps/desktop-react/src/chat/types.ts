export type ChatMessageRole = 'user' | 'assistant' | 'system';

export type RunState = 'queued' | 'pending' | 'running' | 'completed' | 'failed';

export type RunStepStatus = 'started' | 'pending' | 'running' | 'completed' | 'failed';

export type DesktopTimelineAttachment = {
  name: string;
  size?: number | null;
};

export type DesktopRunStepMeta = {
  key: string;
  summary: string;
  status: RunStepStatus;
  index?: number | null;
  total?: number | null;
  detailText?: string | null;
};

export type DesktopRunCardMeta = {
  runId: string;
  state: RunState;
  progressPercent: number;
  runType?: string | null;
  stepKey?: string | null;
  stepSummary?: string | null;
  stepStatus?: RunStepStatus | null;
  stepIndex?: number | null;
  stepsTotal?: number | null;
  detailText?: string | null;
  currentStepKey?: string | null;
  steps?: DesktopRunStepMeta[];
};

export type RunCardMeta = {
  runId: string;
  state: RunState;
  progressPercent: number;
  runType?: string | null;
  stepKey?: string | null;
  stepSummary?: string | null;
  stepStatus?: RunStepStatus | null;
  stepIndex?: number | null;
  stepsTotal?: number | null;
  detailText?: string | null;
  currentStepKey?: string | null;
  steps?: RunStepMeta[];
};

export type RunStepMeta = {
  key: string;
  summary: string;
  status: RunStepStatus;
  index?: number | null;
  total?: number | null;
  detailText?: string | null;
};

export type ChatMessage = {
  id: string;
  role: ChatMessageRole;
  content: string;
  pending?: boolean;
  expandable?: boolean;
  summary?: string;
  details?: string;
  kind?: 'tool' | 'status' | 'run' | 'message';
  runMeta?: RunCardMeta;
  attachments?: Array<{
    name: string;
    size?: number | null;
  }>;
};

export type DesktopTimelineMessage = {
  id: string;
  role: ChatMessageRole;
  kind: 'message' | 'tool' | 'run';
  sourceMessageType?: string | null;
  content: string;
  summary?: string | null;
  details?: string | null;
  attachments?: DesktopTimelineAttachment[];
  runMeta?: DesktopRunCardMeta | null;
  createdAt: string;
};

export type ChatSessionSummary = {
  sessionId: string;
  title: string;
  statusLabel: string;
  manualTitle?: string | null;
  pinned?: boolean;
  updatedAt?: string;
};

export type ChatStreamPhase =
  | 'started'
  | 'decision_ready'
  | 'tool_started'
  | 'tool_finished'
  | 'run_created'
  | 'run_started'
  | 'run_step_started'
  | 'run_step_finished'
  | 'run_progress'
  | 'run_finished'
  | 'assistant_started'
  | 'assistant_chunk'
  | 'stopped'
  | 'completed'
  | 'error';

export type LiveToolStatus = 'started' | 'succeeded' | 'failed';

export type LiveToolEvent = {
  toolCallId: string;
  toolName: string;
  status: LiveToolStatus;
  content: string;
  argumentsText?: string | null;
  resultText?: string | null;
  summary: string;
  details: string;
};

export type LiveRunEvent = {
  runId: string;
  runType?: string | null;
  state: RunState;
  progressPercent?: number | null;
  detailText?: string | null;
  content?: string;
  summary?: string;
  details?: string;
};

export type RunProgressPhase = 'created' | 'state_changed' | 'step_started' | 'step_finished';

export type RunProgressUpdate = {
  phase: RunProgressPhase;
  runId: string;
  runType: string;
  runState: RunState;
  progressPercent?: number | null;
  stepKey?: string | null;
  stepSummary?: string | null;
  stepStatus?: RunStepStatus | null;
  stepIndex?: number | null;
  stepsTotal?: number | null;
  detailText?: string | null;
};

export type ChatStreamEvent = {
  requestId: string;
  sessionId: string;
  phase: ChatStreamPhase;
  // Compatibility rules for the live stream contract migration:
  // - toolEvent/runEvent are the intended primary semantics fields for Task 3.
  // - statusText/timelineText remain in active use until the reducer migration lands.
  // - reducer callers must still handle missing structured payloads safely during migration.
  actionType?: string | null;
  intent?: string | null;
  chunkText?: string | null;
  statusText?: string | null;
  assistantText?: string | null;
  timelineText?: string | null;
  errorText?: string | null;
  createdRunId?: string | null;
  toolEvent?: LiveToolEvent | null;
  runEvent?: LiveRunEvent | null;
  runProgress?: RunProgressUpdate | null;
};

export type ChatState = {
  sessionId: string | null;
  sessionTitle: string;
  messages: ChatMessage[];
  sessions: ChatSessionSummary[];
  isStreaming: boolean;
  errorText: string | null;
  activeRunLabel: string | null;
  streamStatuses: string[];
  decisionSummary: string | null;
  lastToolStatus: string | null;
  lastRunStatus: string | null;
  liveAssistantText: string;
};
