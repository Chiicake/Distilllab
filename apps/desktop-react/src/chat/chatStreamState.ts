import type { ChatMessage, LiveToolEvent } from './types.ts';

export function toolStatusFromText(statusText: string): 'success' | 'failed' {
  return /error|failed|failure/i.test(statusText) ? 'failed' : 'success';
}

export function liveToolStatusLabel(status: LiveToolEvent['status']): 'started' | 'success' | 'failed' {
  if (status === 'succeeded') {
    return 'success';
  }

  if (status === 'failed') {
    return 'failed';
  }

  return 'started';
}

export function deriveCompletedActiveRunLabel(
  messages: ChatMessage[],
  createdRunId: string | null | undefined,
  previousActiveRunLabel: string | null | undefined,
): string | null {
  const activeRunMessage = messages.find((message) => {
    const state = message.runMeta?.state;
    return Boolean(
      message.kind === 'run'
      && message.runMeta?.runId
      && (state === 'queued' || state === 'running' || state === 'pending'),
    );
  });

  return activeRunMessage?.runMeta?.runId ?? null;
}
