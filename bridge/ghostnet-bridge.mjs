#!/usr/bin/env node
// Hecate <-> GhostNet SDK bridge.
//
// The Rust CLI spawns this script with a sub-command and arguments, then reads
// newline-delimited JSON ("NDJSON") from stdout. Every line is a JSON object
// with at least an `ok: boolean` field.
//
// Commands:
//   identity-create
//   identity-load
//   peer-from-key <publicKey>
//   send <peerId> <message> <publicKey> [--endpoint <wss-url>]
//   listen [--endpoint <wss-url>]

import { Atlas } from '@n11x/atlas';

function emit(obj) {
  process.stdout.write(JSON.stringify(obj) + '\n');
}

function fail(message, code = 1) {
  const seed = process.env.GHOSTNET_SEED;
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
const arity = cmd === 'send' ? 3 : cmd === 'peer-from-key' ? 1 : 0;
for (let i = 0; i < rest.length; i++) {
  const arg = rest[i];
  if (positional.length < arity) {
    positional.push(arg);
  } else if (arg === '--endpoint') {
    if (rest[i + 1] == null || rest[i + 1].startsWith('--')) fail(`missing value for ${arg}`);
    flags[arg.slice(2)] = rest[++i];
  } else {
    fail('unexpected bridge argument');
  }
}

const seconds = Number(process.env.GHOSTNET_TIMEOUT_SECS || 60);
if (!Number.isInteger(seconds) || seconds < 1 || seconds > 86400) fail('GHOSTNET_TIMEOUT_SECS must be between 1 and 86400');
let deadline = setTimeout(() => fail('SDK operation timed out'), seconds * 1000);
function finishDeadline() { clearTimeout(deadline); }

// Seed phrase is read only from the environment (set by the Rust CLI) so it
// never appears in this process's argv.
function seedPhrase() {
  return process.env.GHOSTNET_SEED;
}

function newClient() {
  // Passing `endpoint: undefined` falls back to the SDK default.
  return new Atlas(flags.endpoint ? { endpoint: flags.endpoint } : {});
}

async function main() {
  switch (cmd) {
    case 'identity-create': {
      const gn = newClient();
      const id = gn.createIdentity();
      emit({ ok: true, nodeId: id.nodeId, publicKey: id.publicKey, seedPhrase: id.seedPhrase });
      break;
    }

    case 'identity-load': {
      const seed = seedPhrase();
      if (!seed) fail('a 12-word seed phrase is required');
      const gn = newClient();
      const id = gn.loadIdentity(seed);
      emit({ ok: true, nodeId: id.nodeId, publicKey: id.publicKey });
      break;
    }

    case 'peer-from-key': {
      const publicKey = positional[0];
      if (!publicKey) fail('usage: peer-from-key <publicKey>');
      const peer = newClient().addPeer(publicKey);
      emit({ ok: true, nodeId: peer.nodeId, publicKey: peer.publicKey });
      break;
    }

    case 'send': {
      const peer = positional[0];
      const message = positional[1];
      const publicKey = positional[2];
      if (!peer?.trim() || !message || !publicKey) fail('usage: send <peerId> <message> <publicKey> [--endpoint <wss-url>]');
      if (Buffer.byteLength(message, 'utf8') > 64 * 1024) fail('message exceeds the 64 KiB UTF-8 limit');

      const gn = newClient();
      const seed = seedPhrase();
      if (!seed?.trim()) fail('GHOSTNET_SEED is required');
      const id = gn.loadIdentity(seed);
      const trustedPeer = gn.addPeer(publicKey);
      if (trustedPeer.nodeId !== peer) fail('trusted peer public key does not match the requested node ID');
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
      emit({ ok: true, event: 'identity', nodeId: id.nodeId, publicKey: id.publicKey });

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
