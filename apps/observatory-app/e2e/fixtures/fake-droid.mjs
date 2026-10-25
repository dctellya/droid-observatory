#!/usr/bin/env node
// A stand-in for `droid exec --input-format stream-jsonrpc --output-format stream-jsonrpc`: it
// answers the host's requests, then asks one question and two permissions, spaced by
// FAKE_DROID_GAP_MS. Every line the host writes is appended to FAKE_DROID_LOG, so a spec can
// read what the host answered. The envelopes are the ones core/tests/rpc.rs records from a real
// droid 0.247.0.
import { appendFileSync } from 'node:fs';
import { createInterface } from 'node:readline';

const GAP_MS = Number(process.env.FAKE_DROID_GAP_MS ?? '400');
const LOG = process.env.FAKE_DROID_LOG;
// A creation time long before the run: the board must stamp arrival, not this.
const OLD_CREATED_AT = Date.UTC(2026, 0, 1);
const ENVELOPE = { jsonrpc: '2.0', factoryApiVersion: '1.0.0', factoryProtocolVersion: '1.274.0' };

let session = '';
let seq = 0;
let scripted = false;

const send = (value) => process.stdout.write(`${JSON.stringify({ ...ENVELOPE, ...value })}\n`);
const respond = (id, result) => send({ type: 'response', id, result });
const request = (id, method, params) => send({ type: 'request', id, method, params });
const notify = (notification) =>
  send({
    type: 'notification',
    method: 'droid.session_notification',
    params: { sessionId: session, notification },
  });
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function assistantToolUse(toolId, name, input) {
  seq += 1;
  notify({
    type: 'create_message',
    message: {
      id: `msg-${seq}`,
      parentId: 'root',
      seq,
      role: 'assistant',
      content: [{ type: 'tool_use', id: toolId, name, input }],
      createdAt: OLD_CREATED_AT,
      updatedAt: OLD_CREATED_AT,
    },
  });
}

async function script() {
  assistantToolUse('toolu_ASK', 'AskUser', { questions: [{ question: 'Which one?' }] });
  request('req-ask', 'droid.ask_user', {
    toolCallId: 'toolu_ASK',
    questions: [{ index: 1, topic: 'e2e', question: 'Which one?', options: [] }],
  });
  for (const id of ['perm-1', 'perm-2']) {
    await sleep(GAP_MS);
    const tool = `toolu_${id}`;
    assistantToolUse(tool, 'Execute', { command: `echo ${id}` });
    request(id, 'droid.request_permission', {
      toolUses: [{ toolUse: { id: tool, name: 'Execute' } }],
      options: [{ value: 'proceed_once' }, { value: 'cancel' }],
    });
  }
}

notify({ type: 'mcp_status_changed' });

createInterface({ input: process.stdin }).on('line', (text) => {
  if (LOG) appendFileSync(LOG, `${text}\n`);
  let line;
  try {
    line = JSON.parse(text);
  } catch {
    return;
  }
  if (line.method === 'droid.initialize_session') {
    session = line.params?.sessionId ?? 'fake-session';
    respond(line.id, { sessionId: session, settings: { modelId: 'fake' } });
  } else if (line.method) {
    respond(line.id, {});
    if (line.method === 'droid.add_user_message' && !scripted) {
      scripted = true;
      void script();
    }
  } else if (line.id === 'req-ask' && line.result) {
    const answer = line.result.answers?.[0]?.answer ?? '';
    notify({
      type: 'tool_result',
      toolUseId: 'toolu_ASK',
      isError: false,
      content: `1. [question] Which one?\n[answer] ${answer}`,
    });
  }
});

process.stdin.on('end', () => process.exit(0));
