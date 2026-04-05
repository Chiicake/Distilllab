import { useEffect, useRef, useState } from 'react';

import { useI18n } from '../../i18n/I18nProvider';

type SessionActionMenuProps = {
  active: boolean;
  pinned: boolean;
  onRename: () => void;
  onDelete: () => void;
  onTogglePin: () => void;
};

export default function SessionActionMenu({
  active,
  pinned,
  onRename,
  onDelete,
  onTogglePin,
}: SessionActionMenuProps) {
  const { t } = useI18n();
  const [menuOpen, setMenuOpen] = useState(false);
  const [focusedIndex, setFocusedIndex] = useState(0);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const actions = [
    {
      label: 'Rename',
      localizedLabel: t('session.menu.rename'),
      icon: 'edit',
      onSelect: () => {
        setMenuOpen(false);
        onRename();
      },
      danger: false,
    },
    {
      label: pinned ? 'Unpin' : 'Pin to top',
      localizedLabel: pinned ? t('session.menu.unpin') : t('session.menu.pin'),
      icon: 'keep',
      onSelect: () => {
        setMenuOpen(false);
        onTogglePin();
      },
      danger: false,
    },
    {
      label: 'Delete',
      localizedLabel: t('session.menu.delete'),
      icon: 'delete',
      onSelect: () => {
        setMenuOpen(false);
        onDelete();
      },
      danger: true,
    },
  ];

  useEffect(() => {
    if (!menuOpen) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (!menuOpen) {
        return;
      }

      if (event.key === 'Escape') {
        event.preventDefault();
        setMenuOpen(false);
      }

      if (event.key === 'ArrowDown') {
        event.preventDefault();
        setFocusedIndex((previous) => (previous + 1) % 3);
      }

      if (event.key === 'ArrowUp') {
        event.preventDefault();
        setFocusedIndex((previous) => (previous + 2) % 3);
      }

      if (event.key === 'Enter') {
        event.preventDefault();
        actions[focusedIndex]?.onSelect();
      }
    };

    window.addEventListener('mousedown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('mousedown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [menuOpen]);

  return (
    <div className="relative tauri-no-drag shrink-0" ref={containerRef}>
      <button
        aria-expanded={menuOpen}
        aria-haspopup="menu"
        aria-label={t('session.menu.actions')}
        className={`rounded-md p-1 transition-colors ${
          menuOpen || active ? 'text-[#dbe1ff]' : 'text-[#8f95bf] opacity-0 group-hover:opacity-100'
        } hover:bg-[#2a2b2b] hover:text-[#f3faff]`}
        onClick={(event) => {
          event.stopPropagation();
          setFocusedIndex(0);
          setMenuOpen((previous) => !previous);
        }}
        type="button"
      >
        <span className="material-symbols-outlined text-[18px]" data-icon="more_horiz">
          more_horiz
        </span>
      </button>

      {menuOpen ? (
        <div
          className="absolute right-0 top-9 z-50 min-w-[168px] rounded-lg border border-outline-variant/20 bg-[#171818] p-1 shadow-[0_12px_32px_rgba(0,0,0,0.35)]"
          role="menu"
        >
          {actions.map((action, index) => (
            <button
              key={action.label}
              className={`flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs ${
                action.danger ? 'text-[#ffb4b4] hover:bg-[#2b2020]' : 'text-on-surface hover:bg-[#232424]'
              } ${focusedIndex === index ? 'bg-[#232424]' : ''}`}
              onClick={action.onSelect}
              onMouseEnter={() => setFocusedIndex(index)}
              role="menuitem"
              type="button"
            >
              <span className="material-symbols-outlined text-[16px]">{action.icon}</span>
              <span>{action.localizedLabel}</span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
