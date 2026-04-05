import type { ChatMessage, DesktopTimelineMessage, RunCardMeta } from './types';

function desktopRunMetaToRunCardMeta(runMeta: DesktopTimelineMessage['runMeta']): RunCardMeta | undefined {
  if (!runMeta) {
    return undefined;
  }

  return {
    runId: runMeta.runId,
    state: runMeta.state,
    progressPercent: runMeta.progressPercent,
    runType: runMeta.runType ?? null,
    stepKey: runMeta.stepKey ?? null,
    stepSummary: runMeta.stepSummary ?? null,
    stepStatus: runMeta.stepStatus ?? null,
    stepIndex: runMeta.stepIndex ?? null,
    stepsTotal: runMeta.stepsTotal ?? null,
    detailText: runMeta.detailText ?? null,
    currentStepKey: runMeta.currentStepKey ?? null,
    steps: runMeta.steps?.map((step) => ({
      key: step.key,
      summary: step.summary,
      status: step.status,
      index: step.index ?? null,
      total: step.total ?? null,
      detailText: step.detailText ?? null,
    })),
  };
}

// Structured timeline mappers for the Tauri desktop DTO path.
// These helpers are shallow, field-to-field mappings and preserve backend order.
export function desktopTimelineMessageToChatMessage(message: DesktopTimelineMessage): ChatMessage {
  return {
    id: message.id,
    role: message.role,
    kind: message.kind,
    content: message.content,
    summary: message.summary ?? undefined,
    details: message.details ?? undefined,
    expandable: message.kind === 'tool' || message.kind === 'run',
    attachments: message.attachments?.map((attachment) => ({
      name: attachment.name,
      size: attachment.size ?? null,
    })),
    runMeta: desktopRunMetaToRunCardMeta(message.runMeta),
  };
}

export function desktopTimelineToChatMessages(messages: DesktopTimelineMessage[]): ChatMessage[] {
  return messages.map(desktopTimelineMessageToChatMessage);
}
