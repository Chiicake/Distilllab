import assert from 'node:assert/strict';
import test from 'node:test';

import { parseRequestedMaxAgentConcurrency } from './maxAgentConcurrencyInput.ts';

test('parseRequestedMaxAgentConcurrency accepts whole-number text', () => {
  assert.equal(parseRequestedMaxAgentConcurrency('16'), 16);
  assert.equal(parseRequestedMaxAgentConcurrency('-1'), -1);
});

test('parseRequestedMaxAgentConcurrency rejects decimal text instead of truncating it', () => {
  assert.equal(parseRequestedMaxAgentConcurrency('1.5'), null);
});

test('parseRequestedMaxAgentConcurrency rejects blank and non-numeric text', () => {
  assert.equal(parseRequestedMaxAgentConcurrency(''), null);
  assert.equal(parseRequestedMaxAgentConcurrency('abc'), null);
});
