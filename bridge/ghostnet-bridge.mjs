#!/usr/bin/env node
// GhostNet CLI <-> SDK bridge.
//
// The Rust CLI spawns this script with a sub-command and arguments, then reads
// newline-delimited JSON ("NDJSON") from stdout. Every line is a JSON object
// with at least an `ok: boolean` field.
//
// Commands:
//   identity-create
//   identity-load <seedPhrase>
//   send <peerId> <message> [--seed <phrase>] [--endpoint <wss-url>]
//   listen [--seed <phrase>] [--endpoint <wss-url>]

import { GhostNet } from '@n11x/ghostnet-sdk';

function emit(obj) {
  process.stdout.write(JSON.stringify(obj) + '\n');
}

function fail(message, code = 1) {
  const seed = process.env.GHOSTNET_SEED || flags.seed || (cmd === 'identity-load' ? positional[0] : undefined);
  const text = seed ? String(message).split(seed).join('[redacted]') : String(message);
  emit({ ok: false, error: text });
  process.exit(code);
}

const [, , cmd, ...rest] = process.argv;

// Parse `--flag value` pairs out of the remaining args; everything else is
// treated as a positional argument.
const flags = {};
const positional = [];
// Fixed positional arity prevents message text such as "--endpoint" from
// being interpreted as an option. Rust passes options after positionals.
const arity = cmd === 'send' ? 2 : 0;
for (let i = 0; i < rest.length; i++) {
  const arg = rest[i];
  if (positional.length < arity) {
    positional.push(arg);
  } else if (arg === '--endpoint' || arg === '--seed') {
    if (rest[i + 1] == null || rest[i + 1].startsWith('--')) fail(`missing value for ${arg}`);
    flags[arg.slice(2)] = rest[++i];
  } else if (cmd === 'identity-load' && positional.length === 0 && !arg.startsWith('--')) {
    positional.push(arg);
  } else {
    fail('unexpected bridge argument');
  }
}

const seconds = Number(process.env.GHOSTNET_TIMEOUT_SECS || 60);
if (!Number.isInteger(seconds) || seconds < 1 || seconds > 86400) fail('GHOSTNET_TIMEOUT_SECS must be between 1 and 86400');
let deadline = setTimeout(() => fail('SDK operation timed out'), seconds * 1000);
function finishDeadline() { clearTimeout(deadline); }

// Seed phrase is read from the environment (set by the Rust CLI) so it never
// appears in this process's argv. Fall back to flags/positional for ad-hoc use.
function seedPhrase(positionalIndex) {
  return (
    process.env.GHOSTNET_SEED ||
    flags.seed ||
    (positionalIndex != null ? positional[positionalIndex] : undefined)
  );
}

function newClient() {
  // Passing `endpoint: undefined` falls back to the SDK default.
  return new GhostNet(flags.endpoint ? { endpoint: flags.endpoint } : {});
}

async function main() {
  switch (cmd) {
    case 'identity-create': {
      const gn = newClient();
      const id = gn.createIdentity();
      emit({ ok: true, nodeId: id.nodeId, seedPhrase: id.seedPhrase });
      break;
    }

    case 'identity-load': {
      const seed = seedPhrase(0);
      if (!seed) fail('a 12-word seed phrase is required');
      const gn = newClient();
      const id = gn.loadIdentity(seed);
      emit({ ok: true, nodeId: id.nodeId });
      break;
    }

    case 'send': {
      const peer = positional[0];
      const message = positional[1];
      if (!peer?.trim() || !message) fail('usage: send <peerId> <message> [--seed <phrase>]');
      if (Buffer.byteLength(message, 'utf8') > 64 * 1024) fail('message exceeds the 64 KiB UTF-8 limit');

      const gn = newClient();
      const seed = seedPhrase();
      if (!seed?.trim()) fail('GHOSTNET_SEED is required');
      const id = gn.loadIdentity(seed);
      try {
        await gn.connect();
        await gn.send(peer, message);
      } finally {
        gn.disconnect();
      }
      emit({ ok: true, sent: true, to: peer, from: id.nodeId });
      break;
    }

    case 'listen': {
      const gn = newClient();
      const seed = seedPhrase();
      if (!seed?.trim()) fail('GHOSTNET_SEED is required');
      const id = gn.loadIdentity(seed);
      emit({ ok: true, event: 'identity', nodeId: id.nodeId });

      gn.on('connect', () => emit({ ok: true, event: 'connect' }));
      gn.on('disconnect', (reason) => emit({ ok: true, event: 'disconnect', reason: String(reason ?? '') }));
      gn.on('message', (m) =>
        emit({ ok: true, event: 'message', from: m.from, data: m.data, timestamp: m.timestamp }),
      );
      gn.on('error', (e) => fail(e?.message ?? e));
      const stop = () => { gn.disconnect(); process.exit(130); };
      process.once('SIGINT', stop);
      process.once('SIGTERM', stop);

      await gn.connect();
      finishDeadline();
      // The open WebSocket keeps the Node process alive; the CLI kills it on Ctrl-C.
      break;
    }

    default:
      fail(`unknown command: ${cmd ?? '(none)'}`);
  }
}

main().then(finishDeadline).catch((e) => fail(e?.message ?? e));
