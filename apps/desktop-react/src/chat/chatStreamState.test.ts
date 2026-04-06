import test from 'node:test';
import assert from 'node:assert/strict';

import { deriveCompletedActiveRunLabel, liveToolStatusLabel } from './chatStreamState.ts';
import type { ChatMessage } from './types.ts';

function runMessage(runId: string, state: 'queued' | 'pending' | 'running' | 'completed' | 'failed'): ChatMessage {
  return {
    id: `run-card-${runId}`,
    role: 'system',
    kind: 'run',
    content: 'run status',
    summary: 'run summary',
    details: 'run details',
    expandable: true,
    runMeta: {
      runId,
      state,
      progressPercent: state === 'completed' || state === 'failed' ? 100 : 50,
      runType: 'distill',
      steps: [],
    },
  };
}

test('deriveCompletedActiveRunLabel clears active label after sync when synchronized run card is terminal', () => {
  const messages = [runMessage('run-1', 'completed')];

  assert.equal(deriveCompletedActiveRunLabel(messages, 'run-1', 'run-1'), null);
});

test('deriveCompletedActiveRunLabel keeps active label after sync when synchronized run card is still active', () => {
  const messages = [runMessage('run-1', 'running')];

  assert.equal(deriveCompletedActiveRunLabel(messages, 'run-1', null), 'run-1');
});

test('liveToolStatusLabel uses bridge-compatible failed wording for failed tools', () => {
  assert.equal(liveToolStatusLabel('failed'), 'failed');
});
