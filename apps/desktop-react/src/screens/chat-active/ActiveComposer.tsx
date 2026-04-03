import { useState } from 'react';

type ActiveComposerProps = {
  onSend: (message: string) => Promise<void>;
  isStreaming: boolean;
};

export default function ActiveComposer({ onSend, isStreaming }: ActiveComposerProps) {
  const [message, setMessage] = useState('');

  const submit = async () => {
    const trimmed = message.trim();
    if (!trimmed || isStreaming) {
      return;
    }

    await onSend(trimmed);
    setMessage('');
  };

  return (
    <div className="px-8 pb-8 pt-4">
      <div className="max-w-3xl mx-auto relative">
        <div className="bg-surface-container-low rounded-xl p-4 flex flex-col gap-3 focus-within:ring-1 ring-primary/30 transition-all">
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
              <button className="p-1.5 text-on-surface-variant hover:text-primary transition-colors" type="button">
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
              className="gradient-primary px-5 py-2 rounded-lg font-label font-bold text-xs uppercase tracking-widest text-on-primary-container hover:opacity-90 transition-opacity"
              disabled={isStreaming}
              onClick={() => {
                void submit();
              }}
              type="button"
            >
              {isStreaming ? 'Sending' : 'Send'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
