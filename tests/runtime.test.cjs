'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const installer = require('../scripts/install.js');

test('installer only accepts HTTPS release hosts and computes exact digests', () => {
  for (const url of ['http://github.com/a', 'https://evil.example/a', 'https://github.com.evil.example/a']) {
    assert.throws(() => installer.assertAllowedUrl(url));
  }
  assert.equal(installer.assertAllowedUrl('https://release-assets.githubusercontent.com/a').protocol, 'https:');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ghostnet-hash-'));
  try {
    const file = path.join(dir, 'binary');
    fs.writeFileSync(file, 'known artifact');
    assert.equal(installer.sha256(file), createHash('sha256').update('known artifact').digest('hex'));
  } finally { fs.rmSync(dir, { recursive: true }); }
});

test('embedded bridge protocol preserves flag-like messages, requires seed and disconnects on send failure', () => {
  // This fixture validates the real bridge and parser, not SDK/network correctness.
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ghostnet-bridge-'));
  try {
    fs.copyFileSync(path.join(__dirname, '../bridge/ghostnet-bridge.mjs'), path.join(dir, 'bridge.mjs'));
    const sdk = path.join(dir, 'node_modules/@n11x/ghostnet-sdk');
    fs.mkdirSync(sdk, { recursive: true });
    fs.writeFileSync(path.join(sdk, 'package.json'), JSON.stringify({ type: 'module', exports: './index.js' }));
    fs.writeFileSync(path.join(sdk, 'index.js'), `
      import fs from 'node:fs';
      export class GhostNet {
        constructor(options) { fs.writeFileSync('options.json', JSON.stringify(options)); }
        createIdentity() { return {nodeId:'public-id',seedPhrase:'fixture seed'}; }
        loadIdentity(seed) { if(seed === 'bad seed') throw new Error('invalid: '+seed); return {nodeId:'public-id'}; }
        async connect() { if(process.env.TEST_HANG) await new Promise(()=>{}); }
        async send(peer, message) { fs.writeFileSync('message.txt', message); if(message === 'fail') throw new Error('send failed'); }
        disconnect() { fs.writeFileSync('disconnected', 'yes'); }
        on() {}
      }
    `);
    const run = (args, env = {}) => spawnSync(process.execPath, [path.join(dir, 'bridge.mjs'), ...args], {
      cwd: dir, encoding: 'utf8', timeout: 5000,
      env: { ...process.env, GHOSTNET_SEED: 'fixture seed', GHOSTNET_TIMEOUT_SECS: '2', ...env },
    });
    let result = run(['send', 'peer', '--endpoint', '--endpoint', 'wss://relay.example']);
    assert.equal(result.status, 0, result.stderr + result.stdout);
    assert.equal(fs.readFileSync(path.join(dir, 'message.txt'), 'utf8'), '--endpoint');
    assert.deepEqual(JSON.parse(fs.readFileSync(path.join(dir, 'options.json'))), { endpoint: 'wss://relay.example' });
    assert.equal(JSON.parse(result.stdout).sent, true);
    for (const args of [['send', 'peer', 'hello'], ['listen']]) {
      result = run(args, { GHOSTNET_SEED: '' });
      assert.equal(result.status, 1);
      assert.match(result.stdout, /GHOSTNET_SEED is required/);
    }
    result = run(['send', 'peer', 'fail']);
    assert.equal(result.status, 1);
    assert.equal(fs.readFileSync(path.join(dir, 'disconnected'), 'utf8'), 'yes');
    result = run(['identity-load'], { GHOSTNET_SEED: 'bad seed' });
    assert.equal(result.status, 1);
    assert.ok(!result.stdout.includes('bad seed'));
    assert.match(result.stdout, /redacted/);
    result = run(['listen'], { TEST_HANG: '1', GHOSTNET_TIMEOUT_SECS: '1' });
    assert.equal(result.status, 1);
    assert.match(result.stdout, /timed out/);
    result = run(['send', 'peer', 'hello', '--endpoint']);
    assert.equal(result.status, 1);
    assert.match(result.stdout, /missing value/);
    result = run(['identity-create']);
    assert.equal(result.status, 0);
    assert.equal(JSON.parse(result.stdout).nodeId, 'public-id');
  } finally { fs.rmSync(dir, { recursive: true }); }
});

