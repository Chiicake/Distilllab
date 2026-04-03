import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useDeferredValue,
  useRef,
  useState,
  type ReactNode,
} from 'react';

import { getTauriInvoke, getTauriListen, loadTauriEventApi } from './tauri';
import { parseTimelineText } from './timeline';
import type { ChatMessage, ChatSessionSummary, ChatState, ChatStreamEvent } from './types';

type SessionSelectorOption = {
  sessionId: string;
  title: string;
  status: string;
  label: string;
};

type ChatContextValue = {
  state: ChatState;
  refreshSessions: () => Promise<void>;
  openSession: (sessionId: string) => Promise<void>;
  sendFirstMessage: (message: string) => Promise<string | null>;
  sendFollowUpMessage: (message: string) => Promise<void>;
  resetDraft: () => void;
};

const ChatContext = createContext<ChatContextValue | null>(null);

const INITIAL_STATE: ChatState = {
  sessionId: null,
  sessionTitle: 'New Session',
  messages: [],
  sessions: [],
  isStreaming: false,
  errorText: null,
  activeRunLabel: null,
  streamStatuses: [],
  decisionSummary: null,
  lastToolStatus: null,
  lastRunStatus: null,
  liveAssistantText: '',
};

function summarizeToolStatus(statusText: string): string {
  const compact = statusText.replace(/\s+/g, ' ').trim();
  if (compact.length <= 120) {
    return compact;
  }

  return `${compact.slice(0, 117)}...`;
}

function createRequestId() {
  return `request-${crypto.randomUUID()}`;
}

function sessionSummaryFromOption(option: SessionSelectorOption): ChatSessionSummary {
  return {
    sessionId: option.sessionId,
    title: option.title,
    statusLabel: option.status,
  };
}

