import { useEffect, useRef, useState } from 'react';

import { useI18n } from '../../i18n/I18nProvider';

type SessionRenameDialogProps = {
  currentTitle: string;
  open: boolean;
  onClose: () => void;
  onSave: (value: string | null) => void;
};

export default function SessionRenameDialog({ currentTitle, open, onClose, onSave }: SessionRenameDialogProps) {
  const { t } = useI18n();
  const [value, setValue] = useState(currentTitle);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    setValue(currentTitle);
    window.setTimeout(() => inputRef.current?.focus(), 0);

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [currentTitle, onClose, open]);

  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/45 px-4" onMouseDown={onClose}>
      <div
        className="w-full max-w-md rounded-2xl border border-outline-variant/20 bg-[#161717] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.45)]"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="mb-4">
          <p className="text-[11px] font-bold uppercase tracking-[0.16em] text-primary">{t('session.dialog.rename.badge')}</p>
          <h2 className="mt-1 text-lg font-bold text-on-surface">{t('session.dialog.rename.title')}</h2>
          <p className="mt-2 text-sm leading-relaxed text-on-surface-variant">
            {t('session.dialog.rename.description')}
          </p>
        </div>

        <label className="mb-4 block">
          <span className="mb-2 block text-xs font-medium text-on-surface-variant">{t('session.dialog.rename.field')}</span>
          <input
            ref={inputRef}
            className="w-full rounded-xl border border-outline-variant/20 bg-surface-container px-3 py-2.5 text-sm text-on-surface outline-none transition-colors focus:border-primary/40"
            onChange={(event) => setValue(event.target.value)}
            placeholder={t('session.dialog.rename.placeholder')}
            value={value}
          />
        </label>

        <div className="flex justify-end gap-2">
          <button
            className="rounded-xl border border-outline-variant/20 px-4 py-2 text-sm text-on-surface-variant transition-colors hover:bg-surface-container"
            onClick={onClose}
            type="button"
          >
            {t('common.cancel')}
          </button>
          <button
            className="rounded-xl bg-primary px-4 py-2 text-sm font-semibold text-[#0f1014] transition-opacity hover:opacity-90"
            onClick={() => onSave(value.trim() ? value.trim() : null)}
            type="button"
          >
            {t('common.save')}
          </button>
        </div>
      </div>
    </div>
  );
}
