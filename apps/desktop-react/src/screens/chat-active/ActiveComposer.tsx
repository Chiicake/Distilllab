import { useState } from 'react';

import { useChat } from '../../chat/ChatProvider';
import { mergePendingAttachments, pickPendingAttachments, type PendingAttachment } from '../chat/pending-attachments';

type ActiveComposerProps = {
  onSend: (message: string, attachmentPaths: string[]) => Promise<void>;
  isStreaming: boolean;
};

export default function ActiveComposer({ onSend, isStreaming }: ActiveComposerProps) {
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
    <div className="px-8 pb-8 pt-4">
      <div className="max-w-3xl mx-auto relative">
        <div className="bg-surface-container-low rounded-xl p-4 flex flex-col gap-3 focus-within:ring-1 ring-primary/30 transition-all">
          {attachments.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {attachments.map((attachment) => (
                <div
                  key={attachment.path}
                  className="flex items-center gap-2 rounded-full border border-outline-variant/20 bg-surface-container-highest px-3 py-1 text-[11px] text-on-surface"
                >
                  <span className="material-symbols-outlined text-[14px] text-primary">attach_file</span>
                  <span className="max-w-[220px] truncate">{attachment.name}</span>
                  <button
                    className="text-on-surface-variant transition-colors hover:text-on-surface"
                    onClick={() => {
                      setAttachments((previous) => previous.filter((item) => item.path !== attachment.path));
                    }}
                    type="button"
                  >
                    <span className="material-symbols-outlined text-[14px]">close</span>
                  </button>
                </div>
              ))}
            </div>
          ) : null}

          <textarea
            aria-label="Type a command or follow-up question"
            className="bg-transparent border-none focus:ring-0 text-on-surface placeholder:text-outline/50 resize-none font-body text-md h-12 w-full"
            onChange={(event) => setMessage(event.target.value)}
            onKeyDown={(event) => {
              if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
                event.preventDefault();
                void submit();
              }
            }}
            placeholder="Type a command or follow-up question..."
            value={message}
          />

          <div className="flex items-center justify-between">
            <div className="flex gap-2">
              <button
                className="p-1.5 text-on-surface-variant hover:text-primary transition-colors"
                onClick={() => {
                  void (async () => {
                    const picked = await pickPendingAttachments();
                    setAttachments((previous) => mergePendingAttachments(previous, picked));
                  })();
                }}
                type="button"
              >
                <span className="material-symbols-outlined text-lg" data-icon="attach_file">
                  attach_file
                </span>
              </button>

              <button className="p-1.5 text-on-surface-variant hover:text-primary transition-colors" type="button">
                <span className="material-symbols-outlined text-lg" data-icon="library_add">
                  library_add
                </span>
              </button>
            </div>

            <button
              className={`px-5 py-2 rounded-lg font-label font-bold text-xs uppercase tracking-widest transition-opacity ${
                isStreaming
                  ? 'bg-[#ff8d8d] text-[#2f1212] hover:opacity-90'
                  : 'gradient-primary text-on-primary-container hover:opacity-90'
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
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
