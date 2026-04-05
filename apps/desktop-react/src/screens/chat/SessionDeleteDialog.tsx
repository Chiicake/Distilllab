import { useEffect } from 'react';

import { useI18n } from '../../i18n/I18nProvider';

type SessionDeleteDialogProps = {
  open: boolean;
  sessionTitle: string;
  onClose: () => void;
  onDelete: () => void;
};

export default function SessionDeleteDialog({ open, sessionTitle, onClose, onDelete }: SessionDeleteDialogProps) {
  const { t } = useI18n();
  useEffect(() => {
    if (!open) {
      return;
    }

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
  }, [onClose, open]);

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
          <p className="text-[11px] font-bold uppercase tracking-[0.16em] text-[#ffb4b4]">{t('session.dialog.delete.badge')}</p>
          <h2 className="mt-1 text-lg font-bold text-on-surface">{t('session.dialog.delete.title')}</h2>
          <p className="mt-2 text-sm leading-relaxed text-on-surface-variant">
            {t('session.dialog.delete.descriptionPrefix')} <span className="font-semibold text-on-surface">{sessionTitle}</span>{' '}
            {t('session.dialog.delete.descriptionSuffix')}
          </p>
        </div>

        <div className="flex justify-end gap-2">
          <button
            className="rounded-xl border border-outline-variant/20 px-4 py-2 text-sm text-on-surface-variant transition-colors hover:bg-surface-container"
            onClick={onClose}
            type="button"
          >
            {t('common.cancel')}
          </button>
          <button
            className="rounded-xl bg-[#ffb4b4] px-4 py-2 text-sm font-semibold text-[#311111] transition-opacity hover:opacity-90"
            onClick={onDelete}
            type="button"
          >
            {t('session.menu.delete')}
          </button>
        </div>
      </div>
    </div>
  );
}
