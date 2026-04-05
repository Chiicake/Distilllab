import type { ChatMessage, ChatMessageRole, RunCardMeta } from './types';

const HEADER_PATTERN = /^\[(User|Assistant|System|Tool|Run)\]/;

function compactJsonSnippet(text: string): string {
  return text.replace(/\s+/g, ' ').trim();
}

function summarizeToolHeader(header: string, body: string): string {
  const rawHeader = header.replace(/^\[Tool\]\s*/, '').trim();
  const success = !/error|failed|failure/i.test(body);
  const status = success ? 'success' : 'error';

  const match = rawHeader.match(/^([^\(]+)(\((.*)\))?$/);
  if (!match) {
    return `${rawHeader} · ${status}`;
  }

  const toolName = match[1]?.trim() ?? rawHeader;
  const rawArgs = match[3]?.trim();

  if (!rawArgs) {
    return `${toolName} · ${status}`;
  }

  let argsPreview = rawArgs;
  try {
    const parsed = JSON.parse(rawArgs) as Record<string, unknown>;
    const previewEntries = Object.entries(parsed)
      .slice(0, 2)
      .map(([key, value]) => `${key}=${compactJsonSnippet(JSON.stringify(value))}`);
    argsPreview = previewEntries.join(', ');
  } catch {
    argsPreview = compactJsonSnippet(rawArgs);
  }

  if (argsPreview.length > 72) {
    argsPreview = `${argsPreview.slice(0, 69)}...`;
  }

  return `${toolName}(${argsPreview}) · ${status}`;
}

function roleFromHeader(header: string): ChatMessageRole {
  if (header.includes('[User]')) {
    return 'user';
  }

  if (header.includes('[System]') || header.includes('[Tool]')) {
    return 'system';
  }

  return 'assistant';
}

function parseRunHeaderMeta(header: string): {
  runId: string;
  stepKey: string | null;
  phase: string;
} {
  const normalized = header.trim();
  const withStep = normalized.match(/^\[Run\]\s+([^\s\[]+)\s+\[([^\]]+)\]\s+\(([^\)]+)\)$/);
  if (withStep) {
    return {
      runId: withStep[1],
      stepKey: withStep[2],
      phase: withStep[3],
    };
  }

  const withoutStep = normalized.match(/^\[Run\]\s+([^\s\[]+)\s+\(([^\)]+)\)$/);
  if (withoutStep) {
    return {
      runId: withoutStep[1],
      stepKey: null,
      phase: withoutStep[2],
    };
  }

  return {
    runId: 'run-unknown',
    stepKey: null,
    phase: 'state_changed',
  };
}

function parseRunMetaFromDataJson(dataJson: string, fallbackHeader: string): RunCardMeta {
  const fallback = parseRunHeaderMeta(fallbackHeader);
  const fallbackState: RunCardMeta['state'] =
    fallback.phase.includes('finished') || fallback.phase.includes('completed')
      ? 'completed'
      : fallback.phase.includes('failed')
        ? 'failed'
        : fallback.phase.includes('started')
          ? 'running'
          : 'pending';

  const fallbackPercent =
    fallbackState === 'completed' || fallbackState === 'failed'
      ? 100
      : fallbackState === 'running'
        ? 55
        : 10;

  try {
    const parsed = JSON.parse(dataJson) as {
      runProgress?: {
        runId?: string;
        runState?: string;
        progressPercent?: number;
        runType?: string;
        stepKey?: string;
        stepSummary?: string;
        stepStatus?: string;
        stepIndex?: number;
        stepsTotal?: number;
        detailText?: string;
      };
    };
    const rp = parsed.runProgress;
    if (!rp) {
      return {
        runId: fallback.runId,
        state: fallbackState,
        progressPercent: fallbackPercent,
        stepKey: fallback.stepKey,
      };
    }

    const normalizedState = (rp.runState ?? '').toLowerCase();
    const state: RunCardMeta['state'] =
      normalizedState === 'completed'
        ? 'completed'
        : normalizedState === 'failed'
          ? 'failed'
          : normalizedState === 'running'
            ? 'running'
            : 'pending';

    return {
      runId: rp.runId ?? fallback.runId,
      state,
      progressPercent:
        typeof rp.progressPercent === 'number'
          ? Math.max(0, Math.min(100, rp.progressPercent))
          : fallbackPercent,
      runType: rp.runType ?? null,
      stepKey: rp.stepKey ?? fallback.stepKey,
      stepSummary: rp.stepSummary ?? null,
      stepStatus: rp.stepStatus ?? null,
      stepIndex: typeof rp.stepIndex === 'number' ? rp.stepIndex : null,
      stepsTotal: typeof rp.stepsTotal === 'number' ? rp.stepsTotal : null,
      detailText: rp.detailText ?? null,
    };
  } catch {
    return {
      runId: fallback.runId,
      state: fallbackState,
      progressPercent: fallbackPercent,
      stepKey: fallback.stepKey,
    };
  }
}

