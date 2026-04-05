import type { SettingsSection } from '../../app-state/screen-state';
import { useChatAppearance, type ChatFontSize } from '../../chat/ChatAppearanceProvider';
import { useI18n } from '../../i18n/I18nProvider';
import DebugPanel from './DebugPanel';
import SettingsSidebar from './SettingsSidebar';

const chatFontSizeOptions: Array<{ value: ChatFontSize; label: string; description: string }> = [
  { value: 'small', label: 'Small', description: 'Denser chat reading layout' },
  { value: 'medium', label: 'Medium', description: 'Balanced chat reading size' },
  { value: 'large', label: 'Large', description: 'Roomier chat reading size' },
];

type SettingsScreenProps = {
  section?: SettingsSection;
  onChangeSection: (section: SettingsSection) => void;
  showLeftSidebar: boolean;
};

function WorkspaceSettingsView() {
  const { locale, localeOptions, setLocale, t } = useI18n();
  const { chatFontSize, setChatFontSize } = useChatAppearance();

  return (
    <div className="w-full max-w-3xl space-y-16">
      <header>
        <h2 className="font-headline text-4xl font-extrabold tracking-tighter text-on-surface">
          {t('settings.workspace.title')}
        </h2>
        <p className="mt-2 max-w-lg font-body text-on-surface-variant">
          {t('settings.workspace.description')}
        </p>
      </header>

      <div className="space-y-12">
        <section className="space-y-6">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold uppercase tracking-widest text-secondary-dim">
              01 / {t('settings.section.identity')}
            </span>
          </div>
          <div className="rounded-sm bg-surface-container p-8">
            <label className="mb-3 block font-label text-sm font-semibold uppercase tracking-wider text-on-surface-variant">
              {t('settings.field.workspaceName')}
            </label>
            <input
              className="w-full rounded-sm border-0 bg-surface-container-low p-4 font-body text-on-surface outline-none transition-all focus:ring-1 focus:ring-primary"
              defaultValue="Technical Atelier Alpha"
              type="text"
            />
            <p className="mt-2 text-xs text-on-surface-variant/60">
              {t('settings.field.workspaceNameHint')}
            </p>
          </div>
        </section>

        <section className="space-y-6">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold uppercase tracking-widest text-secondary-dim">
              02 / {t('settings.section.appearance')}
            </span>
          </div>
          <div className="flex items-center justify-between rounded-sm bg-surface-container p-8">
            <div>
              <h4 className="font-semibold text-on-surface">{t('settings.field.visualMode')}</h4>
              <p className="text-sm text-on-surface-variant">{t('settings.field.visualModeHint')}</p>
            </div>
            <div className="flex rounded-sm bg-surface-container-low p-1">
              <div className="px-4 py-2 text-sm text-on-surface-variant">
                {t('settings.theme.system')}
              </div>
              <div className="rounded-sm bg-surface-container-high px-4 py-2 text-sm font-medium text-primary shadow-sm">
                {t('settings.theme.darkMode')}
              </div>
            </div>
          </div>
        </section>

        <section className="space-y-6">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold uppercase tracking-widest text-secondary-dim">
              03 / {t('settings.section.dataIntegrity')}
            </span>
          </div>
          <div className="rounded-sm bg-surface-container p-8">
            <div className="mb-4 flex justify-between gap-8">
              <div>
                <h4 className="font-semibold text-on-surface">{t('settings.field.archiveFrequency')}</h4>
                <p className="text-sm text-on-surface-variant">{t('settings.field.archiveFrequencyHint')}</p>
              </div>
              <span className="font-label font-bold text-primary">
                {t('settings.field.archiveFrequencyValue')}
              </span>
            </div>
            <input
              className="h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-surface-container-low accent-primary"
              defaultValue={30}
              max={90}
              min={7}
              type="range"
            />
            <div className="mt-2 flex justify-between text-[10px] font-bold uppercase tracking-widest text-on-surface-variant/40">
              <span>{t('settings.field.archiveFrequencyMin')}</span>
              <span>{t('settings.field.archiveFrequencyMax')}</span>
            </div>
          </div>
        </section>

        <section className="space-y-6">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold uppercase tracking-widest text-secondary-dim">
              04 / {t('settings.section.security')}
            </span>
          </div>
          <div className="flex items-center justify-between rounded-sm border-l-2 border-primary-container bg-surface-container p-8">
            <div>
              <h4 className="font-semibold text-on-surface">{t('settings.field.searchVisibility')}</h4>
              <p className="text-sm text-on-surface-variant">{t('settings.field.searchVisibilityHint')}</p>
            </div>
            <label className="relative inline-flex cursor-pointer items-center">
              <input checked className="peer sr-only" readOnly type="checkbox" />
              <div className="h-6 w-11 rounded-full bg-surface-container-high peer-checked:bg-primary after:absolute after:left-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:bg-white after:transition-all after:content-[''] peer-checked:after:translate-x-full" />
            </label>
          </div>
        </section>

        <section className="space-y-6">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold uppercase tracking-widest text-secondary-dim">
              05 / Chat Typography
            </span>
          </div>
          <div className="rounded-sm border-l-2 border-primary bg-surface-container p-8">
            <div className="flex items-center justify-between gap-8">
              <div>
                <h4 className="font-semibold text-on-surface">Chat font size</h4>
                <p className="text-sm text-on-surface-variant">Adjusts message and composer text in chat screens only.</p>
              </div>
              <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-primary">
                {chatFontSize}
              </span>
            </div>

            <div className="mt-4 flex flex-wrap gap-2">
              {chatFontSizeOptions.map((option) => {
                const isActive = option.value === chatFontSize;

                return (
                  <button
                    key={option.value}
                    className={`rounded-sm border px-4 py-2 text-xs font-semibold uppercase tracking-[0.12em] transition-colors ${
                      isActive
                        ? 'border-primary bg-primary/10 text-primary'
                        : 'border-outline-variant/40 text-on-surface-variant hover:border-primary/40 hover:text-on-surface'
                    }`}
                    onClick={() => setChatFontSize(option.value)}
                    type="button"
                  >
                    {option.label}
                  </button>
                );
              })}
            </div>

            <p className="mt-3 text-xs text-on-surface-variant/70">
              {chatFontSizeOptions.find((option) => option.value === chatFontSize)?.description}
            </p>
          </div>
        </section>

        <section className="space-y-6">
          <div className="flex items-center gap-2">
            <span className="text-xs font-bold uppercase tracking-widest text-secondary-dim">
              06 / {t('settings.section.language')}
            </span>
          </div>
          <div className="rounded-sm border-l-2 border-secondary-container bg-surface-container p-8">
            <div className="flex items-center justify-between gap-8">
              <div>
                <h4 className="font-semibold text-on-surface">{t('settings.field.language')}</h4>
                <p className="text-sm text-on-surface-variant">{t('settings.field.languageHint')}</p>
              </div>
              <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-primary">
                {t('settings.field.languageActive')}
              </span>
            </div>

            <div className="mt-4 flex flex-wrap gap-2">
              {localeOptions.map((option) => {
                const isActive = option.value === locale;

                return (
                  <button
                    key={option.value}
                    className={`rounded-sm border px-4 py-2 text-xs font-semibold uppercase tracking-[0.12em] transition-colors ${
                      isActive
                        ? 'border-primary bg-primary/10 text-primary'
                        : 'border-outline-variant/40 text-on-surface-variant hover:border-primary/40 hover:text-on-surface'
                    }`}
                    onClick={() => setLocale(option.value)}
                    type="button"
                  >
                    {option.label}
                  </button>
                );
              })}
            </div>
          </div>
        </section>

        <div className="flex justify-end gap-4 border-t border-outline-variant/10 pt-8">
          <button
            className="px-6 py-2 font-label text-sm uppercase tracking-widest text-on-surface-variant transition-colors hover:text-on-surface"
            type="button"
          >
            {t('settings.action.discard')}
          </button>
          <button
            className="rounded-sm bg-gradient-to-br from-primary to-primary-container px-8 py-3 text-xs font-bold uppercase tracking-widest text-on-primary shadow-lg transition-all active:scale-95"
            type="button"
          >
            {t('settings.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
}

function DebugSettingsView({ onReturnToWorkspace }: { onReturnToWorkspace: () => void }) {
  const { t } = useI18n();

  return (
    <div className="w-full max-w-5xl space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <p className="text-xs font-bold uppercase tracking-[0.16em] text-on-surface-variant">
            {t('settings.debug.sectionLabel')}
          </p>
        </div>
        <button
          className="rounded-sm border border-outline-variant/40 bg-surface-container-high px-4 py-2 text-xs font-semibold uppercase tracking-[0.12em] text-on-surface-variant transition-colors hover:border-primary/40 hover:text-primary"
          onClick={onReturnToWorkspace}
          type="button"
        >
          {t('settings.debug.back')}
        </button>
      </header>

      <DebugPanel />
    </div>
  );
}

export default function SettingsScreen({ section = 'workspace', onChangeSection, showLeftSidebar }: SettingsScreenProps) {
  const activeSection: SettingsSection = section === 'debug' ? 'debug' : 'workspace';

  return (
    <div className="flex min-w-0 flex-1 overflow-hidden bg-surface text-on-surface">
      {showLeftSidebar ? <SettingsSidebar section={activeSection} onChangeSection={onChangeSection} /> : null}

      <main className="flex min-w-0 flex-1 overflow-y-auto px-12 py-12">
        {activeSection === 'workspace' ? (
          <WorkspaceSettingsView />
        ) : (
          <DebugSettingsView onReturnToWorkspace={() => onChangeSection('workspace')} />
        )}
      </main>
    </div>
  );
}
