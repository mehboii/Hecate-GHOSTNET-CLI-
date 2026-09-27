#!/usr/bin/env node
'use strict';

// Postinstall: download the prebuilt Hecate binary for this platform from
// the matching GitHub Release, verify its SHA-256 against the checksum manifest
// shipped inside this npm package, then install it to scripts/bin/.
//
// Security model: the npm tarball is immutable per version and is the trust
// anchor. checksums.json (published in the tarball) pins the exact hash of each
// release binary, so a tampered/compromised GitHub Release or a man-in-the-middle
// on the download is detected and rejected before the binary is ever executed.

const fs = require('fs');
const path = require('path');
const https = require('https');
const crypto = require('crypto');
const { URL } = require('url');
const { version } = require('../package.json');
const checksums = require('../checksums.json');

const REPO = 'mehboii/Hecate-GHOSTNET-CLI-';
const MAX_REDIRECTS = 5;
const MAX_BYTES = 64 * 1024 * 1024; // 64 MB ceiling — binaries are a few MB.

// Only these hosts may serve the binary (GitHub + its asset CDN).
const ALLOWED_HOSTS = new Set([
  'github.com',
  'objects.githubusercontent.com',
  'release-assets.githubusercontent.com',
  'codeload.github.com',
]);

// Map Node's platform-arch to the release asset name produced by CI.
const ASSETS = {
  'win32-x64': 'ghostnet-win32-x64.exe',
  'linux-x64': 'ghostnet-linux-x64',
  'linux-arm64': 'ghostnet-linux-arm64',
  'darwin-x64': 'ghostnet-darwin-x64',
  'darwin-arm64': 'ghostnet-darwin-arm64',
};

function resolveAsset() {
  const key = `${process.platform}-${process.arch}`;
  const asset = ASSETS[key];
  if (!asset) {
    throw new Error(
      `unsupported platform "${key}". Build from source instead: https://github.com/${REPO}`,
    );
  }
  return asset;
}

function assertAllowedUrl(rawUrl) {
  const u = new URL(rawUrl);
  if (u.protocol !== 'https:') {
    throw new Error(`refusing non-HTTPS URL: ${u.protocol}//${u.host}`);
  }
  if (!ALLOWED_HOSTS.has(u.hostname)) {
    throw new Error(`refusing download from untrusted host: ${u.hostname}`);
  }
  return u;
}

// Download to a temp file, following a bounded number of redirects, enforcing
// HTTPS + host allowlist and a size ceiling. Resolves with the temp path.
function download(rawUrl, tmpPath, redirectsLeft = MAX_REDIRECTS) {
  return new Promise((resolve, reject) => {
    let u;
    try {
      u = assertAllowedUrl(rawUrl);
    } catch (err) {
      return reject(err);
    }

    https
      .get(u, { headers: { 'User-Agent': 'hecate-cli-installer' } }, (res) => {
        const { statusCode, headers } = res;

        if (statusCode >= 300 && statusCode < 400 && headers.location) {
          res.resume();
          if (redirectsLeft <= 0) {
            return reject(new Error('too many redirects while downloading'));
          }
          // Resolve relative redirects against the current URL.
          const next = new URL(headers.location, u).toString();
          return resolve(download(next, tmpPath, redirectsLeft - 1));
        }

        if (statusCode !== 200) {
          res.resume();
          return reject(new Error(`download failed (HTTP ${statusCode}) for ${u.href}`));
        }

        const file = fs.createWriteStream(tmpPath, { mode: 0o755 });
        let bytes = 0;
        res.on('data', (chunk) => {
          bytes += chunk.length;
          if (bytes > MAX_BYTES) {
            res.destroy();
            file.destroy();
            reject(new Error('download exceeded maximum allowed size'));
          }
        });
        res.pipe(file);
        file.on('finish', () => file.close(() => resolve(tmpPath)));
        file.on('error', reject);
      })
      .on('error', reject);
  });
}

function sha256(filePath) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(filePath));
  return hash.digest('hex');
}

async function main() {
  const asset = resolveAsset();

  const expected = checksums[asset];
  if (!expected || !/^[a-f0-9]{64}$/i.test(expected)) {
    throw new Error(`no valid checksum pinned for "${asset}" — refusing to install`);
  }

  const binDir = path.join(__dirname, 'bin');
  fs.mkdirSync(binDir, { recursive: true });

  const ext = process.platform === 'win32' ? '.exe' : '';
  const dest = path.join(binDir, `hecate${ext}`);
  const tmp = path.join(binDir, `.hecate-download-${process.pid}${ext}`);
  const url = `https://github.com/${REPO}/releases/download/v${version}/${asset}`;

  process.stdout.write(`Downloading Hecate v${version} (${asset})…\n`);

  try {
    await download(url, tmp);

    const actual = sha256(tmp);
    if (actual.toLowerCase() !== expected.toLowerCase()) {
      throw new Error(
        `checksum mismatch for ${asset}\n  expected ${expected}\n  got      ${actual}\n` +
          'The downloaded binary does not match the pinned hash and will NOT be installed.',
      );
    }

    // Atomic-ish install: only move the verified file into place.
    fs.renameSync(tmp, dest);
    if (process.platform !== 'win32') {
      fs.chmodSync(dest, 0o755);
    }
    process.stdout.write('Hecate installed (checksum verified). Run `hecate --help`.\n');
  } finally {
    // Never leave a partial/unverified download lying around.
    try {
      if (fs.existsSync(tmp)) fs.unlinkSync(tmp);
    } catch {
      /* best effort */
    }
  }
}

main().catch((err) => {
  console.error(`Hecate install failed: ${err.message}`);
  process.exit(1);
});
