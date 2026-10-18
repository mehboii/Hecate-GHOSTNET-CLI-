#!/usr/bin/env node
'use strict';

// Launcher: forwards all arguments to the downloaded native Hecate binary.

const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

const ext = process.platform === 'win32' ? '.exe' : '';
const bin = path.join(__dirname, 'bin', `hecate${ext}`);

if (!fs.existsSync(bin)) {
  console.error(
    'Hecate binary not found. Try reinstalling: npm install -g @n11x/hecate-cli',
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(`Failed to run Hecate: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
