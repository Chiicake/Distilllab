import type { SettingsSection } from '../../app-state/screen-state';
import { useI18n } from '../../i18n/I18nProvider';

type SettingsSidebarProps = {
  section: SettingsSection;
  onChangeSection: (section: SettingsSection) => void;
};

export default function SettingsSidebar({ section, onChangeSection }: SettingsSidebarProps) {
  const { t } = useI18n();

  return (
    <aside className="flex h-full w-64 shrink-0 flex-col gap-2 border-r border-[#484848]/15 bg-[#191a1a] p-4 font-body text-sm font-medium">
      <div className="mb-8 flex items-center gap-3 px-2">
        <div className="flex h-8 w-8 items-center justify-center rounded-sm bg-gradient-to-br from-primary to-primary-container">
          <div className="relative h-4 w-4">
            <span className="absolute left-0 top-0 h-1.5 w-1.5 rounded-[2px] bg-on-primary" />
            <span className="absolute right-0 top-0 h-1.5 w-1.5 rounded-[2px] bg-on-primary/80" />
            <span className="absolute left-1/2 top-1/2 h-2 w-2 -translate-x-1/2 -translate-y-1/2 rounded-[2px] bg-on-primary" />
          </div>
        </div>
        <div>
          <h1 className="font-headline text-xs font-bold uppercase tracking-widest text-[#f3faff]">
            Distilllab
          </h1>
          <p className="text-[10px] uppercase tracking-[0.18em] text-on-surface-variant opacity-80">
            Distill Work Into Structure
          </p>
        </div>
      </div>

      <nav className="flex-1 space-y-1">
        <div className="flex items-center gap-3 px-4 py-2 text-[#acabaa] transition-all hover:bg-[#1f2020]/50">
          <span className="material-symbols-outlined text-lg">person</span>
          <span>{t('settings.sidebar.profile')}</span>
        </div>

        <button
          className={`flex w-full items-center gap-3 rounded-md px-4 py-2 text-left transition-all ${
            section === 'workspace'
              ? 'bg-[#1f2020] text-[#f3faff]'
              : 'text-[#acabaa] hover:bg-[#1f2020]/50'
          }`}
          onClick={() => onChangeSection('workspace')}
          type="button"
        >
          <span className="material-symbols-outlined text-lg">database</span>
          <span>{t('settings.sidebar.workspace')}</span>
        </button>

        <div className="flex items-center gap-3 px-4 py-2 text-[#acabaa] transition-all hover:bg-[#1f2020]/50">
          <span className="material-symbols-outlined text-lg">notifications</span>
          <span>{t('settings.sidebar.notifications')}</span>
        </div>

        <button
          className={`flex w-full items-center gap-3 rounded-md px-4 py-2 text-left transition-all ${
            section === 'debug'
              ? 'bg-[#1f2020] text-[#f3faff]'
              : 'text-[#acabaa] hover:bg-[#1f2020]/50'
          }`}
          onClick={() => onChangeSection('debug')}
          type="button"
        >
          <span className="material-symbols-outlined text-lg">settings_suggest</span>
          <span>{t('settings.sidebar.system')}</span>
        </button>
      </nav>

      <div className="mt-auto space-y-1 pt-4">
        <div className="flex items-center gap-3 px-4 py-2 text-[#acabaa] transition-all hover:bg-[#1f2020]/50">
          <span className="material-symbols-outlined text-lg">help</span>
          <span>{t('settings.sidebar.help')}</span>
        </div>
      </div>
    </aside>
  );
}
