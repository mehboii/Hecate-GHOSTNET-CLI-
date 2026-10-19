# Hecate

A Rust command-line messaging client with clap command parsing and a bundled Node bridge to `@n11x/ghostnet-sdk`. npm distributes the native executable through a small launcher and a SHA-256-verifying installer. See [CLI_AUDIT.md](CLI_AUDIT.md) for architecture, original findings, fixes and verification evidence.

**Current dependency limitation:** the public npm registry returned `E404` for `@n11x/ghostnet-sdk` during this audit. Local commands work without it; setup and SDK commands require access to an exact intended package version through the configured registry. No replacement SDK or cryptographic guarantees are assumed. Live relay messaging has not been verified.

## Install and build

```sh
npm install -g @n11x/hecate-cli
# Upgrade an existing installation:
npm install -g @n11x/hecate-cli@latest
```

The installer supports Windows x64, Linux x64/arm64 and macOS x64/arm64. Windows arm64 has no native release asset. Node.js 18+ must be on PATH at runtime. npm must be available for setup. Downloads use HTTPS, a GitHub host allowlist, bounded redirects, size and time limits, exclusive temporary files, and hashes pinned in the npm package. Downloads that fail verification are never installed.

From source, use a current stable Rust toolchain and Node.js 18+:

```sh
cargo build --locked --release
cargo install --locked --path .
```

Cargo.lock is checked in. The minimum Rust version has not been independently tested. Source builds include the bridge; npm release binaries include their own embedded bridge. Editing source does not update an already downloaded npm binary.

## Commands and examples

Run `hecate --help` or `hecate COMMAND --help` for clap-generated usage. `-h` is help; `-v` and `--version` report the native version. `--no-color` and `--debug` work on all commands. `NO_COLOR` disables color; redirected output is plain. Debug prints error context, without dumping the environment or raw bridge output.

| Command | Purpose / example |
| --- | --- |
| `hecate setup --sdk-version 1.2.3` | Install an exact SDK version with lifecycle scripts disabled and a five-minute deadline |
| `hecate info` | Native version, SDK package and local installation readiness |
| `hecate doctor` | Check Node/npm executables, bridge files/package metadata and configuration; exits 1 if a check fails |
| `hecate status` | Saved relay endpoint, public label count, and whether a seed environment value is present |
| `hecate config show [--json]` | Inspect non-secret saved configuration |
| `hecate config set-endpoint wss://relay.example/path` | Save a secure relay URL; substitute your actual relay |
| `hecate config clear-endpoint` | Return to SDK default endpoint |
| `hecate identity create [--name laptop]` | Generate identity with SDK and print seed for backup; optionally save public label |
| `hecate identity load [--name laptop]` | Restore using `GHOSTNET_SEED`; optionally save public label |
| `hecate identity list` | List saved labels and public node IDs |
| `hecate identity remove laptop` | Remove public label only; does not delete a network identity |
| `hecate send PEER "hello" [--endpoint wss://relay.example]` | SDK send using `GHOSTNET_SEED`; requires configured SDK and real peer |
| `hecate listen [--endpoint wss://relay.example]` | Stream incoming events using `GHOSTNET_SEED`; Ctrl-C stops the bridge |
| `hecate history [--limit 20] [--json]` | Newest-first local message metadata; JSON is an array; limit 0 returns no records |

Network examples use placeholders, not an advertised public relay or a validated node ID. The SDK validates the actual identity/peer format; the CLI rejects empty peers, whitespace/control characters, empty messages, and messages over 65,536 UTF-8 bytes. A send success means the SDK send operation completed; it does not establish receipt by the peer.

For message text beginning with a dash, end option parsing explicitly: `hecate send PEER -- "--endpoint"`. Put any actual options before `--`.

### Seeds and first use

Run setup with a trusted exact SDK version once the package is accessible, then create or restore an identity. Setup rejects tags, ranges, URLs and local package specifications, records the exact version in its generated lockfile, and disables npm lifecycle scripts. Back up the printed seed privately. It is never accepted in CLI arguments or saved in CLI configuration or history; provide it through `GHOSTNET_SEED` using a secret manager or masked prompt.

PowerShell example (masked input):

```powershell
$secret = Read-Host 'Seed phrase' -AsSecureString
$env:GHOSTNET_SEED = [System.Net.NetworkCredential]::new('', $secret).Password
hecate identity load --name laptop
hecate send REAL_PEER_NODE_ID 'hello from the mesh'
hecate listen
# When finished:
Remove-Item Env:GHOSTNET_SEED
```

