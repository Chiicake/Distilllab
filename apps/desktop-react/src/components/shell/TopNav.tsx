import { useCallback, useEffect, useState, type MouseEvent as ReactMouseEvent } from 'react';

import brandIconDp1 from '../../assets/brand-icon-dp1.svg';
import type { Screen } from '../../app-state/screen-state';
import { useI18n } from '../../i18n/I18nProvider';

type WindowControlsApi = {
  minimize: () => Promise<void>;
  maximize: () => Promise<void>;
  unmaximize: () => Promise<void>;
  toggleMaximize: () => Promise<void>;
  isMaximized: () => Promise<boolean>;
  startDragging: () => Promise<void>;
  startResizeDragging: (direction: 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West') => Promise<void>;
  close: () => Promise<void>;
};

type TopNavProps = {
  currentScreen: Screen;
  onOpenChat: () => void;
  onOpenCanvas: () => void;
  onOpenSettings: () => void;
  canToggleLeftSidebar: boolean;
  canToggleRightSidebar: boolean;
  leftSidebarOpen: boolean;
  rightSidebarOpen: boolean;
  onToggleLeftSidebar: () => void;
  onToggleRightSidebar: () => void;
};

export default function TopNav({
  currentScreen,
  onOpenChat,
  onOpenCanvas,
  onOpenSettings,
  canToggleLeftSidebar,
  canToggleRightSidebar,
  leftSidebarOpen,
  rightSidebarOpen,
  onToggleLeftSidebar,
  onToggleRightSidebar,
}: TopNavProps) {
  const { t } = useI18n();
  const isChatActive = currentScreen.kind === 'chat-draft' || currentScreen.kind === 'chat-active';
  const isCanvasActive = currentScreen.kind === 'canvas';
  const isSettingsActive = currentScreen.kind === 'settings';
  const [windowControls, setWindowControls] = useState<WindowControlsApi | null>(null);
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const setupWindowControls = async () => {
      try {
        const module = await import('@tauri-apps/api/window');
        const appWindow = module.getCurrentWindow();
        if (cancelled) {
          return;
        }

        setWindowControls({
          minimize: () => appWindow.minimize(),
          maximize: () => appWindow.maximize(),
          unmaximize: () => appWindow.unmaximize(),
          toggleMaximize: () => appWindow.toggleMaximize(),
          isMaximized: () => appWindow.isMaximized(),
          startDragging: () => appWindow.startDragging(),
          startResizeDragging: (direction) => appWindow.startResizeDragging(direction),
          close: () => appWindow.close(),
        });

        const maximized = await appWindow.isMaximized();
        if (!cancelled) {
          setIsMaximized(maximized);
        }
        } catch (error) {
          if (!cancelled) {
            setWindowControls(null);
            if (typeof window !== 'undefined') {
              window.console.error('[TopNav] failed to initialize window controls', error);
            }
          }
        }
      };

    void setupWindowControls();

    return () => {
      cancelled = true;
    };
  }, []);

  const handleMinimize = useCallback(() => {
    if (!windowControls) {
      if (typeof window !== 'undefined') {
        window.console.warn('[TopNav] window controls unavailable: minimize ignored');
      }
      return;
    }

    void windowControls.minimize().catch((error) => {
      if (typeof window !== 'undefined') {
        window.console.error('[TopNav] minimize failed', error);
      }
    });
  }, [windowControls]);

  const handleToggleMaximize = useCallback(() => {
    if (!windowControls) {
      if (typeof window !== 'undefined') {
        window.console.warn('[TopNav] window controls unavailable: maximize ignored');
      }
      return;
    }

    void (async () => {
      try {
        await windowControls.toggleMaximize();
        const nextMaximized = await windowControls.isMaximized();
        setIsMaximized(nextMaximized);
      } catch (error) {
        if (typeof window !== 'undefined') {
          window.console.error('[TopNav] toggle maximize failed', error);
        }
      }
    })();
  }, [windowControls]);

  const handleClose = useCallback(() => {
    if (!windowControls) {
      if (typeof window !== 'undefined') {
        window.console.warn('[TopNav] window controls unavailable: close ignored');
      }
      return;
    }

    void windowControls.close().catch((error) => {
      if (typeof window !== 'undefined') {
        window.console.error('[TopNav] close failed', error);
      }
    });
  }, [windowControls]);

  const handleDragRegionMouseDown = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      if (event.button !== 0 || !windowControls) {
        return;
      }

      event.preventDefault();
      void windowControls.startDragging().catch((error) => {
        if (typeof window !== 'undefined') {
          window.console.error('[TopNav] start dragging failed', error);
        }
      });
    },
    [windowControls],
  );

  return (
    <header className="flex h-11 w-full items-center border-b border-outline-variant/25 bg-[#151616]/95 px-3 text-[#bac3ff] backdrop-blur-sm">
      <div className="tauri-no-drag flex items-center gap-3 font-['Manrope']">
        <div className="flex items-center gap-2">
          <img alt="DistillLab brand icon" className="h-5 w-5 rounded-sm" src={brandIconDp1} />
          <span className="text-[12px] font-extrabold tracking-[0.08em] text-[#cfd5ff]">DistillLab</span>
        </div>
        <nav className="hidden items-center gap-1 md:flex">
          <button
            className={
              isChatActive
                ? 'rounded-md border border-[#bac3ff]/40 bg-[#bac3ff]/10 px-2 py-1 text-[11px] font-bold uppercase tracking-[0.14em] text-[#bac3ff]'
                : 'rounded-md px-2 py-1 text-[11px] font-bold uppercase tracking-[0.14em] text-[#8f95bf] transition-colors hover:text-[#dbe1ff]'
            }
            onClick={onOpenChat}
            type="button"
          >
            {t('nav.chat')}
          </button>
          <button
            className={
              isCanvasActive
                ? 'rounded-md border border-[#bac3ff]/40 bg-[#bac3ff]/10 px-2 py-1 text-[11px] font-bold uppercase tracking-[0.14em] text-[#bac3ff]'
                : 'rounded-md px-2 py-1 text-[11px] font-bold uppercase tracking-[0.14em] text-[#8f95bf] transition-colors hover:text-[#dbe1ff]'
            }
            onClick={onOpenCanvas}
            type="button"
          >
            {t('nav.canvas')}
          </button>
        </nav>
      </div>

      <div
        className="tauri-drag-region mx-2 h-7 flex-1 rounded-md"
        data-tauri-drag-region
        onDoubleClick={handleToggleMaximize}
        onMouseDown={handleDragRegionMouseDown}
      />

      <div className="tauri-no-drag flex items-center gap-1">
        <button
          aria-label={leftSidebarOpen ? 'Hide left sidebar' : 'Show left sidebar'}
          className={`rounded-md p-1 transition-colors ${
            canToggleLeftSidebar
              ? leftSidebarOpen
                ? 'bg-[#bac3ff]/15 text-[#bac3ff] hover:text-[#dbe1ff]'
                : 'text-[#8f95bf] hover:text-[#dbe1ff]'
              : 'cursor-not-allowed text-[#58607f]/60'
          }`}
          disabled={!canToggleLeftSidebar}
          onClick={onToggleLeftSidebar}
          type="button"
        >
          <span className="material-symbols-outlined text-[18px]" data-icon="left_panel_open">
            {leftSidebarOpen ? 'left_panel_close' : 'left_panel_open'}
          </span>
        </button>

        <button
          aria-label={rightSidebarOpen ? 'Hide right sidebar' : 'Show right sidebar'}
          className={`rounded-md p-1 transition-colors ${
            canToggleRightSidebar
              ? rightSidebarOpen
                ? 'bg-[#bac3ff]/15 text-[#bac3ff] hover:text-[#dbe1ff]'
                : 'text-[#8f95bf] hover:text-[#dbe1ff]'
              : 'cursor-not-allowed text-[#58607f]/60'
          }`}
          disabled={!canToggleRightSidebar}
          onClick={onToggleRightSidebar}
          type="button"
        >
          <span className="material-symbols-outlined text-[18px]" data-icon="right_panel_open">
            {rightSidebarOpen ? 'right_panel_close' : 'right_panel_open'}
          </span>
        </button>

        <button
          aria-label={t('nav.settings')}
          className={
            isSettingsActive
              ? 'rounded-md bg-[#bac3ff]/15 p-1 text-[#bac3ff] transition-colors'
              : 'rounded-md p-1 text-[#8f95bf] transition-colors hover:text-[#dbe1ff]'
          }
          onClick={onOpenSettings}
          type="button"
        >
          <span className="material-symbols-outlined text-[18px]" data-icon="settings">
            settings
          </span>
        </button>

        {windowControls ? (
          <div className="ml-1 flex items-center rounded-md border border-outline-variant/20 bg-surface-container/40">
            <button
              aria-label="Minimize window"
              className="window-control-btn"
              onClick={handleMinimize}
              type="button"
            >
              <span className="material-symbols-outlined text-[14px]">remove</span>
            </button>
            <button
              aria-label="Toggle maximize window"
              className="window-control-btn"
              onClick={handleToggleMaximize}
              type="button"
            >
              <span className="material-symbols-outlined text-[12px]">
                {isMaximized ? 'filter_none' : 'crop_square'}
              </span>
            </button>
            <button
              aria-label="Close window"
              className="window-control-btn hover:bg-[#d65f5f]/25 hover:text-[#ffd9d9]"
              onClick={handleClose}
              type="button"
            >
              <span className="material-symbols-outlined text-[14px]">close</span>
            </button>
          </div>
        ) : null}
      </div>
    </header>
  );
}
