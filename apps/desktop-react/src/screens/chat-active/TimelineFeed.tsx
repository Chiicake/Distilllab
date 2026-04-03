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