export default function ChatProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<ChatState>(INITIAL_STATE);
  const activeRequestRef = useRef<string | null>(null);
  const deferredState = useDeferredValue(state);

  const refreshSessions = useCallback(async () => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    const raw = await invoke<string>('list_session_selector_options');
    const options = JSON.parse(raw) as SessionSelectorOption[];

    setState((previous) => ({
      ...previous,
      sessions: options.map(sessionSummaryFromOption),
      sessionTitle:
        previous.sessionId != null
          ? options.find((option) => option.sessionId === previous.sessionId)?.title ?? previous.sessionTitle
          : previous.sessionTitle,
    }));
  }, []);

  const openSession = useCallback(async (sessionId: string) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    const timelineText = await invoke<string>('list_session_messages_command', { sessionId });

    setState((previous) => ({
      ...previous,
      sessionId,
      sessionTitle:
        previous.sessions.find((session) => session.sessionId === sessionId)?.title ?? previous.sessionTitle,
      messages: parseTimelineText(timelineText),
      errorText: null,
      activeRunLabel: null,
      streamStatuses: [],
      decisionSummary: null,
      lastToolStatus: null,
      lastRunStatus: null,
      liveAssistantText: '',
    }));
  }, []);

  const applyStreamEvent = useCallback((event: ChatStreamEvent) => {
    if (activeRequestRef.current !== event.requestId) {
      return;
    }

    setState((previous) => {
      switch (event.phase) {
        case 'started': {
          return {
            ...previous,
            sessionId: event.sessionId,
            errorText: null,
            isStreaming: true,
            activeRunLabel: null,
            streamStatuses: event.statusText ? [event.statusText] : previous.streamStatuses,
            decisionSummary: null,
            lastToolStatus: null,
            lastRunStatus: null,
            liveAssistantText: '',
          };
        }
        case 'decision_ready': {
          return {
            ...previous,
            activeRunLabel: event.createdRunId ?? (event.actionType === 'create_run' ? 'Active Run' : null),
            decisionSummary: event.statusText ?? previous.decisionSummary,
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : previous.streamStatuses,
          };
        }
        case 'assistant_started': {
          const pendingMessage: ChatMessage = {
            id: 'assistant-pending',
            role: 'assistant',
            content: '',
            pending: true,
          };

          return {
            ...previous,
            messages: [...previous.messages, pendingMessage],
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : previous.streamStatuses,
            liveAssistantText: '',
          };
        }
        case 'tool_started': {
          const rawStatus = event.statusText ?? 'Tool started.';
          const toolMessage: ChatMessage = {
            id: `system-tool-start-${event.requestId}`,
            role: 'system',
            kind: 'tool',
            expandable: true,
            summary: summarizeToolStatus(rawStatus),
            details: rawStatus,
            content: rawStatus,
          };

          return {
            ...previous,
            messages: [...previous.messages, toolMessage],
            lastToolStatus: event.statusText ?? previous.lastToolStatus,
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : previous.streamStatuses,
          };
        }
        case 'tool_finished': {
          const rawStatus = event.statusText ?? 'Tool finished.';
          const toolMessage: ChatMessage = {
            id: `system-tool-finish-${event.requestId}`,
            role: 'system',
            kind: 'tool',
            expandable: true,
            summary: summarizeToolStatus(rawStatus),
            details: rawStatus,
            content: rawStatus,
          };

          return {
            ...previous,
            messages: [...previous.messages, toolMessage],
            lastToolStatus: event.statusText ?? previous.lastToolStatus,
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : previous.streamStatuses,
          };
        }
        case 'run_started': {
          const runMessage: ChatMessage = {
            id: `system-run-start-${event.requestId}`,
            role: 'system',
            kind: 'status',
            content: event.statusText ?? 'Run started.',
            expandable: true,
            summary: event.statusText ?? 'Run started.',
            details: event.statusText ?? 'Run started.',
          };

          return {
            ...previous,
            messages: [...previous.messages, runMessage],
            lastRunStatus: event.statusText ?? previous.lastRunStatus,
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : previous.streamStatuses,
          };
        }
        case 'run_finished': {
          const runMessage: ChatMessage = {
            id: `system-run-finish-${event.requestId}`,
            role: 'system',
            kind: 'status',
            content: event.statusText ?? 'Run finished.',
            expandable: true,
            summary: event.statusText ?? 'Run finished.',
            details: event.statusText ?? 'Run finished.',
          };

          return {
            ...previous,
            messages: [...previous.messages, runMessage],
            lastRunStatus: event.statusText ?? previous.lastRunStatus,
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : previous.streamStatuses,
          };
        }
        case 'assistant_chunk': {
          return {
            ...previous,
            messages: previous.messages.map((message) =>
              message.id === 'assistant-pending'
                ? {
                    ...message,
                    content: `${message.content}${event.chunkText ?? ''}`,
                  }
                : message,
            ),
            liveAssistantText: `${previous.liveAssistantText}${event.chunkText ?? ''}`,
          };
        }
        case 'completed': {
          const parsedTimeline = event.timelineText ? parseTimelineText(event.timelineText) : previous.messages;

          return {
            ...previous,
            isStreaming: false,
            errorText: null,
            messages: parsedTimeline,
            activeRunLabel: event.createdRunId ?? previous.activeRunLabel,
            streamStatuses: [...previous.streamStatuses, 'timeline synchronized'],
            liveAssistantText: '',
          };
        }
        case 'error': {
          return {
            ...previous,
            isStreaming: false,
            errorText: event.errorText ?? 'Unknown error',
            messages: previous.messages.filter((message) => message.id !== 'assistant-pending'),
            streamStatuses: event.errorText
              ? [...previous.streamStatuses, `error: ${event.errorText}`]
              : previous.streamStatuses,
            liveAssistantText: '',
          };
        }
      }
    });
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    const tryBindListener = async () => {
      const builtinListen = getTauriListen();

      if (builtinListen) {
        unlisten = await builtinListen<ChatStreamEvent>('distilllab://chat-stream', ({ payload }) => {
          applyStreamEvent(payload);
        });
        return;
      }

      const eventApi = await loadTauriEventApi();
      if (cancelled || !eventApi?.listen) {
        return;
      }

      unlisten = await eventApi.listen<ChatStreamEvent>('distilllab://chat-stream', ({ payload }) => {
        applyStreamEvent(payload);
      });
    };

    void tryBindListener();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [applyStreamEvent]);

  useEffect(() => {
    void refreshSessions();
  }, [refreshSessions]);

  const sendFirstMessage = useCallback(async (message: string) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      setState((previous) => ({ ...previous, errorText: 'Tauri bridge unavailable.' }));
      return null;
    }

    const requestId = createRequestId();
    activeRequestRef.current = requestId;

    setState((previous) => ({
      ...previous,
      messages: [{ id: `user-${requestId}`, role: 'user', content: message }],
      isStreaming: true,
      errorText: null,
      streamStatuses: [],
      decisionSummary: null,
      lastToolStatus: null,
      lastRunStatus: null,
      liveAssistantText: '',
    }));

    let sessionId: string | null = null;

    try {
      sessionId = await invoke<string>('stream_first_session_message_command', {
        payload: {
          requestId,
          form: {
            sessionId: 'draft',
            userMessage: message,
            attachmentPaths: [],
          },
        },
      });
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error);
      setState((previous) => ({
        ...previous,
        isStreaming: false,
        errorText,
      }));
      return null;
    }

    await refreshSessions();
    return sessionId;
  }, [refreshSessions]);

  const sendFollowUpMessage = useCallback(async (message: string) => {
    const invoke = getTauriInvoke();
    if (!invoke || !state.sessionId) {
      setState((previous) => ({ ...previous, errorText: 'Active session required.' }));
      return;
    }

    const requestId = createRequestId();
    activeRequestRef.current = requestId;

    setState((previous) => ({
      ...previous,
      messages: [...previous.messages, { id: `user-${requestId}`, role: 'user', content: message }],
      isStreaming: true,
      errorText: null,
      streamStatuses: [],
      decisionSummary: null,
      lastToolStatus: null,
      lastRunStatus: null,
      liveAssistantText: '',
    }));

    try {
      await invoke('stream_session_message_command', {
        payload: {
          requestId,
          form: {
            sessionId: state.sessionId,
            userMessage: message,
            attachmentPaths: [],
          },
        },
      });
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error);
      setState((previous) => ({
        ...previous,
        isStreaming: false,
        errorText,
      }));
      return;
    }

    await refreshSessions();
  }, [refreshSessions, state.sessionId]);

  const resetDraft = useCallback(() => {
    activeRequestRef.current = null;
    setState((previous) => ({
      ...previous,
      sessionId: null,
      sessionTitle: 'New Session',
      messages: [],
      isStreaming: false,
      errorText: null,
      activeRunLabel: null,
      streamStatuses: [],
      decisionSummary: null,
      lastToolStatus: null,
      lastRunStatus: null,
      liveAssistantText: '',
    }));
  }, []);

  const value = useMemo<ChatContextValue>(
    () => ({
      state: deferredState,
      refreshSessions,
      openSession,
      sendFirstMessage,
      sendFollowUpMessage,
      resetDraft,
    }),
    [deferredState, openSession, refreshSessions, resetDraft, sendFirstMessage, sendFollowUpMessage],
  );

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
}

export function useChat() {
  const context = useContext(ChatContext);
  if (!context) {
    throw new Error('useChat must be used within ChatProvider');
  }

  return context;
}
