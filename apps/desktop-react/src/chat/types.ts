export type ChatMessageRole = 'user' | 'assistant' | 'system';

export type ChatMessage = {
  id: string;
  role: ChatMessageRole;
  content: string;
  pending?: boolean;
  expandable?: boolean;
  summary?: string;
  details?: string;
  kind?: 'tool' | 'status' | 'message';
};

export type ChatSessionSummary = {
  sessionId: string;
  title: string;
  statusLabel: string;
};

export type ChatStreamPhase =
  | 'started'
  | 'decision_ready'
  | 'tool_started'
  | 'tool_finished'
  | 'run_started'
  | 'run_finished'
  | 'assistant_started'
  | 'assistant_chunk'
  | 'completed'
  | 'error';

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