Rust passes the seed to Node through the environment, never Node's argv. Environment variables can still be inspected by sufficiently privileged local processes. No command creates a temporary network identity implicitly. Public labels are descriptive metadata and do not select/store a private identity.

## Configuration and automation

The default data directory is `%USERPROFILE%\.ghostnet-cli` on Windows or `$HOME/.ghostnet-cli` elsewhere. It contains `config.json`, `history.ndjson`, and the installed `bridge/`. `GHOSTNET_HOME` overrides the complete data directory and must be absolute, allowing isolated profiles and CI runs. No configuration is read from the working directory.

Endpoint precedence: explicit `--endpoint`, saved `config.json` endpoint, SDK default. Saved configuration is validated even when overriding the endpoint; repair an invalid file rather than silently ignoring it. URLs require `wss://`, a hostname or bracketed IPv6 address, optional port 1-65535, no credentials, whitespace, backslashes or fragments. International hostnames should use ASCII punycode. Do not put secrets in endpoint URLs or labels. Configuration schema 0 from older versions is accepted and upgraded to 1 on save; unknown future versions fail. Saves use exclusive, unique temporary files and rename; concurrent writers are last-writer-wins.

| Environment variable | Purpose |
| --- | --- |
| `GHOSTNET_HOME` | Absolute data directory override |
| `GHOSTNET_SEED` | Identity seed for load, send and listen; seeds are not accepted in argv |
| `GHOSTNET_NODE` | Node executable path (no shell command or extra arguments) |
| `GHOSTNET_NPM` | npm executable path; default `npm.cmd` on Windows, `npm` elsewhere |
| `GHOSTNET_TIMEOUT_SECS` | One-shot bridge and listener connection deadline, integer 1-86400; default 60 seconds |
| `NO_COLOR` | Disable ANSI colors |

Setup has a separate fixed 300-second deadline. It requires `--sdk-version` with an exact semantic version, invokes npm with `--save-exact --ignore-scripts`, and creates a package lock in the private bridge directory. A connected listener has no overall runtime deadline. `info`, `status` and doctor readiness check local files; they do not prove SDK import compatibility, relay connectivity or seed validity. npm registry/auth/cache configuration follows npm's standard environment/configuration. Setup captures npm output and returns actionable failure status without forwarding potentially sensitive package-manager logs; inspect npm's local debug log for detailed install failures.

Exit codes: 0 success/help/version, 2 clap usage errors, 1 runtime/configuration/validation failures. SDK SIGINT handling uses 130, while platform signal forwarding through Rust/npm can vary. Ordinary errors have no stack trace. Failed listener responses, malformed JSON, failed child status and missing success fields cannot produce successful command completion.

Without a subcommand, the interactive shell opens only when stdin and stdout are terminals. Piped/no-input invocations show help and exit successfully. Use explicit commands in scripts; piping command batches into the old REPL is no longer supported. REPL built-ins: `help`, `exit`, `quit`, `:q`, `clear`, `cls`. Single/double and smart quotes group arguments; this is not a shell and does not expand variables or execute shell syntax.

`config show --json` and `history --json` produce JSON without banners. History contains timestamps, direction, public peer IDs, byte counts and outcomes, never seed phrases or message bodies. History corruption is reported on stderr while intact entries remain readable. A metadata write failure after SDK success is a warning so callers do not retry an already completed send. There is no history rotation yet.

## Development and release

```sh
npm test
npm run check
npm run build
npm pack --dry-run --ignore-scripts
```

Tests use Rust's built-in harness and Node's built-in test runner. Real CLI/Node subprocesses are exercised with isolated homes. SDK fixtures test the bridge protocol only; they do not replace real SDK/relay validation. CI runs on Windows, Linux and macOS; release publication waits for CI. These hosted jobs were authored but not executed locally.

For a new release, choose a new matching version in package.json, Cargo.toml and Cargo.lock, tag that version, build the platform assets, then run `node scripts/gen-checksums.mjs vNEW_VERSION` against those exact assets before npm publication. Review and commit the resulting checksums. Existing checksums pin the previous published binaries and are intentionally unchanged by this source audit. Never publish changed binaries under an existing version. Users must supply the intended exact SDK version to setup until the package owner publishes and documents a supported version.

The former launcher health beacon and embedded credential were removed. If that credential was valid, its owner should rotate it; repository changes cannot revoke a remote credential. There is no automatic telemetry, plugin system, project-level identity configuration, secret persistence or destructive network command.

## License

MIT - N11X Collective
