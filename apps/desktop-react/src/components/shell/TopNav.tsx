import brandIconDp1 from '../../assets/brand-icon-dp1.svg';
import type { Screen } from '../../app-state/screen-state';
import { useI18n } from '../../i18n/I18nProvider';

type TopNavProps = {
  currentScreen: Screen;
  onOpenChat: () => void;
  onOpenCanvas: () => void;
  onOpenSettings: () => void;
};

export default function TopNav({ currentScreen, onOpenChat, onOpenCanvas, onOpenSettings }: TopNavProps) {
  const { t } = useI18n();
  const isChatActive = currentScreen.kind === 'chat-draft' || currentScreen.kind === 'chat-active';
  const isCanvasActive = currentScreen.kind === 'canvas';
  const isSettingsActive = currentScreen.kind === 'settings';

  return (
    <header className="bg-[#0e0e0e] text-[#bac3ff] font-['Manrope'] tracking-wider uppercase text-sm font-bold docked w-full top-0 h-16 no-border tonal-shift bg-[#191a1a] flat no shadows flex justify-between items-center px-6 w-full max-w-full z-50">
      <div className="flex items-center gap-8">
        <div className="flex items-center gap-3">
          <img alt="DistillLab brand icon" className="h-8 w-8 rounded-sm" src={brandIconDp1} />
          <span className="text-xl font-black tracking-tighter text-[#bac3ff] normal-case">DistillLab</span>
        </div>
        <nav className="hidden md:flex gap-6 items-center lowercase tracking-normal">
          <button
            className={
              isChatActive
                ? 'text-[#bac3ff] border-b-2 border-[#bac3ff] pb-1 hover:text-[#f3faff] transition-colors scale-95 duration-200'
                : 'text-[#acabaa] opacity-60 hover:text-[#f3faff] transition-colors scale-95 duration-200'
            }
            onClick={onOpenChat}
            type="button"
          >
            {t('nav.chat')}
          </button>
          <button
            className={
              isCanvasActive
                ? 'text-[#bac3ff] border-b-2 border-[#bac3ff] pb-1 hover:text-[#f3faff] transition-colors scale-95 duration-200'
                : 'text-[#acabaa] opacity-60 hover:text-[#f3faff] transition-colors scale-95 duration-200'
            }
            onClick={onOpenCanvas}
            type="button"
          >
            {t('nav.canvas')}
          </button>
        </nav>
      </div>
      <div className="flex items-center gap-4">
        <button
          aria-label={t('nav.notifications')}
          className="text-[#acabaa] opacity-60 hover:text-[#f3faff] transition-colors"
        >
          <span className="material-symbols-outlined" data-icon="notifications">
            notifications
          </span>
        </button>
        <button
          aria-label={t('nav.settings')}
          className={
            isSettingsActive
              ? 'text-[#bac3ff] hover:text-[#f3faff] transition-colors'
              : 'text-[#acabaa] opacity-60 hover:text-[#f3faff] transition-colors'
          }
          onClick={onOpenSettings}
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="settings">
            settings
          </span>
        </button>
        <div className="w-8 h-8 rounded-full overflow-hidden bg-surface-container-highest flex items-center justify-center border border-outline-variant/10">
          <img
            alt="User profile"
            className="w-full h-full object-cover"
            data-alt="professional portrait of a creative technologist in a minimalist studio environment with neutral lighting"
            src="https://lh3.googleusercontent.com/aida-public/AB6AXuBLE1X6_qx-lu7Xt3Zg2kw2IJTC6EWjIwuvAzJKqJmWeI1giXx2k46EZQhuGlNNeL0oBJumtPz9osVMI-y_pVbIHYKSGLbeOlqVHQWCZ80k1OScacXnpHTc2SKGj5UWNSS_45KLysTfEjeh29fSqtI0vAyOH37VcEJmz4U4vdFj4e5S0zRo_WsECGpZR9C7zdH4d16aX97HtkNQV_WlE_KsWb_1ioFgbNpzA_ldzuOMpIYvad3EsxF8qZrCCD8NNAhrdv7ppR5hchQ"
          />
        </div>
      </div>
    </header>
  );
}
