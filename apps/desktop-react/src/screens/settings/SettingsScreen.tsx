import type { SettingsSection } from '../../app-state/screen-state';

type SettingsScreenProps = {
  section?: SettingsSection;
  onChangeSection: (section: SettingsSection) => void;
};

const sectionCopy: Record<SettingsSection, { title: string; body: string }> = {
  workspace: {
    title: 'Workspace Settings',
    body: 'Workspace settings placeholder. Future preferences for project defaults and desktop behavior will live here.',
  },
  debug: {
    title: 'Debug Settings',
    body: 'Debug settings placeholder. Future diagnostics, logging, and developer-only controls will live here.',
  },
};

export default function SettingsScreen({ section = 'workspace', onChangeSection }: SettingsScreenProps) {
  const activeSectionKey = section;
  const activeSection = sectionCopy[section];

  return (
    <div className="flex min-w-0 flex-1 items-center justify-center bg-surface px-6 py-10">
      <div className="w-full max-w-2xl space-y-6 rounded-3xl border border-outline-variant/20 bg-surface-container/40 p-8 text-on-surface shadow-sm">
        <div className="space-y-2 text-center">
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-on-surface-variant">Settings Placeholder</p>
          <h1 className="font-headline text-3xl font-extrabold">{activeSection.title}</h1>
          <p className="text-sm leading-relaxed text-on-surface-variant">{activeSection.body}</p>
        </div>

        <div className="flex justify-center gap-3">
          {(['workspace', 'debug'] as const).map((nextSection) => {
            const isActive = nextSection === activeSectionKey;

            return (
              <button
                key={nextSection}
                className={`rounded-full border px-4 py-2 text-sm font-medium transition-colors ${
                  isActive
                    ? 'border-on-surface bg-on-surface text-surface'
                    : 'border-outline-variant/30 text-on-surface-variant hover:border-on-surface/40 hover:text-on-surface'
                }`}
                onClick={() => onChangeSection(nextSection)}
                type="button"
              >
                {nextSection === 'workspace' ? 'Workspace' : 'Debug'}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
