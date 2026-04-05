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
import { desktopTimelineToChatMessages } from './timeline';
import { parseTimelineText } from './timelineTextLegacy';
import type {
  ChatMessage,
  ChatSessionSummary,
  ChatState,
  ChatStreamEvent,
  DesktopTimelineMessage,
  RunState,
  RunStepStatus,
} from './types';

function attachmentSummariesFromPaths(paths: string[]) {
  return paths.map((path) => ({
    name: path.split(/[\\/]/).pop() ?? path,
    size: null,
  }));
}

type SessionSelectorOption = {
  sessionId: string;
  title: string;
  manualTitle?: string | null;
  pinned?: boolean;
  updatedAt: string;
  status: string;
  label: string;
};

type ChatContextValue = {
  state: ChatState;
  refreshSessions: () => Promise<void>;
  openSession: (sessionId: string) => Promise<void>;
  renameSession: (sessionId: string, manualTitle: string | null) => Promise<void>;
  pinSession: (sessionId: string, pinned: boolean) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  cancelActiveRequest: () => Promise<void>;
  sendFirstMessage: (message: string, attachmentPaths?: string[]) => Promise<string | null>;
  sendFollowUpMessage: (message: string, attachmentPaths?: string[]) => Promise<void>;
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

function toolStatusFromText(statusText: string): 'success' | 'error' {
  return /error|failed|failure/i.test(statusText) ? 'error' : 'success';
}

function toolNameFromStatusText(statusText: string, fallback: string): string {
  const match = statusText.match(/^tool\s+(?:started|finished):\s+(.+)$/i);
  return match?.[1]?.trim() || fallback;
}

function buildLiveToolMessage(messageId: string, statusText: string, fallbackToolName: string): ChatMessage {
  const toolName = toolNameFromStatusText(statusText, fallbackToolName);
  const status = toolStatusFromText(statusText);

  return {
    id: messageId,
    role: 'system',
    kind: 'tool',
    expandable: true,
    content: statusText,
    summary: `${toolName} · ${status}`,
    details: `tool: ${toolName}\nstatus: ${status}\n\nresult:\n${statusText}`,
  };
}

function createRequestId() {
  return `request-${crypto.randomUUID()}`;
}

function runStateFromStatus(statusText: string): RunState {
  const normalized = statusText.toLowerCase();
  if (normalized.includes('queued') || normalized.includes('created')) {
    return 'queued';
  }
  if (normalized.includes('failed')) {
    return 'failed';
  }
  if (normalized.includes('completed') || normalized.includes('finished')) {
    return 'completed';
  }
  if (normalized.includes('running') || normalized.includes('started')) {
    return 'running';
  }
  return 'pending';
}

function runStateFromStatusOrMeta(statusText: string, previousState: RunState | null | undefined, fallback: RunState): RunState {
  if (!statusText.trim()) {
    return previousState ?? fallback;
  }

  const derived = runStateFromStatus(statusText);
  return derived === 'pending' ? previousState ?? fallback : derived;
}

function runFinishedStateFromStatusOrMeta(statusText: string, previousState: RunState | null | undefined): RunState {
  if (previousState === 'failed') {
    return 'failed';
  }

  return runStateFromStatusOrMeta(statusText, previousState, 'completed');
}

function runProgressFromState(state: RunState): number {
  if (state === 'completed') {
    return 100;
  }
  if (state === 'queued') {
    return 10;
  }
  if (state === 'running') {
    return 55;
  }
  if (state === 'failed') {
    return 100;
  }
  return 10;
}

function asRunState(value: string | null | undefined): RunState {
  const normalized = (value ?? '').toLowerCase();
  if (normalized === 'queued' || normalized === 'created') {
    return 'queued';
  }
  if (normalized === 'failed') {
    return 'failed';
  }
  if (normalized === 'completed') {
    return 'completed';
  }
  if (normalized === 'running') {
    return 'running';
  }
  return 'pending';
}

function normalizeStepStatus(value: string | null | undefined): RunStepStatus {
  const normalized = (value ?? '').toLowerCase();
  if (normalized === 'started') {
    return 'started';
  }
  if (normalized === 'failed') {
    return 'failed';
  }
  if (normalized === 'completed' || normalized === 'finished') {
    return 'completed';
  }
  if (normalized === 'running' || normalized === 'started') {
    return 'running';
  }
  return 'pending';
}

function mergeRunSteps(
  previousSteps: NonNullable<ChatMessage['runMeta']>['steps'] | undefined,
  nextStep: {
    key: string;
    summary: string;
    status: RunStepStatus;
    index?: number | null;
    total?: number | null;
    detailText?: string | null;
  } | null,
) {
  if (!nextStep) {
    return previousSteps ?? [];
  }

  const base = [...(previousSteps ?? [])];
  const existingIndex = base.findIndex((step) => step.key === nextStep.key);
  if (existingIndex >= 0) {
    base[existingIndex] = {
      ...base[existingIndex],
      ...nextStep,
    };
  } else {
    base.push(nextStep);
  }

  base.sort((a, b) => {
    const aIndex = a.index ?? Number.MAX_SAFE_INTEGER;
    const bIndex = b.index ?? Number.MAX_SAFE_INTEGER;
    if (aIndex !== bIndex) {
      return aIndex - bIndex;
    }
    return a.key.localeCompare(b.key);
  });

  return base;
}

function sessionSummaryFromOption(option: SessionSelectorOption): ChatSessionSummary {
  return {
    sessionId: option.sessionId,
    title: option.title,
    statusLabel: option.status,
    manualTitle: option.manualTitle ?? null,
    pinned: option.pinned ?? false,
    updatedAt: option.updatedAt,
  };
}

function sortSessions(sessions: ChatSessionSummary[]) {
  return [...sessions].sort((left, right) => {
    if (Boolean(left.pinned) !== Boolean(right.pinned)) {
      return left.pinned ? -1 : 1;
    }

    return (right.updatedAt ?? '').localeCompare(left.updatedAt ?? '');
  });
}

export default function ChatProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<ChatState>(INITIAL_STATE);
  const activeRequestRef = useRef<string | null>(null);
  const completedSyncRef = useRef<{ requestId: string; sessionId: string } | null>(null);
  const deferredState = useDeferredValue(state);

  const loadStructuredTimelineMessages = useCallback(async (sessionId: string) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      throw new Error('Tauri bridge unavailable.');
    }

    const timelineMessages = await invoke<DesktopTimelineMessage[]>('list_session_messages_structured_command', {
      sessionId,
    });

    return desktopTimelineToChatMessages(timelineMessages);
  }, []);

  const loadCompletedSyncTimelineMessages = useCallback(async (sessionId: string) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      throw new Error('Tauri bridge unavailable.');
    }

    try {
      return await loadStructuredTimelineMessages(sessionId);
    } catch {
      // Keep any legacy fallback explicit and contained to post-stream resync.
      const timelineText = await invoke<string>('list_session_messages_command', { sessionId });
      return parseTimelineText(timelineText);
    }
  }, [loadStructuredTimelineMessages]);

  const refreshSessions = useCallback(async () => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    const raw = await invoke<string>('list_session_selector_options');
    const options = JSON.parse(raw) as SessionSelectorOption[];

    setState((previous) => ({
      ...previous,
      sessions: sortSessions(options.map(sessionSummaryFromOption)),
      sessionTitle:
        previous.sessionId != null
          ? options.find((option) => option.sessionId === previous.sessionId)?.title ?? previous.sessionTitle
          : previous.sessionTitle,
    }));
  }, []);

  const openSession = useCallback(async (sessionId: string) => {
    completedSyncRef.current = null;
    const messages = await loadStructuredTimelineMessages(sessionId);

    setState((previous) => ({
      ...previous,
      sessionId,
      sessionTitle:
        previous.sessions.find((session) => session.sessionId === sessionId)?.title ?? previous.sessionTitle,
      messages,
      errorText: null,
      activeRunLabel: null,
      streamStatuses: [],
      decisionSummary: null,
      lastToolStatus: null,
      lastRunStatus: null,
      liveAssistantText: '',
    }));
  }, [loadStructuredTimelineMessages]);

  const renameSession = useCallback(async (sessionId: string, manualTitle: string | null) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    const previousState = state;
    setState((previous) => {
      const nextSessions = previous.sessions.map((session) =>
        session.sessionId === sessionId
          ? {
              ...session,
              manualTitle,
              title: manualTitle && manualTitle.trim().length > 0 ? manualTitle.trim() : session.title,
            }
          : session,
      );

      const nextTitle = nextSessions.find((session) => session.sessionId === sessionId)?.title ?? previous.sessionTitle;

      return {
        ...previous,
        sessions: sortSessions(nextSessions),
        sessionTitle: previous.sessionId === sessionId ? nextTitle : previous.sessionTitle,
      };
    });

    try {
      await invoke<string>('rename_session_command', {
        payload: {
          sessionId,
          manualTitle,
        },
      });

      await refreshSessions();
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error);
      setState({
        ...previousState,
        errorText,
      });
    }
  }, [refreshSessions, state.sessionId]);

  const pinSession = useCallback(async (sessionId: string, pinned: boolean) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    const previousState = state;
    setState((previous) => ({
      ...previous,
      sessions: sortSessions(
        previous.sessions.map((session) =>
          session.sessionId === sessionId
            ? {
                ...session,
                pinned,
              }
            : session,
        ),
      ),
    }));

    try {
      await invoke<string>('pin_session_command', {
        payload: {
          sessionId,
          pinned,
        },
      });

      await refreshSessions();
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error);
      setState({
        ...previousState,
        errorText,
      });
    }
  }, [refreshSessions]);

  const deleteSession = useCallback(async (sessionId: string) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    const previousState = state;
    setState((previous) => {
      const deletingActive = previous.sessionId === sessionId;
      return {
        ...previous,
        sessions: previous.sessions.filter((session) => session.sessionId !== sessionId),
        sessionId: deletingActive ? null : previous.sessionId,
        sessionTitle: deletingActive ? 'New Session' : previous.sessionTitle,
        messages: deletingActive ? [] : previous.messages,
        errorText: deletingActive ? null : previous.errorText,
        activeRunLabel: deletingActive ? null : previous.activeRunLabel,
        streamStatuses: deletingActive ? [] : previous.streamStatuses,
        decisionSummary: deletingActive ? null : previous.decisionSummary,
        lastToolStatus: deletingActive ? null : previous.lastToolStatus,
        lastRunStatus: deletingActive ? null : previous.lastRunStatus,
        liveAssistantText: deletingActive ? '' : previous.liveAssistantText,
      };
    });

    try {
      await invoke('delete_session_command', { sessionId });
      await refreshSessions();
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error);
      setState({
        ...previousState,
        errorText,
      });
    }
  }, [refreshSessions, state]);

  const cancelActiveRequest = useCallback(async () => {
    const invoke = getTauriInvoke();
    const requestId = activeRequestRef.current;
    const sessionId = state.sessionId;
    if (!invoke || !requestId || !sessionId) {
      return;
    }

    try {
      await invoke('cancel_stream_request_command', {
        payload: {
          sessionId,
          requestId,
        },
      });
    } catch (error) {
      const errorText = error instanceof Error ? error.message : String(error);
      setState((previous) => ({
        ...previous,
        errorText,
      }));
    }
  }, [state.sessionId]);

  const applyStreamEvent = useCallback((event: ChatStreamEvent) => {
    if (activeRequestRef.current !== event.requestId) {
      return;
    }

    if (event.phase === 'completed') {
      const syncContext = {
        requestId: event.requestId,
        sessionId: event.sessionId,
      };
      completedSyncRef.current = syncContext;
      activeRequestRef.current = null;

      setState((previous) => ({
        ...previous,
        isStreaming: false,
        errorText: null,
        streamStatuses: [...previous.streamStatuses, 'timeline synchronizing'],
        liveAssistantText: '',
      }));

      void loadCompletedSyncTimelineMessages(event.sessionId)
        .then((messages) => {
          if (
            completedSyncRef.current?.requestId !== syncContext.requestId
            || completedSyncRef.current?.sessionId !== syncContext.sessionId
          ) {
            return;
          }

          setState((previous) => {
            if (previous.sessionId !== syncContext.sessionId) {
              return previous;
            }

            return {
              ...previous,
              messages,
              activeRunLabel: event.createdRunId ?? previous.activeRunLabel,
              streamStatuses: [...previous.streamStatuses, 'timeline synchronized'],
            };
          });
        })
        .catch((error) => {
          if (
            completedSyncRef.current?.requestId !== syncContext.requestId
            || completedSyncRef.current?.sessionId !== syncContext.sessionId
          ) {
            return;
          }

          const errorText = error instanceof Error ? error.message : String(error);
          setState((previous) => {
            if (previous.sessionId !== syncContext.sessionId) {
              return previous;
            }

            return {
              ...previous,
              errorText: errorText || previous.errorText,
              streamStatuses: [...previous.streamStatuses, `timeline sync failed: ${errorText}`],
            };
          });
        });

      return;
    }

    completedSyncRef.current = null;

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
          const handoffMessage =
            event.actionType === 'create_run' && event.assistantText
              ? {
                  id: `assistant-handoff-${event.requestId}`,
                  role: 'assistant' as const,
                  content: event.assistantText,
                }
              : null;

          const nextMessages = handoffMessage
            && !previous.messages.some((message) => message.id === handoffMessage.id)
              ? [...previous.messages, handoffMessage]
              : previous.messages;

          return {
            ...previous,
            messages: nextMessages,
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
          const toolMessage = buildLiveToolMessage(`system-tool-start-${event.requestId}`, rawStatus, 'tool');

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
          const toolMessage = buildLiveToolMessage(`system-tool-finish-${event.requestId}`, rawStatus, 'tool');

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
          const runId = event.createdRunId ?? 'run-active';
          const statusText = event.statusText ?? 'Run started.';
          const previousRunMeta = previous.messages.find((message) => message.id === `run-card-${runId}`)?.runMeta;
          const state = runStateFromStatusOrMeta(statusText, previousRunMeta?.state, 'queued');
          const runMessage: ChatMessage = {
            id: `run-card-${runId}`,
            role: 'system',
            kind: 'run',
            content: statusText,
            expandable: true,
            summary: statusText,
            details: statusText,
            runMeta: {
              runId,
              state,
              progressPercent: runProgressFromState(state),
              runType: previousRunMeta?.runType ?? null,
              stepKey: previousRunMeta?.stepKey ?? null,
              stepSummary: previousRunMeta?.stepSummary ?? null,
              stepStatus: previousRunMeta?.stepStatus ?? null,
              stepIndex: previousRunMeta?.stepIndex ?? null,
              stepsTotal: previousRunMeta?.stepsTotal ?? null,
              detailText: previousRunMeta?.detailText ?? null,
              currentStepKey: previousRunMeta?.currentStepKey ?? null,
              steps: previousRunMeta?.steps ?? [],
            },
          };

          const filteredMessages = previous.messages.filter((message) => message.id !== runMessage.id);

          return {
            ...previous,
            messages: [...filteredMessages, runMessage],
            lastRunStatus: statusText,
            streamStatuses: statusText
              ? [...previous.streamStatuses, statusText]
              : previous.streamStatuses,
          };
        }
        case 'run_created':
        case 'run_step_started':
        case 'run_step_finished':
        case 'run_progress': {
          const update = event.runProgress;
          if (!update) {
            return previous;
          }

          const runId = update.runId;
          const runState = asRunState(update.runState);
          const progressPercent =
            typeof update.progressPercent === 'number'
              ? Math.max(0, Math.min(100, update.progressPercent))
              : runProgressFromState(runState);

          const fallbackStatus = [
            update.stepSummary ?? update.stepKey ?? `Run ${runId}`,
            update.detailText,
          ]
            .filter(Boolean)
            .join(' - ');
          const statusText = event.statusText ?? (fallbackStatus || `run ${runId} progress`);

          const previousRunMessage = previous.messages.find((message) => message.id === `run-card-${runId}`);
          const previousRunMeta = previousRunMessage?.runMeta;
          const normalizedStepStatus = normalizeStepStatus(update.stepStatus ?? update.phase);
          const nextStep = update.stepKey
            ? {
                key: update.stepKey,
                summary: update.stepSummary ?? update.stepKey,
                status: normalizedStepStatus,
                index: update.stepIndex,
                total: update.stepsTotal,
                detailText: update.detailText,
              }
            : null;
          const mergedSteps = mergeRunSteps(previousRunMeta?.steps, nextStep);

          const runMessage: ChatMessage = {
            id: `run-card-${runId}`,
            role: 'system',
            kind: 'run',
            content: statusText,
            expandable: true,
            summary: statusText,
            details: statusText,
            runMeta: {
              runId,
              state: runState,
              progressPercent,
              runType: update.runType,
              stepKey: update.stepKey,
              stepSummary: update.stepSummary,
              stepStatus: update.stepKey ? normalizedStepStatus : previousRunMeta?.stepStatus ?? null,
              stepIndex: update.stepIndex,
              stepsTotal: update.stepsTotal,
              detailText: update.detailText,
              currentStepKey: update.stepKey ?? previousRunMeta?.currentStepKey,
              steps: mergedSteps,
            },
          };

          const filteredMessages = previous.messages.filter((message) => message.id !== runMessage.id);
          return {
            ...previous,
            messages: [...filteredMessages, runMessage],
            activeRunLabel: runId,
            lastRunStatus: statusText,
            streamStatuses: [...previous.streamStatuses, statusText],
          };
        }
        case 'run_finished': {
          const runId = event.createdRunId ?? 'run-active';
          const statusText = event.statusText ?? 'Run finished.';
          const previousRunMeta = previous.messages.find((message) => message.id === `run-card-${runId}`)?.runMeta;
          const state = runFinishedStateFromStatusOrMeta(statusText, previousRunMeta?.state);
          const runMessage: ChatMessage = {
            id: `run-card-${runId}`,
            role: 'system',
            kind: 'run',
            content: statusText,
            expandable: true,
            summary: statusText,
            details: statusText,
            runMeta: {
              runId,
              state,
              progressPercent: runProgressFromState(state),
              runType: previousRunMeta?.runType ?? null,
              stepKey: previousRunMeta?.stepKey ?? null,
              stepSummary: previousRunMeta?.stepSummary ?? null,
              stepStatus: previousRunMeta?.stepStatus ?? null,
              stepIndex: previousRunMeta?.stepIndex ?? null,
              stepsTotal: previousRunMeta?.stepsTotal ?? null,
              detailText: previousRunMeta?.detailText ?? null,
              currentStepKey: previousRunMeta?.currentStepKey ?? null,
              steps: previousRunMeta?.steps ?? [],
            },
          };

          const filteredMessages = previous.messages.filter((message) => message.id !== runMessage.id);

          return {
            ...previous,
            messages: [...filteredMessages, runMessage],
            activeRunLabel: state === 'queued' || state === 'running' || state === 'pending' ? runId : null,
            lastRunStatus: statusText,
            streamStatuses: statusText
              ? [...previous.streamStatuses, statusText]
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
        case 'stopped': {
          activeRequestRef.current = null;
          return {
            ...previous,
            isStreaming: false,
            messages: previous.messages.map((message) =>
              message.id === 'assistant-pending'
                ? {
                    ...message,
                    pending: false,
                    content: message.content
                      ? `${message.content}

[generation stopped]`
                      : '[generation stopped]',
                  }
                : message,
            ),
            streamStatuses: event.statusText
              ? [...previous.streamStatuses, event.statusText]
              : [...previous.streamStatuses, 'request stopped'],
            liveAssistantText: '',
          };
        }
        case 'error': {
          activeRequestRef.current = null;
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
        default:
          return previous;
      }
    });
  }, [loadCompletedSyncTimelineMessages]);

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

  const sendFirstMessage = useCallback(async (message: string, attachmentPaths: string[] = []) => {
    const invoke = getTauriInvoke();
    if (!invoke) {
      setState((previous) => ({ ...previous, errorText: 'Tauri bridge unavailable.' }));
      return null;
    }

    const requestId = createRequestId();
    activeRequestRef.current = requestId;

    setState((previous) => ({
      ...previous,
      messages: [{
        id: `user-${requestId}`,
        role: 'user',
        content: message,
        attachments: attachmentSummariesFromPaths(attachmentPaths),
      }],
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
            attachmentPaths,
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

  const sendFollowUpMessage = useCallback(async (message: string, attachmentPaths: string[] = []) => {
    const invoke = getTauriInvoke();
    if (!invoke || !state.sessionId) {
      setState((previous) => ({ ...previous, errorText: 'Active session required.' }));
      return;
    }

    const requestId = createRequestId();
    activeRequestRef.current = requestId;

    setState((previous) => ({
      ...previous,
      messages: [...previous.messages, {
        id: `user-${requestId}`,
        role: 'user',
        content: message,
        attachments: attachmentSummariesFromPaths(attachmentPaths),
      }],
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
            attachmentPaths,
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
    completedSyncRef.current = null;
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
      renameSession,
      pinSession,
      deleteSession,
      cancelActiveRequest,
      sendFirstMessage,
      sendFollowUpMessage,
      resetDraft,
    }),
    [cancelActiveRequest, deferredState, deleteSession, openSession, pinSession, refreshSessions, renameSession, resetDraft, sendFirstMessage, sendFollowUpMessage],
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
