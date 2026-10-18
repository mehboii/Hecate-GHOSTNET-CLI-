#!/usr/bin/env node
'use strict';

// Launcher: forwards all arguments to the downloaded native GhostNet binary.

const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

const ext = process.platform === 'win32' ? '.exe' : '';
const bin = path.join(__dirname, 'bin', `ghostnet${ext}`);

if (!fs.existsSync(bin)) {
  console.error(
    'GhostNet CLI binary not found. Try reinstalling: npm install -g @n11x/ghostnet-cli',
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(`Failed to run GhostNet CLI: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
