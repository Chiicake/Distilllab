import { useState } from 'react';

import { useChat } from '../../chat/ChatProvider';
import { mergePendingAttachments, pickPendingAttachments, type PendingAttachment } from '../chat/pending-attachments';

type DraftComposerProps = {
  onSend: (message: string, attachmentPaths: string[]) => Promise<void>;
  isStreaming: boolean;
  errorText: string | null;
};

export default function DraftComposer({ onSend, isStreaming, errorText }: DraftComposerProps) {
  const { cancelActiveRequest } = useChat();
  const [message, setMessage] = useState('');
  const [attachments, setAttachments] = useState<PendingAttachment[]>([]);

  const submit = async () => {
    const trimmed = message.trim();
    if (!trimmed || isStreaming) {
      return;
    }

    await onSend(trimmed, attachments.map((attachment) => attachment.path));
    setMessage('');
  };

  return (
    <div className="bg-gradient-to-t from-surface to-transparent p-6">
      <div className="relative mx-auto max-w-4xl">
        <div className="overflow-hidden rounded-2xl bg-surface-container-high shadow-2xl transition-all focus-within:ring-1 focus-within:ring-primary/30">
          {attachments.length > 0 ? (
            <div className="flex flex-wrap gap-2 border-b border-outline-variant/10 px-5 py-3">
              {attachments.map((attachment) => (
                <div
                  key={attachment.path}
                  className="flex items-center gap-2 rounded-full border border-outline-variant/20 bg-surface-container-highest px-3 py-1 text-[11px] text-on-surface"
                >
                  <span aria-hidden="true" className="material-symbols-outlined text-[14px] text-primary" data-icon="attach_file">
                    attach_file
                  </span>
                  <span className="max-w-[220px] truncate">{attachment.name}</span>
                  <button
                    className="text-on-surface-variant transition-colors hover:text-on-surface"
                    onClick={() => {
                      setAttachments((previous) => previous.filter((item) => item.path !== attachment.path));
                    }}
                    type="button"
                  >
                    <span aria-hidden="true" className="material-symbols-outlined text-[14px]" data-icon="close">
                      close
                    </span>
                  </button>
                </div>
              ))}
            </div>
          ) : null}

          <textarea
            aria-label="Describe the work you want to distill into structure"
            className="min-h-[64px] w-full resize-none border-none bg-transparent p-5 font-body text-on-surface placeholder:text-on-surface-variant/40 focus:ring-0"
            onChange={(event) => setMessage(event.target.value)}
            onKeyDown={(event) => {
              if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
                event.preventDefault();
                void submit();
              }
            }}
            placeholder="Describe the work you want to distill into structure..."
            rows={1}
            value={message}
          />

          <div className="flex items-center justify-between bg-surface-container-highest/50 px-5 py-3">
            <div className="flex gap-4">
              <button
                className="flex items-center gap-1 text-xs text-on-surface-variant transition-colors hover:text-on-surface"
                onClick={() => {
                  void (async () => {
                    const picked = await pickPendingAttachments();
                    setAttachments((previous) => mergePendingAttachments(previous, picked));
                  })();
                }}
                type="button"
              >
                <span aria-hidden="true" className="material-symbols-outlined" data-icon="attach_file">
                  attach_file
                </span>
                Attach
              </button>

              <button
                className="flex items-center gap-1 text-xs text-on-surface-variant transition-colors hover:text-on-surface"
                type="button"
              >
                <span aria-hidden="true" className="material-symbols-outlined" data-icon="memory">
                  memory
                </span>
                Context
              </button>
            </div>

            <button
              className={`flex items-center gap-2 rounded-lg px-4 py-1.5 text-xs font-bold uppercase tracking-widest transition-all ${
                isStreaming
                  ? 'bg-[#ff8d8d] text-[#2f1212] hover:opacity-90'
                  : 'bg-primary text-on-primary hover:brightness-110'
              }`}
              onClick={() => {
                if (isStreaming) {
                  void cancelActiveRequest();
                  return;
                }

                void submit();
              }}
              type="button"
            >
              {isStreaming ? 'Stop' : 'Send'}
              <span aria-hidden="true" className="material-symbols-outlined" data-icon="arrow_forward">
                {isStreaming ? 'stop' : 'arrow_forward'}
              </span>
            </button>
          </div>
        </div>

        {errorText ? <p className="mt-3 text-sm text-[#ff8d8d]">{errorText}</p> : null}

        <div className="mt-3 text-center text-[10px] uppercase tracking-wide text-on-surface-variant/40">
          Press <kbd className="rounded border border-outline-variant/20 bg-surface px-1.5 py-0.5">Ctrl/Cmd</kbd> +{' '}
          <kbd className="rounded border border-outline-variant/20 bg-surface px-1.5 py-0.5">Enter</kbd> to send
        </div>
      </div>
    </div>
  );
}