function parseStepHistoryFromDataJson(dataJson: string): RunCardMeta['steps'] {
  try {
    const parsed = JSON.parse(dataJson) as {
      runProgressHistory?: Array<{
        stepKey?: string;
        stepSummary?: string;
        stepStatus?: string;
        stepIndex?: number;
        stepsTotal?: number;
        detailText?: string;
      }>;
    };

    if (!Array.isArray(parsed.runProgressHistory)) {
      return [];
    }

    return parsed.runProgressHistory
      .filter((entry) => typeof entry.stepKey === 'string' && entry.stepKey.trim().length > 0)
      .map((entry) => {
        const normalized = (entry.stepStatus ?? '').toLowerCase();
        const status: 'pending' | 'running' | 'completed' | 'failed' =
          normalized === 'failed'
            ? 'failed'
            : normalized === 'completed' || normalized === 'finished'
              ? 'completed'
              : normalized === 'running' || normalized === 'started'
                ? 'running'
                : 'pending';

        return {
          key: entry.stepKey as string,
          summary: entry.stepSummary ?? (entry.stepKey as string),
          status,
          index: typeof entry.stepIndex === 'number' ? entry.stepIndex : null,
          total: typeof entry.stepsTotal === 'number' ? entry.stepsTotal : null,
          detailText: entry.detailText ?? null,
        };
      });
  } catch {
    return [];
  }
}

function parseToolMessage(header: string, body: string, index: number): ChatMessage {
  const summary = summarizeToolHeader(header, body);

  return {
    id: `timeline-${index}`,
    role: 'system',
    kind: 'tool',
    expandable: true,
    summary,
    details: body,
    content: [summary, body].filter(Boolean).join('\n'),
  };
}

function parseRegularMessage(header: string, body: string, index: number): ChatMessage {
  const bodyLines = body.split('\n');
  const lastLine = bodyLines[bodyLines.length - 1]?.trim() ?? '';
  const maybeJson = lastLine.startsWith('{') && lastLine.endsWith('}') ? lastLine : null;

  let visibleBody = body;
  let attachments: ChatMessage['attachments'] | undefined;

  if (maybeJson) {
    try {
      const parsed = JSON.parse(maybeJson) as {
        attachments?: Array<{
          name?: string;
          size?: number;
        }>;
      };

      if (Array.isArray(parsed.attachments) && parsed.attachments.length > 0) {
        attachments = parsed.attachments
          .filter((attachment) => typeof attachment.name === 'string' && attachment.name.trim().length > 0)
          .map((attachment) => ({
            name: attachment.name as string,
            size: typeof attachment.size === 'number' ? attachment.size : null,
          }));
        visibleBody = bodyLines.slice(0, -1).join('\n').trim();
      }
    } catch {
      // ignore parse failure and keep body as-is
    }
  }

  return {
    id: `timeline-${index}`,
    role: roleFromHeader(header),
    kind: 'message',
    content: visibleBody,
    attachments,
  };
}

function parseRunMessage(header: string, body: string, dataJson: string): ChatMessage {
  const runMeta = parseRunMetaFromDataJson(dataJson, header);
  const stepHistory = parseStepHistoryFromDataJson(dataJson);
  const summary = body.split('\n').find((line) => line.trim().length > 0)?.trim() ?? body;

  return {
    id: `run-card-${runMeta.runId}`,
    role: 'system',
    kind: 'run',
    expandable: true,
    summary,
    details: body,
    content: body,
    runMeta: {
      ...runMeta,
      currentStepKey: runMeta.stepKey,
      steps: stepHistory,
    },
  };
}

export function parseTimelineText(timelineText: string): ChatMessage[] {
  const lines = timelineText.split('\n');
  const messages: ChatMessage[] = [];

  let activeHeader: string | null = null;
  let activeContent: string[] = [];

  const flush = () => {
    if (!activeHeader) {
      return;
    }

    const normalizedBody = activeContent.map((line) => line.replace(/^\s{2}/, '')).join('\n').trim();
    if (activeHeader.startsWith('[Tool]')) {
      messages.push(parseToolMessage(activeHeader, normalizedBody, messages.length));
    } else if (activeHeader.startsWith('[Run]')) {
      const bodyLines = normalizedBody.split('\n');
      const lastLine = bodyLines[bodyLines.length - 1]?.trim() ?? '';
      const dataJson = lastLine.startsWith('{') && lastLine.endsWith('}') ? lastLine : '{}';
      const visibleBody = dataJson === '{}'
        ? normalizedBody
        : bodyLines.slice(0, -1).join('\n').trim();
      const runMessage = parseRunMessage(activeHeader, visibleBody, dataJson);
      const existingIndex = messages.findIndex((message) => message.id === runMessage.id);
      if (existingIndex >= 0) {
        messages[existingIndex] = runMessage;
      } else {
        messages.push(runMessage);
      }
    } else if (normalizedBody.length > 0) {
      messages.push(parseRegularMessage(activeHeader, normalizedBody, messages.length));
    }

    activeHeader = null;
    activeContent = [];
  };

  for (const line of lines) {
    if (HEADER_PATTERN.test(line)) {
      flush();
      activeHeader = line.trim();
      continue;
    }

    if (!activeHeader) {
      continue;
    }

    activeContent.push(line);
  }

  flush();

  return messages;
}
