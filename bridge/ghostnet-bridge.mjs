#!/usr/bin/env node
// Hecate <-> GhostNet SDK bridge.
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
  emit({ ok: false, error: String(message) });
  process.exit(code);
}

const [, , cmd, ...rest] = process.argv;

// Parse `--flag value` pairs out of the remaining args; everything else is
// treated as a positional argument.
const flags = {};
const positional = [];
for (let i = 0; i < rest.length; i++) {
  const arg = rest[i];
  if (arg.startsWith('--')) {
    flags[arg.slice(2)] = rest[++i];
  } else {
    positional.push(arg);
  }
}

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
      if (!peer || !message) fail('usage: send <peerId> <message> [--seed <phrase>]');

      const gn = newClient();
      const seed = seedPhrase();
      const id = seed ? gn.loadIdentity(seed) : gn.createIdentity();
      await gn.connect();
      await gn.send(peer, message);
      gn.disconnect();
      emit({ ok: true, sent: true, to: peer, from: id.nodeId });
      break;
    }

    case 'listen': {
      const gn = newClient();
      const seed = seedPhrase();
      const id = seed ? gn.loadIdentity(seed) : gn.createIdentity();
      emit({ ok: true, event: 'identity', nodeId: id.nodeId });

      gn.on('connect', () => emit({ ok: true, event: 'connect' }));
      gn.on('disconnect', (reason) => emit({ ok: true, event: 'disconnect', reason: String(reason ?? '') }));
      gn.on('message', (m) =>
        emit({ ok: true, event: 'message', from: m.from, data: m.data, timestamp: m.timestamp }),
      );
      gn.on('error', (e) => emit({ ok: false, event: 'error', error: String(e?.message ?? e) }));

      await gn.connect();
      // The open WebSocket keeps the Node process alive; the CLI kills it on Ctrl-C.
      break;
    }

    default:
      fail(`unknown command: ${cmd ?? '(none)'}`);
  }
}

main().catch((e) => fail(e?.message ?? e));
