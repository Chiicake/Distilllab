import { useEffect, useRef, useState } from 'react';

import type { ChatMessage } from '../../chat/types';

type TimelineFeedProps = {
  messages: ChatMessage[];
  errorText: string | null;
};

export default function TimelineFeed({ messages, errorText }: TimelineFeedProps) {
  const [expandedMessageIds, setExpandedMessageIds] = useState<Record<string, boolean>>({});
  const containerRef = useRef<HTMLDivElement | null>(null);

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

  return (
    <div className="flex-1 overflow-y-auto space-y-12 px-8 py-10 no-scrollbar" ref={containerRef}>
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

        return (
          <div
            key={message.id}
            className={`mx-auto flex max-w-3xl flex-col ${isUser ? 'items-end' : 'items-start'} w-full`}
          >
            {isAssistant ? (
              <div className="mb-3 flex items-center gap-2">
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

            <div className={`flex items-start gap-4 ${isUser ? '' : 'w-full'}`}>
              <div className={isUser ? 'text-right' : ''}>
                <div
                  className={
                    isUser
                      ? ''
                      : `max-w-xl rounded-xl p-6 ${
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
                        Run
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
                      {(runMeta.stepSummary || runMeta.stepKey || runMeta.detailText) ? (
                        <div className="mt-2 rounded-lg border border-secondary/15 bg-surface-container-low px-3 py-2 text-[11px] text-on-surface-variant">
                          <p className="font-medium text-on-surface">
                            {runMeta.stepSummary ?? runMeta.stepKey ?? 'run step'}
                          </p>
                          {runMeta.detailText ? <p className="mt-1">{runMeta.detailText}</p> : null}
                          {runMeta.stepIndex != null && runMeta.stepsTotal != null ? (
                            <p className="mt-1 text-[10px] uppercase tracking-[0.12em]">
                              Step {runMeta.stepIndex}/{runMeta.stepsTotal}
                            </p>
                          ) : null}
                        </div>
                      ) : null}
                    </div>
                  ) : null}

                  <p className="max-w-xl whitespace-pre-wrap font-body text-md leading-relaxed text-on-surface-variant">
                    {displayText || (message.pending ? '...' : '')}
                  </p>

                  {isSystem && isExpandable ? (
                    <button
                      className="mt-2 text-[10px] font-bold uppercase tracking-widest text-primary"
                      onClick={() => toggleExpanded(message.id)}
                      type="button"
                    >
                      {isExpanded ? 'Collapse' : 'Expand'}
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
