#!/usr/bin/env node
'use strict';

// Launcher: forwards all arguments to the downloaded native Hecate binary.

const path = require('path');
const fs = require('fs');
const { spawnSync, spawn } = require('child_process');

const ext = process.platform === 'win32' ? '.exe' : '';
const bin = path.join(__dirname, 'bin', `hecate${ext}`);

// PlusPanel health beacon. Reports this CLI's health to the pluspanel-health API
// ("{base}/api/push", header x-api-key) once per invocation. Runs in a DETACHED,
// unref'd child so it never blocks the command, delays exit, or affects the CLI
// if it fails. URL/key are env-overridable; defaults wire it up out of the box.
function reportHealth() {
  try {
    const base = (process.env.PLUSPANEL_HEALTH_URL || 'http://localhost:3000')
      .trim()
      .replace(/\/+$/, '');
    if (!base) return;
    const key =
      process.env.PLUSPANEL_API_KEY || 'ppk_TwPPMZDa0Ma-MAddva3e7oglVEfALNrY';
    let version = '0.0.0';
    try {
      version = require('../package.json').version || version;
    } catch {}
    const body = JSON.stringify({
      name: process.env.PLUSPANEL_APP_NAME || 'hecate-cli',
      status: 'ok',
      version,
    });
    const child = spawn(
      process.execPath,
      [
        '-e',
        `const t=setTimeout(()=>process.exit(0),4000);` +
          `fetch(${JSON.stringify(`${base}/api/push`)},{method:'POST',` +
          `headers:{'content-type':'application/json','x-api-key':${JSON.stringify(key)}},` +
          `body:${JSON.stringify(body)}}).catch(()=>{}).finally(()=>{clearTimeout(t);process.exit(0);});`,
      ],
      { detached: true, stdio: 'ignore' },
    );
    child.unref();
  } catch {
    // Health reporting must never break the CLI.
  }
}

if (!fs.existsSync(bin)) {
  console.error(
    'Hecate binary not found. Try reinstalling: npm install -g @n11x/hecate-cli',
  );
  process.exit(1);
}

reportHealth();

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(`Failed to run Hecate: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
