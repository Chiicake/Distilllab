import type { RunCardMeta } from '../../chat/types';

type ActiveInspectorProps = {
  decisionSummary: string | null;
  lastToolStatus: string | null;
  lastRunStatus: string | null;
  streamStatuses: string[];
  liveAssistantText: string;
  currentRun?: RunCardMeta | null;
};

function formatRunTypeLabel(runType: string | null | undefined) {
  if (!runType) {
    return 'Unknown Run';
  }

  return runType
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export default function ActiveInspector({
  decisionSummary,
  lastToolStatus,
  lastRunStatus,
  streamStatuses,
  liveAssistantText,
  currentRun,
}: ActiveInspectorProps) {
  const recentStatuses = streamStatuses.slice(-6).reverse();
  const hasActiveRun = Boolean(
    currentRun
    && (currentRun.state === 'queued' || currentRun.state === 'running' || currentRun.state === 'pending'),
  );

  return (
    <aside className="hidden lg:flex flex-col w-80 h-full border-l border-outline-variant/10 bg-surface-bright/60 backdrop-blur-[20px] z-20">
      <div className="p-6 flex flex-col h-full">
        <div className="flex items-center justify-between mb-8">
          <h3 className="font-headline font-bold text-on-surface text-sm uppercase tracking-wider">Inspector</h3>
          <span className="material-symbols-outlined text-on-surface-variant" data-icon="close">
            close
          </span>
        </div>

        <div className="space-y-8 overflow-y-auto no-scrollbar">
          <section>
            <label className="text-[10px] font-bold text-primary uppercase tracking-[0.2em] block mb-4">
              {hasActiveRun ? 'Run Context' : 'Session Context'}
            </label>

            <div className="space-y-4">
              <div className="flex flex-col gap-1">
                <span className="text-xs text-on-surface-variant font-medium">Goal</span>
                <span className="text-sm text-on-surface leading-tight">
                  {hasActiveRun && currentRun
                    ? `${formatRunTypeLabel(currentRun.runType)} is active for this conversation.`
                    : 'Track the conversation, summarize key intent changes, and wait for the next run-worthy action.'}
                </span>
              </div>

              {hasActiveRun && currentRun ? (
                <div className="grid grid-cols-2 gap-3">
                  <div className="rounded-xl border border-outline-variant/20 bg-surface-container-highest p-3">
                    <span className="text-[10px] uppercase tracking-[0.14em] text-on-surface-variant">Run Type</span>
                    <p className="mt-2 text-sm font-semibold text-on-surface">{formatRunTypeLabel(currentRun.runType)}</p>
                  </div>
                  <div className="rounded-xl border border-outline-variant/20 bg-surface-container-highest p-3">
                    <span className="text-[10px] uppercase tracking-[0.14em] text-on-surface-variant">Progress</span>
                    <p className="mt-2 text-sm font-semibold text-on-surface">{currentRun.progressPercent}%</p>
                  </div>
                </div>
              ) : null}

              {!hasActiveRun ? (
                <div className="rounded-xl border border-outline-variant/20 bg-surface-container-highest p-3">
                  <span className="text-[10px] uppercase tracking-[0.14em] text-on-surface-variant">Session State</span>
                  <p className="mt-2 text-sm font-semibold text-on-surface">
                    {decisionSummary ?? 'Waiting for the next user instruction.'}
                  </p>
                </div>
              ) : null}

              <div className="flex flex-col gap-1">
                <span className="text-xs text-on-surface-variant font-medium">Decision</span>
                <span className="text-sm text-on-surface">{decisionSummary ?? 'Waiting for next decision...'}</span>
              </div>

              <div className="flex flex-col gap-1">
                <span className="text-xs text-on-surface-variant font-medium">Tool Status</span>
                <span className="text-sm text-primary">{lastToolStatus ?? 'No tool activity yet.'}</span>
              </div>

              <div className="flex flex-col gap-1">
                <span className="text-xs text-on-surface-variant font-medium">Run Status</span>
                <span className="text-sm text-primary">{lastRunStatus ?? 'No active run.'}</span>
              </div>

              {hasActiveRun && currentRun ? (
                <div className="flex flex-col gap-1">
                  <span className="text-xs text-on-surface-variant font-medium">Current Step</span>
                  <span className="text-sm text-on-surface">
                    {currentRun.stepSummary ?? currentRun.stepKey ?? 'Waiting for first step...'}
                  </span>
                  {currentRun.detailText ? (
                    <span className="text-xs leading-relaxed text-on-surface-variant">{currentRun.detailText}</span>
                  ) : null}
                </div>
              ) : null}

              {!hasActiveRun ? (
                <div className="flex flex-col gap-1">
                  <span className="text-xs text-on-surface-variant font-medium">Next Trigger</span>
                  <span className="text-sm text-on-surface-variant">
                    A new run will appear here only while work is actively executing.
                  </span>
                </div>
              ) : null}
            </div>
          </section>

          <section>
            <label className="mb-4 block text-[10px] font-bold uppercase tracking-[0.2em] text-secondary">
              Stream Events
            </label>

            <div className="space-y-2 rounded-xl border border-outline-variant/20 bg-surface-container-highest p-3">
              {recentStatuses.length === 0 ? (
                <p className="text-xs text-on-surface-variant">No stream events yet.</p>
              ) : (
                recentStatuses.map((status, index) => (
                  <p key={`${status}-${index}`} className="text-xs leading-relaxed text-on-surface-variant">
                    {status}
                  </p>
                ))
              )}
            </div>
          </section>

          <section>
            <label className="mb-4 block text-[10px] font-bold uppercase tracking-[0.2em] text-primary">
              Live Assistant Output
            </label>

            <div className="rounded-xl border border-primary/20 bg-primary/5 p-3">
              <p className="text-xs leading-relaxed text-on-surface-variant whitespace-pre-wrap">
                {liveAssistantText || 'Waiting for streaming chunks...'}
              </p>
            </div>
          </section>

          <section>
            <label className="text-[10px] font-bold text-secondary uppercase tracking-[0.2em] block mb-4">Related Objects</label>

            <div className="flex flex-wrap gap-2 mb-3">
              <div className="px-3 py-1.5 bg-surface-container-highest rounded-xl border border-outline-variant/20 flex items-center gap-2 transition-colors">
                <span className="material-symbols-outlined text-[16px] text-secondary" data-icon="workspaces">
                  workspaces
                </span>
                <span className="text-[11px] font-medium">{hasActiveRun ? 'Active Run Context' : 'Session Context'}</span>
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              {hasActiveRun && currentRun ? (
                <div className="px-3 py-1.5 bg-surface-container-highest rounded-xl border border-outline-variant/20 flex items-center gap-2 transition-colors">
                  <span className="material-symbols-outlined text-[16px] text-on-surface-variant" data-icon="play_circle">
                    play_circle
                  </span>
                  <span className="text-[11px] font-medium">{formatRunTypeLabel(currentRun.runType)}</span>
                </div>
              ) : (
                <div className="px-3 py-1.5 bg-surface-container-highest rounded-xl border border-outline-variant/20 flex items-center gap-2 transition-colors">
                  <span className="material-symbols-outlined text-[16px] text-on-surface-variant" data-icon="forum">
                    forum
                  </span>
                  <span className="text-[11px] font-medium">Conversation Timeline</span>
                </div>
              )}
            </div>
          </section>

          <section className="mt-auto pt-8">
            <div className="bg-primary/5 rounded-xl p-4 border border-primary/10">
              <div className="flex items-center gap-3 mb-3">
                <div className="w-8 h-8 rounded-full gradient-primary flex items-center justify-center">
                  <span className="material-symbols-outlined text-on-primary text-sm" data-icon="bolt">
                    bolt
                  </span>
                </div>

                <div>
                  <h4 className="text-xs font-bold text-on-surface uppercase">Agent / Analyst</h4>
                  <p className="text-[10px] text-primary">Structural reasoning mode</p>
                </div>
              </div>

              <p className="text-[11px] text-on-surface-variant leading-relaxed">
                {hasActiveRun
                  ? 'Focused on tracking the active run, preserving execution context, and surfacing the current step clearly.'
                  : 'Focused on preserving conversation context and making the next actionable transition into a run clear.'}
              </p>
            </div>
          </section>
        </div>
      </div>
    </aside>
  );
}
