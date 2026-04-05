import { useEffect, useRef, useState } from 'react';

import { useChatAppearance } from '../../chat/ChatAppearanceProvider';
import { chatBodyTextClass, chatSecondaryTextClass } from '../../chat/font-size';
import type { ChatMessage } from '../../chat/types';

function formatRunTypeLabel(runType: string | null | undefined) {
  if (!runType) {
    return 'Run';
  }

  return runType
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

type TimelineFeedProps = {
  messages: ChatMessage[];
  errorText: string | null;
};

export default function TimelineFeed({ messages, errorText }: TimelineFeedProps) {
  const [expandedMessageIds, setExpandedMessageIds] = useState<Record<string, boolean>>({});
  const containerRef = useRef<HTMLDivElement | null>(null);
  const { chatFontSize } = useChatAppearance();
  const bodyTextClass = chatBodyTextClass(chatFontSize);
  const secondaryTextClass = chatSecondaryTextClass(chatFontSize);

  const toggleExpanded = (messageId: string) => {
    setExpandedMessageIds((previous) => ({
      ...previous,
      [messageId]: !previous[messageId],
    }));
  };

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
  }, [messages]);

  useEffect(() => {
    setExpandedMessageIds((previous) => {
      const next = { ...previous };
      let changed = false;

      for (const message of messages) {
        if (message.kind !== 'run' || !message.runMeta) {
          continue;
        }

        const shouldExpandByDefault = message.runMeta.state === 'running' || message.runMeta.state === 'pending';
        if (previous[message.id] == null && shouldExpandByDefault) {
          next[message.id] = true;
          changed = true;
        }

        if (previous[message.id] === true && message.runMeta.state === 'completed') {
          next[message.id] = false;
          changed = true;
        }
      }

      return changed ? next : previous;
    });
  }, [messages]);

  return (
    <div className="flex-1 overflow-y-auto space-y-4 px-8 py-6 no-scrollbar" ref={containerRef}>
      {messages.map((message) => {
        const isUser = message.role === 'user';
        const isAssistant = message.role === 'assistant';
        const isSystem = message.role === 'system';
        const isExpandable = Boolean(message.expandable);
        const isExpanded = Boolean(expandedMessageIds[message.id]);
        const summaryText = (message.summary ?? message.content).replace(/\s+/g, ' ').trim();

        let collapsedText = summaryText;
        if (collapsedText.length > 120) {
          collapsedText = `${collapsedText.slice(0, 117)}...`;
        }

        const showCollapsed = isSystem && isExpandable && !isExpanded;
        const displayText = showCollapsed ? collapsedText : message.details ?? message.content;
        const isToolLike = message.kind === 'tool';
        const isRunLike = message.kind === 'run';
        const runMeta = message.runMeta;
        const runSteps = runMeta?.steps ?? [];
        const showRunDetails = isRunLike && isExpanded;
        const attachments = message.attachments ?? [];

        return (
          <div
            key={message.id}
            className={`mx-auto flex max-w-3xl flex-col ${isUser ? 'items-end' : 'items-start'} w-full`}
          >
            {isAssistant ? (
              <div className="mb-2 flex items-center gap-2">
                <div className="flex h-6 w-6 items-center justify-center rounded-full border border-primary/20 bg-surface-container-high">
                  <span className="material-symbols-outlined text-[14px] text-primary" data-icon="smart_toy">
                    smart_toy
                  </span>
                </div>
                <span className="font-label text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                  Agent / Analyst
                </span>
              </div>
            ) : null}

            <div className={`flex items-start gap-3 ${isUser ? '' : 'w-full'}`}>
              <div className={isUser ? 'text-right' : ''}>
                <div
                  className={
                    isUser
                      ? ''
                      : `max-w-xl rounded-xl p-5 ${
                          isToolLike
                            ? 'border border-primary/15 bg-primary/5'
                            : isRunLike
                              ? 'border border-secondary/20 bg-secondary-container/20'
                            : 'bg-surface-container-low'
                        }`
                  }
                >
                  {isToolLike ? (
                    <div className="mb-2 flex items-center gap-2">
                      <span className="material-symbols-outlined text-[16px] text-primary" data-icon="build">
                        build
                      </span>
                      <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-primary">
                        Tool Call
                      </span>
                    </div>
                  ) : null}

                  {isRunLike ? (
                    <div className="mb-2 flex items-center gap-2">
                      <span className="material-symbols-outlined text-[16px] text-secondary" data-icon="play_circle">
                        play_circle
                      </span>
                      <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-secondary">
                        {formatRunTypeLabel(runMeta?.runType)}
                      </span>
                      {runMeta ? (
                        <span className="ml-auto text-[10px] font-bold uppercase tracking-[0.12em] text-secondary">
                          {runMeta.state}
                        </span>
                      ) : null}
                    </div>
                  ) : null}

                  {isRunLike && runMeta ? (
                    <div className="mb-3">
                      <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface-container-highest">
                        <div
                          className="h-full rounded-full bg-secondary transition-all duration-500"
                          style={{ width: `${Math.max(0, Math.min(100, runMeta.progressPercent))}%` }}
                        />
                      </div>
                      <div className="mt-1 flex justify-between text-[10px] text-on-surface-variant">
                        <span>{runMeta.runId}</span>
                        <span>{runMeta.progressPercent}%</span>
                      </div>
                      {runSteps.length > 0 && !showRunDetails ? (
                        <div className="mt-2 text-[10px] uppercase tracking-[0.12em] text-on-surface-variant">
                          {runSteps.length} tracked step{runSteps.length === 1 ? '' : 's'}
                        </div>
                      ) : null}
                      {(runMeta.stepSummary || runMeta.stepKey || runMeta.detailText) ? (
                        <div className="mt-2 rounded-lg border border-secondary/15 bg-surface-container-low px-3 py-2 text-[11px] text-on-surface-variant">
                          <p className="mb-1 text-[10px] font-bold uppercase tracking-[0.12em] text-secondary">Current Step</p>
                          <p className="font-medium text-on-surface">
                            {runMeta.stepSummary ?? runMeta.stepKey ?? 'run step'}
                          </p>
                          {showRunDetails && runMeta.detailText ? <p className="mt-1">{runMeta.detailText}</p> : null}
                          {runMeta.stepIndex != null && runMeta.stepsTotal != null ? (
                            <p className="mt-1 text-[10px] uppercase tracking-[0.12em]">
                              Step {runMeta.stepIndex}/{runMeta.stepsTotal}
                            </p>
                          ) : null}
                        </div>
                      ) : null}

                      {showRunDetails && runSteps.length > 0 ? (
                        <div className="mt-2 space-y-1">
                          {runSteps.map((step) => {
                            const isCurrent = runMeta.currentStepKey === step.key;
                            const isFailed = step.status === 'failed';
                            const isCompleted = step.status === 'completed';
                            const isRunning = step.status === 'running';
                            const statusColor =
                              isCompleted
                                ? 'text-secondary'
                                : isFailed
                                  ? 'text-[#ff8d8d]'
                                  : isRunning
                                    ? 'text-primary'
                                    : 'text-on-surface-variant';
                            return (
                              <div
                                key={step.key}
                                className="relative pl-5"
                              >
                                <div className="absolute left-[6px] top-0 bottom-0 w-px bg-outline-variant/25" />
                                <div
                                  className={`absolute left-0 top-1.5 h-3 w-3 rounded-full border ${
                                    isCompleted
                                      ? 'border-secondary/30 bg-secondary'
                                      : isFailed
                                        ? 'border-[#ff8d8d]/30 bg-[#ff8d8d]'
                                        : isRunning
                                          ? 'border-primary/30 bg-primary'
                                          : 'border-outline-variant/30 bg-surface-container-highest'
                                  }`}
                                />
                                <div
                                  className={`flex items-start justify-between rounded-md border px-2 py-1 text-[11px] ${
                                    isCurrent
                                      ? 'border-primary/25 bg-primary/5'
                                      : isFailed
                                        ? 'border-[#ff8d8d]/20 bg-[#2a1b1b]'
                                        : 'border-outline-variant/20 bg-surface-container'
                                  }`}
                                >
                                  <div className="min-w-0">
                                    <p className="truncate text-on-surface">{step.summary}</p>
                                    {!isFailed && step.detailText ? (
                                      <p className="truncate text-[10px] text-on-surface-variant">{step.detailText}</p>
                                    ) : null}
                                    {isFailed && step.detailText ? (
                                      <div className="mt-1 rounded-md border border-[#ff8d8d]/20 bg-[#221515] px-2 py-1.5 text-[10px] leading-relaxed text-[#ffb4b4]">
                                        {step.detailText}
                                      </div>
                                    ) : null}
                                  </div>
                                  <div className={`ml-2 shrink-0 text-[10px] uppercase tracking-[0.1em] ${statusColor}`}>
                                    {step.status}
                                  </div>
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      ) : null}
                    </div>
                  ) : null}

                  <p className={`max-w-xl whitespace-pre-wrap font-body leading-relaxed text-on-surface-variant ${bodyTextClass}`}>
                    {displayText || (message.pending ? '...' : '')}
                  </p>

                  {attachments.length > 0 ? (
                    <div className="mt-3 flex max-w-xl flex-wrap gap-2">
                      {attachments.map((attachment) => (
                        <div
                          key={`${message.id}-${attachment.name}`}
                          className="flex items-center gap-2 rounded-full border border-outline-variant/20 bg-surface-container-highest px-3 py-1 text-[11px] text-on-surface"
                        >
                          <span className="material-symbols-outlined text-[14px] text-primary">attach_file</span>
                          <span className="max-w-[220px] truncate">{attachment.name}</span>
                          {attachment.size != null ? (
                            <span className="text-[10px] text-on-surface-variant">
                              {Math.max(1, Math.round(attachment.size / 1024))} KB
                            </span>
                          ) : null}
                        </div>
                      ))}
                    </div>
                  ) : null}

                  {isSystem && isExpandable ? (
                    <button
                      className={`mt-2 font-bold uppercase tracking-widest text-primary ${secondaryTextClass}`}
                      onClick={() => toggleExpanded(message.id)}
                      type="button"
                    >
                      {isRunLike ? (isExpanded ? 'Hide Details' : 'Show Details') : isExpanded ? 'Collapse' : 'Expand'}
                    </button>
                  ) : null}
                </div>
              </div>

              {isUser ? <div className="mt-1 h-12 w-0.5 rounded-full bg-primary" /> : null}
            </div>
          </div>
        );
      })}

      {messages.length === 0 ? (
        <div className="mx-auto max-w-3xl rounded-xl border border-outline-variant/10 bg-surface-container-low p-6 text-sm text-on-surface-variant">
          Start a real conversation from the draft screen to populate this timeline.
        </div>
      ) : null}

      {errorText ? (
        <div className="mx-auto max-w-3xl rounded-xl border border-[#ff8d8d]/20 bg-[#2a1b1b] p-6 text-sm text-[#ffb3b3]">
          {errorText}
        </div>
      ) : null}
    </div>
  );
}
