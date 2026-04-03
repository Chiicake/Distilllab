import { useMemo, useState } from 'react';

import { useI18n } from '../../i18n/I18nProvider';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

function resolveTauriInvoke(): TauriInvoke | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const tauriInternals = (window as Window & {
    __TAURI_INTERNALS__?: { invoke?: TauriInvoke };
  }).__TAURI_INTERNALS__;

  return tauriInternals?.invoke ?? null;
}

function parseErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

type DebugAction = {
  labelKey: 'settings.debug.action.createRun' | 'settings.debug.action.createSession' | 'settings.debug.action.listSessions';
  command: 'create_demo_run' | 'create_demo_session' | 'list_sessions';
};

const debugActions: DebugAction[] = [
  { labelKey: 'settings.debug.action.createRun', command: 'create_demo_run' },
  { labelKey: 'settings.debug.action.createSession', command: 'create_demo_session' },
  { labelKey: 'settings.debug.action.listSessions', command: 'list_sessions' },
];

export default function DebugPanel() {
  const { t } = useI18n();
  const invoke = useMemo(resolveTauriInvoke, []);
  const [output, setOutput] = useState<string>(t('settings.debug.output.default'));
  const [isRunningCommand, setIsRunningCommand] = useState(false);

  const handleRunCommand = async (action: DebugAction) => {
    if (!invoke) {
      setOutput(t('settings.debug.output.bridgeMissing'));
      return;
    }

    setIsRunningCommand(true);
    setOutput(`${t('settings.debug.output.running')} ${action.command}...`);

    try {
      const result = await invoke<string>(action.command);
      setOutput(result);
    } catch (error) {
      setOutput(`${t('settings.debug.output.errorPrefix')}${parseErrorMessage(error)}`);
    } finally {
      setIsRunningCommand(false);
    }
  };

  return (
    <div className="w-full max-w-5xl space-y-8">
      <header className="space-y-2">
        <h2 className="font-headline text-4xl font-extrabold tracking-tighter text-on-surface">
          {t('settings.debug.title')}
        </h2>
        <p className="max-w-2xl text-sm leading-relaxed text-on-surface-variant">
          {t('settings.debug.description')}
        </p>
      </header>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="font-headline text-lg font-bold text-on-surface">
            {t('settings.debug.commands.title')}
          </h3>
          <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-on-surface-variant">
            {t('settings.debug.commands.badge')}
          </span>
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          {debugActions.map((action) => (
            <button
              key={action.command}
              className="rounded-sm border border-outline-variant/30 bg-surface-container-high px-4 py-3 text-left text-xs font-semibold uppercase tracking-[0.12em] text-on-surface transition-colors hover:border-primary/40 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50"
              disabled={isRunningCommand}
              onClick={() => {
                void handleRunCommand(action);
              }}
              type="button"
            >
              {t(action.labelKey)}
            </button>
          ))}
        </div>
      </section>

      <section className="rounded-sm border border-outline-variant/20 bg-surface-container p-6">
        <h3 className="mb-3 font-headline text-lg font-bold text-on-surface">
          {t('settings.debug.output.title')}
        </h3>
        <pre className="min-h-40 overflow-auto rounded-sm bg-surface-container-low p-4 font-mono text-xs text-on-surface-variant">
          {output}
        </pre>
      </section>
    </div>
  );
}
