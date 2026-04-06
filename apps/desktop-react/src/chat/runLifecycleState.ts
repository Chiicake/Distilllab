import type { RunCardMeta, RunState } from './types';

export function hasDetailedRunProgress(runMeta: RunCardMeta | undefined): boolean {
  return Boolean(
    runMeta
    && (
      (runMeta.steps?.length ?? 0) > 0
      || runMeta.currentStepKey
      || runMeta.stepStatus
      || runMeta.stepSummary
      || runMeta.detailText
    ),
  );
}

export function resolveLifecycleRunState(
  previousRunMeta: RunCardMeta | undefined,
  lifecycleState: RunState,
): RunState {
  if (!previousRunMeta) {
    return lifecycleState;
  }

  if (previousRunMeta.state === 'failed') {
    return 'failed';
  }

  if (lifecycleState === 'completed' || lifecycleState === 'failed') {
    return lifecycleState;
  }

  if (hasDetailedRunProgress(previousRunMeta)) {
    return previousRunMeta.state;
  }

  return lifecycleState;
}
