#!/usr/bin/env node
// Generate checksums.json by hashing the release binaries for a given tag.
//
// Usage: node scripts/gen-checksums.mjs <tag>   (defaults to v<package version>)
//
// Writes ../checksums.json mapping each asset name -> its SHA-256. install.js
// verifies downloaded binaries against this manifest, so it must be regenerated
// whenever the binaries change (i.e. every release) and committed before publish.

import { createHash } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO = 'mehboii/Hecate-GHOSTNET-CLI-';
const ASSETS = [
  'ghostnet-win32-x64.exe',
  'ghostnet-linux-x64',
  'ghostnet-linux-arm64',
  'ghostnet-darwin-x64',
  'ghostnet-darwin-arm64',
];

const pkg = (await import('../package.json', { with: { type: 'json' } })).default;
const tag = process.argv[2] || `v${pkg.version}`;

async function hashAsset(asset) {
  const url = `https://github.com/${REPO}/releases/download/${tag}/${asset}`;
  const res = await fetch(url, { redirect: 'follow' });
  if (!res.ok) throw new Error(`HTTP ${res.status} for ${url}`);
  const buf = Buffer.from(await res.arrayBuffer());
  return createHash('sha256').update(buf).digest('hex');
}

const checksums = {};
for (const asset of ASSETS) {
  process.stderr.write(`hashing ${asset}…\n`);
  checksums[asset] = await hashAsset(asset);
}

const out = join(__dirname, '..', 'checksums.json');
writeFileSync(out, JSON.stringify(checksums, null, 2) + '\n');
process.stderr.write(`wrote ${out} for ${tag}\n`);
console.log(JSON.stringify(checksums, null, 2));
