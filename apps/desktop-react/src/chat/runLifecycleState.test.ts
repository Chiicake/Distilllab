import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveLifecycleRunState } from './runLifecycleState.ts';
import type { RunCardMeta } from './types.ts';

function runMeta(overrides: Partial<RunCardMeta> = {}): RunCardMeta {
  return {
    runId: 'run-1',
    state: 'running',
    progressPercent: 60,
    runType: 'distill',
    stepKey: 'draft',
    stepSummary: 'Draft answer',
    stepStatus: 'running',
    stepIndex: 2,
    stepsTotal: 3,
    detailText: 'Drafting',
    currentStepKey: 'draft',
    steps: [{
      key: 'draft',
      summary: 'Draft answer',
      status: 'running',
      index: 2,
      total: 3,
      detailText: 'Drafting',
    }],
    ...overrides,
  };
}

test('resolveLifecycleRunState advances detailed running state to completed on terminal lifecycle event', () => {
  assert.equal(resolveLifecycleRunState(runMeta(), 'completed'), 'completed');
});

test('resolveLifecycleRunState returns lifecycle state when no previous run meta exists', () => {
  assert.equal(resolveLifecycleRunState(undefined, 'running'), 'running');
});

test('resolveLifecycleRunState uses lifecycle state when previous run meta lacks detailed progress', () => {
  assert.equal(resolveLifecycleRunState(runMeta({
    state: 'queued',
    stepKey: null,
    stepSummary: null,
    stepStatus: null,
    detailText: null,
    currentStepKey: null,
    steps: [],
  }), 'running'), 'running');
});

test('resolveLifecycleRunState preserves failed detailed state against completed lifecycle event', () => {
  assert.equal(resolveLifecycleRunState(runMeta({ state: 'failed' }), 'completed'), 'failed');
});

test('resolveLifecycleRunState allows terminal failed lifecycle to override non-failed detailed state', () => {
  assert.equal(resolveLifecycleRunState(runMeta({ state: 'running' }), 'failed'), 'failed');
});

test('resolveLifecycleRunState keeps detailed running state for non-terminal lifecycle updates', () => {
  assert.equal(resolveLifecycleRunState(runMeta(), 'running'), 'running');
});