test('package and native versions match and release manifest covers supported assets', () => {
  const pkg = require('../package.json');
  const cargo = fs.readFileSync(path.join(__dirname, '../Cargo.toml'), 'utf8');
  assert.ok(cargo.includes(`version = "${pkg.version}"`));
  const checksums = require('../checksums.json');
  assert.equal(Object.keys(checksums).length, 5);
  for (const hash of Object.values(checksums)) assert.match(hash, /^[a-f0-9]{64}$/i);
  assert.ok(!fs.readFileSync(path.join(__dirname, '../scripts/run.js'), 'utf8').includes('PLUSPANEL_API_KEY'));
});

test('installer streams safely, rejects response failures, redirects and existing temp files', async () => {
  const https = require('node:https');
  const { Readable } = require('node:stream');
  const { EventEmitter } = require('node:events');
  const original = https.get;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ghostnet-download-'));
  let mode = 'success';
  https.get = (url, options, callback) => {
    assert.ok(options.signal instanceof AbortSignal);
    const request = new EventEmitter();
    request.setTimeout = (ms, handler) => { assert.equal(ms, 30000); return request; };
    process.nextTick(() => {
      const response = new Readable({ read() {} });
      response.statusCode = mode === 'status' ? 404 : mode === 'redirect' ? 302 : 200;
      response.headers = mode === 'redirect' ? { location: 'https://evil.example/payload' } : {};
      callback(response);
      process.nextTick(() => {
        if (mode === 'aborted') response.destroy(new Error('response aborted'));
        else if (mode === 'oversized') { response.push(Buffer.alloc(64 * 1024 * 1024 + 1)); response.push(null); }
        else { response.push('verified artifact'); response.push(null); }
      });
    });
    return request;
  };
  try {
    const file = path.join(dir, 'artifact');
    await installer.download('https://github.com/asset', file);
    assert.equal(fs.readFileSync(file, 'utf8'), 'verified artifact');
    await assert.rejects(installer.download('https://github.com/asset', file), /EEXIST/);
    for (const failure of ['status', 'redirect', 'aborted', 'oversized']) {
      mode = failure;
      await assert.rejects(installer.download('https://github.com/asset', path.join(dir, failure)));
    }
  } finally {
    https.get = original;
    fs.rmSync(dir, { recursive: true });
  }
});

test('real installer process installs matching artifact and preserves it on checksum failure', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ghostnet-install-'));
  try {
    fs.mkdirSync(path.join(dir, 'scripts'));
    fs.copyFileSync(path.join(__dirname, '../scripts/install.js'), path.join(dir, 'scripts/install.js'));
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify({ version: '0.0.0' }));
    const hash = createHash('sha256').update('fixture artifact').digest('hex');
    fs.writeFileSync(path.join(dir, 'checksums.json'), JSON.stringify({ [installer.resolveAsset()]: hash }));
    fs.writeFileSync(path.join(dir, 'transport.cjs'), `
      const {Readable}=require('node:stream');
      const {EventEmitter}=require('node:events');
      require('node:https').get=(url,options,callback)=>{
        const request=new EventEmitter();request.setTimeout=()=>request;
        setImmediate(()=>{
          const response=Readable.from([Buffer.from(process.env.TEST_TAMPER?'tampered':'fixture artifact')]);
          response.statusCode=200;response.headers={};callback(response);
        });return request;
      };
    `);
    const run = (tamper) => spawnSync(process.execPath, ['--require', path.join(dir, 'transport.cjs'), path.join(dir, 'scripts/install.js')], {
      cwd: dir, encoding: 'utf8', timeout: 5000, env: { ...process.env, TEST_TAMPER: tamper ? '1' : '' },
    });
    let result = run(false);
    assert.equal(result.status, 0, result.stderr);
    const bin = path.join(dir, 'scripts/bin', process.platform === 'win32' ? 'hecate.exe' : 'hecate');
    assert.equal(fs.readFileSync(bin, 'utf8'), 'fixture artifact');
    result = run(true);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /checksum mismatch/);
    assert.equal(fs.readFileSync(bin, 'utf8'), 'fixture artifact');
    assert.equal(fs.readdirSync(path.dirname(bin)).length, 1);
  } finally { fs.rmSync(dir, { recursive: true }); }
});
