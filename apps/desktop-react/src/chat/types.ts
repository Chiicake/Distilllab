export type ChatMessageRole = 'user' | 'assistant' | 'system';

export type RunCardMeta = {
  runId: string;
  state: 'pending' | 'running' | 'completed' | 'failed';
  progressPercent: number;
  runType?: string | null;
  stepKey?: string | null;
  stepSummary?: string | null;
  stepStatus?: string | null;
  stepIndex?: number | null;
  stepsTotal?: number | null;
  detailText?: string | null;
  currentStepKey?: string | null;
  steps?: RunStepMeta[];
};

export type RunStepMeta = {
  key: string;
  summary: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | string;
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

export type RunProgressPhase = 'created' | 'state_changed' | 'step_started' | 'step_finished';

export type RunProgressUpdate = {
  phase: RunProgressPhase;
  runId: string;
  runType: string;
  runState: 'pending' | 'running' | 'completed' | 'failed' | string;
  progressPercent?: number | null;
  stepKey?: string | null;
  stepSummary?: string | null;
  stepStatus?: string | null;
  stepIndex?: number | null;
  stepsTotal?: number | null;
  detailText?: string | null;
};

export type ChatStreamEvent = {
  requestId: string;
  sessionId: string;
  phase: ChatStreamPhase;
  actionType?: string | null;
  intent?: string | null;
  chunkText?: string | null;
  statusText?: string | null;
  assistantText?: string | null;
  timelineText?: string | null;
  errorText?: string | null;
  createdRunId?: string | null;
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
