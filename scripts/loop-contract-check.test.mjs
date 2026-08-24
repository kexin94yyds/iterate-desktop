import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');
const checkScript = path.join(repoRoot, 'scripts/loop-contract-check.mjs');

function runCheck(args = []) {
  return spawnSync(process.execPath, [checkScript, '--repo-root', repoRoot, ...args], {
    cwd: repoRoot,
    encoding: 'utf8',
  });
}

function readDoc(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

test('loop contract check validates the P0 contract documents', () => {
  const result = runCheck(['--json']);

  assert.equal(result.status, 0, result.stderr || result.stdout);

  const payload = JSON.parse(result.stdout);
  assert.deepEqual(payload.checked, [
    'docs/loop-contract-v0.md',
    'docs/verification-receipt-v0.md',
    'docs/worker-handoff-schema-v0.md',
  ]);
  assert.equal(payload.status, 'ok');
});

test('LoopContract v0 defines the runtime envelope and control gates', () => {
  const doc = readDoc('docs/loop-contract-v0.md');

  for (const required of [
    'loop_id',
    'run_id',
    'timeline_route_id',
    'goal',
    'context_policy',
    'tool_policy',
    'verification',
    'handoff',
    'stop_conditions',
  ]) {
    assert.match(doc, new RegExp(`\\b${required}\\b`), `${required} missing`);
  }
});

test('VerificationReceipt v0 defines evidence required before completion claims', () => {
  const doc = readDoc('docs/verification-receipt-v0.md');

  for (const required of [
    'Goal',
    'Scope',
    'Changes',
    'Commands',
    'Evidence',
    'Risks',
    'Next Gate',
  ]) {
    assert.match(doc, new RegExp(required), `${required} missing`);
  }
});

test('WorkerHandoff schema separates pairoom workers from built-in subagents', () => {
  const doc = readDoc('docs/worker-handoff-schema-v0.md');

  for (const required of [
    'Owner',
    'Task',
    'Inputs Reviewed',
    'Output',
    'Evidence',
    'Assumptions',
    'Risks',
    'Verification',
    'Open Questions',
    'Next Handoff',
    'pairoom',
    'built_in_subagent',
  ]) {
    assert.match(doc, new RegExp(required), `${required} missing`);
  }
});
