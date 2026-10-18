# CLI audit

## Original inspection (before implementation)

Inspected all tracked source, README (the only Markdown documentation), both package manifests, Cargo manifest, installer/launcher/checksum scripts, release workflow and ignore rules. Clean initial working tree. Baseline: `cargo test --locked` passed the single endpoint unit test. No integration tests or PR CI existed. Findings below describe the original checkout, not assumptions about a different GhostNet/Hecate checkout.

### Architecture and execution

Rust 2021 executable `ghostnet`, clap 4 derive parser, anyhow errors, serde JSON and colored terminal output. npm registers `scripts/run.js`, which launches the downloaded native binary; `scripts/install.js` downloads a platform release with package-pinned SHA-256 verification. `main::main` parses arguments, prints a banner and calls `commands::dispatch`; no command starts `repl::run`, which tokenizes input and reuses clap/dispatch. Local commands use `config` and `history`. SDK commands use `node::build_command` to spawn Node directly in the per-user bridge directory, passing seeds in the child environment. Embedded `bridge/ghostnet-bridge.mjs` imports `@n11x/ghostnet-sdk`, calls identity/connect/send APIs, and emits NDJSON. Rust renders the response. Setup writes embedded bridge files and runs npm there. This is an encrypted messaging client, not a task runner or general shell.

### Actual command reference at inspection

| Command | Original behavior |
| --- | --- |
| no command | REPL, including when stdin is a pipe; errors do not affect final status |
| setup | Writes embedded bridge files, runs npm install, checks npm status |
| info | Version, package and directory-existence SDK readiness |
| doctor | Probes Node/npm, checks configuration, always returns success |
| status | Reports saved endpoint, public label count and seed environment presence |
| config show / set-endpoint / clear-endpoint | Reads/writes non-secret per-user JSON |
| history --limit N | Reverse metadata history, silently skips corrupt records |
| identity create / load [seed] [--name NAME] | SDK identity operation, optional public label; missing response fields become `(unknown)` |
| identity list / remove NAME | Local public label management |
| send PEER MESSAGE [--seed SEED] [--endpoint URL] | Requires seed, checks 64 KiB limit, invokes bridge, records metadata |
| listen [--seed SEED] [--endpoint URL] | Streams NDJSON; ignores child failure status |

### Checklist and evidence

| Area | Original status | Evidence / gap |
| --- | --- | --- |
| 1 Entry/structure | Implemented | Cargo bin, npm bin, `cli::Commands`, `commands::dispatch`; modular handlers and explicit bridge cwd |
| 2 Parsing/validation | Partial | clap rejects unknown/missing arguments; `config::validate_endpoint` accepts invalid authorities; empty seeds/messages inconsistent; bridge reparses message strings beginning `--` as flags |
| 3 Help/version | Partial | clap root/nested help and `--version`; missing `-v`; Cargo 0.2.0 vs npm 0.2.12; README omits existing commands |
| 4 Configuration | Partial | User JSON and environment seed/executable overrides exist; derived default writes version 0, loaded endpoints/schema unvalidated; fixed temp filename races in `config::save` |
| 5 Execution | Partial | Direct Node spawning and async SDK calls; `node::run_bridge` has no deadline; failed sends skip disconnect; listener ignores failure and does not guarantee cleanup |
| 6 Output/UX | Partial | Readable output and no-color; unconditional banner pollutes pipes; SDK result fields not verified; REPL in CI; no structured metadata export |
| 7 Exit/errors | Partial | clap 2 / runtime 1 / success 0; `node::parse_last_json` ignores exit status and absent ok; `doctor::run` always succeeds; listener error events do not fail command |
| 8 Interactive | Partial | REPL supports quoting and EOF; should require terminal; no destructive network operations requiring confirmation |
| 9 Diagnostics | Partial | Doctor exists; no debug chain; `scripts/run.js::reportHealth` embeds a credential in child argv and reports healthy before execution |
| 10 Security/reliability | Partial | HTTPS/hash/size/redirect protections and no seed persistence already exist; installer lacks request timeout/response failure handling; endpoint validation and bridge argument ambiguity; unrequested health POST with shipped credential |
| 11 Maintainability | Implemented | Shared parser/dispatch, separate services and commands; plugin architecture not applicable to this focused messaging CLI |
| 12 Distribution | Partial | Five release targets and pinned actions; ignored Cargo.lock, unlocked release build, no release version gate, no tests in release; SDK `latest` is not reproducible |
| 13 Tests/docs | Partial | Only one endpoint test; no subprocess/failure/config precedence/installer tests; README makes unsupported audited-SDK claim |

### Ranked original findings

- **P1** False success: `node::parse_last_json`, `commands::listen::run`, `commands::doctor::run`, `commands::identity::field`, `commands::send::run`. A failing child or malformed success response can appear successful. Verified statically.
- **P1** Bridge argument ambiguity: `bridge/ghostnet-bridge.mjs` flag loop consumes user messages beginning `--`; send/listen bridge paths allow ephemeral identities. Verified statically.
- **P1** Embedded health credential and unsolicited health reporting: `scripts/run.js::reportHealth` places credential in `node -e` argv. Credential validity/external service authorization not verified. Remove shipped default and make reporting explicit, with no credentials in argv.
- **P1** Missing execution deadline/cleanup: `node::run_bridge`, bridge send/listen paths. Network hangs and skipped disconnect are possible; no claim that a production hang was reproduced.
- **P2** Config validation/default/temp-file defects: `config::{load,save,endpoint,validate_endpoint}`. Invalid persisted endpoint/version accepted; default schema 0; fixed temp path can collide/follow pre-existing symlink.
- **P2** Noninteractive/diagnostics gaps: `main::main`, `repl::run`, `history::recent`; no-arg pipe starts REPL, banners contaminate data, corrupt history is silently discarded, history reads entire file.
- **P2** Release and docs drift: manifest versions mismatch, ignored lock, no test gate, Node 18 checksum generator uses newer JSON import attributes; mutable SDK latest. SDK compatibility and published checksums require external verification before changing the dependency pin/releasing.
- **P2** Installer network failures: `scripts/install.js::download` lacks timeout and response error/aborted handling and exclusive temp-file creation.
- **P3** Optional broader JSON/quiet/progress support. Prioritize JSON for metadata and keep long-lived listen streaming. No project configuration needed: credentials/identities belong to the user rather than cwd. No credential vault, plugin system, log daemon or forced prompts needed.

No P0 was demonstrated. SDK cryptography, relay delivery, remote release hashes and external service behavior were not verified by this source inspection.

## Implementation and verification

### Changes made

1. **Failure correctness:** `node::{parse_last_json,run_bridge_json}` require successful process status and explicit `ok:true`, reject malformed/empty output and redact seeds from structured bridge errors. `commands::send::run` requires `sent:true`; identity creation/restoration requires actual response fields before reporting success. `commands::listen::run` rejects malformed/error events and nonzero child status. `commands::doctor::run` returns nonzero when checks fail and requires Node major version >=18.
2. **Execution reliability:** `node::{capture,ChildGuard,timeout}` provide subprocess deadlines, concurrent pipe draining, 4 MiB capture limits, kill/reap on early return and noninteractive stdin. One-shot bridge deadline defaults to 60 seconds, configurable with `GHOSTNET_TIMEOUT_SECS` (1-86400). Setup deadline is 300 seconds. Listener connection deadline is enforced in the embedded bridge, then cleared for continuous streaming; listener JSON event size is bounded to 1 MiB. Bridge send disconnects in `finally`; bridge installs SIGINT/SIGTERM disconnect handlers for listening. No new dependencies.
3. **Bridge input handling:** the fixed positional arity in `bridge/ghostnet-bridge.mjs` preserves message text beginning `--`; unknown flags/missing values fail. Both send/listen bridge paths require a seed rather than creating throwaway identities. Empty/multibyte oversized messages are rejected. Rust seed resolution rejects explicit whitespace-only seeds and removes inherited seed values from identity creation and npm/probe subprocesses. Listener message display strips control characters to limit terminal escape injection.
4. **Configuration:** one shared `config::data_dir` supports absolute `GHOSTNET_HOME` for isolated profiles. Defaults use schema 1; loads validate schema, endpoint and labels; older schema 0 is accepted and upgraded on save. Endpoint validation checks host/IPv6/port, forbids userinfo, fragments, backslashes, whitespace and controls. `config::save` uses unique exclusive temp files, sync and rename, with Unix mode 0600. Invalid config reports its path; `--debug` shows context. Endpoint and seed precedence are tested. There is no second configuration system.
5. **Output and automation:** `main::main` omits one-shot banners and opens the REPL only with terminal stdin/stdout; other no-command invocations show help. `-v` is available alongside `--version`; NO_COLOR and redirected output suppress colors. `config show --json` and `history --json` provide clean structured output. REPL clap errors use clap's intended output stream; REPL `--no-color` is honored. Root help includes examples.
6. **History reliability:** `history::recent` streams the file, retaining at most the requested number of records, and warns about malformed records. A metadata write failure following an SDK-confirmed send is a warning, avoiding a failure status that encourages duplicate delivery. Received-message metadata write errors are also visible. No message content or seeds are recorded.
7. **Installer and launcher:** removed automatic health POST and shipped credential from `scripts/run.js`; no telemetry replacement was added. `scripts/install.js::download` uses stream pipeline for reliable error/abort propagation and closed-file cleanup, an immediate response error handler (a regression test exposed the early-error race), exclusive temp creation, 30-second inactivity timeout and 120-second absolute timeout per download hop. Existing SHA-256/host/size/redirect protections are retained and tested. `scripts/gen-checksums.mjs` reads JSON in a Node-18-compatible manner.
8. **Distribution:** Cargo version now matches existing npm version 0.2.12. Cargo.lock is no longer ignored and is included in the working tree; builds/tests use `--locked`. `.github/workflows/ci.yml` runs formatting, clippy and Rust/Node tests on Windows/Linux/macOS, checks matching metadata/tag versions, and is a prerequisite for release publication. Release matrix uses locked builds. No release was published and no old checksums were replaced with hashes of locally changed binaries.
9. **Documentation:** README now lists every command/option, safe PowerShell seed input, environment overrides, precedence, statuses, script behavior, migration notes, release process and test limitations. Removed the unsubstantiated audited-SDK claim.

### Newly verified external blocker (P1, unresolved)

On 2026-10-19 local time, a real `ghostnet setup` using an isolated `target/audit-runtime` home failed with status 1. Running npm directly to inspect the underlying failure produced:

```text
npm error code E404
npm error 404 Not Found - GET https://registry.npmjs.org/@n11x%2fghostnet-sdk - Not found
npm error 404 '@n11x/ghostnet-sdk@latest' is not in this registry.
```

The intended private registry/package/local SDK source was requested from the user. An E404 does not establish whether the package is removed, renamed or private. Replacing the SDK with a similarly named package without inspecting compatible APIs would be unsafe and is intentionally not done. The bridge dependency remains unchanged. **Setup and live identity/messaging cannot be considered functional against the current public registry.** Local metadata/config/help commands and protocol behavior are independently verified.

### Final area assessment

| Area | Final status / remaining boundary |
| --- | --- |
| Entry and command hierarchy | Implemented; existing public command names preserved |
| Argument parsing and validation | Implemented for local/parser/bridge input; SDK-specific peer/seed semantics remain external |
| Help/version | Implemented and tested, version metadata synchronized |
| Configuration | Implemented and tested; no project-level config needed for user identities |
| Core execution | Partial: tested deadlines/status/cleanup/SDK fixture protocol; real SDK unavailable, cross-platform Ctrl-C/process-tree termination not verified |
| Output/UX | Implemented for human output and metadata JSON; generic quiet/full network JSON modes are optional |
| Exit codes/errors | Implemented for tested success/usage/runtime/failure paths; platform signal forwarding remains unverified |
| Interactive features | Partial: existing REPL retained and parser shared; actual interactive terminal/Ctrl-C sessions not automated in this environment |
| Diagnostics/logging | Implemented local doctor/debug context; no persistent debug logs or telemetry needed |
| Security/reliability | Partial: demonstrated local defects fixed and regression tested; dependency/SDK/relay security is not established by this audit |
| Maintainability | Implemented with existing modules and dependencies |
| Distribution | Partial: native build and package contents verified; fresh release assets/checksums/new version still required before publishing |
| Tests/documentation | Implemented for modified local functionality; real network and hosted CI coverage remain unavailable/unexecuted |

### Verification results

Environment: Windows x64, Cargo/Rust toolchain 1.96.0, Node 22.22.2, npm 10.9.7. Runtime dependency downloads used a workspace-local npm cache, and CLI subprocess tests used disposable per-test absolute homes, not the user's existing configuration.

| Check actually executed | Result |
| --- | --- |
| Original `cargo test --locked` | 1 unit test passed |
| Final `npm test` (`cargo test --locked` + Node test runner) | 15 tests passed: 3 Rust unit, 7 Rust executable integration, 5 Node tests |
| `npm run check` | Formatting and all-target clippy with warnings denied passed |
| `cargo build --locked` | Debug binary built successfully |
| `cargo build --locked --release` | Final release binary built successfully |
| Release binary and isolated npm launcher smoke checks | Correct 0.2.12 version, parseable config/history JSON, and launcher preserves usage exit code 2 |
| `node --check` on bridge and install/run/checksum scripts | Passed |
| `npm pack --dry-run --ignore-scripts --cache target/npm-cache` | Passed; package includes README, package.json, checksums.json and installer/launcher; no local homes, tests, credentials or target artifacts included |
| `git diff --check` | Passed |
| Real isolated setup and direct npm install | Failed as expected with npm E404 above; exact limitation documented |

Integration coverage includes root/nested help, version aliases, unknown/missing arguments and options, usage status 2, noninteractive no-command behavior, JSON parsing, missing/invalid config, secure endpoint boundaries, repeat config replacement, schema migration, public label operations, metadata limits/corruption, seed requirements, child nonzero/missing/malformed responses, deadline termination, explicit seed vs environment precedence, explicit endpoint vs saved precedence, seeds absent from child argv/history, missing identity fields and listener error/oversized event failures.

Node coverage runs the actual embedded bridge with an explicitly synthetic SDK fixture to test flag-like message handling, seed requirements, disconnect-on-error, redaction, connection timeout and identity response protocol. Installer tests use a controlled transport fixture to test host allowlisting, response failures/aborts, size caps, exclusive temp creation and actual installer subprocess success/hash-mismatch/previous-binary preservation/cleanup. These are protocol/distribution regressions, not evidence of actual SDK crypto, relay delivery or genuine public release assets. Intermediate test failures (early response error race and clippy module placement) were corrected and rerun; no failures are hidden by changing expected outcomes.

### Final command reference and migration

The canonical full reference and examples are in [README.md](README.md#commands-and-examples); parser-generated reference is `ghostnet --help` and nested `--help`. New interface additions are `-v`, global `--debug`, `config show --json`, `history --json`, `GHOSTNET_HOME` and `GHOSTNET_TIMEOUT_SECS`. Existing commands and positional/seed/endpoint interfaces remain available.

Automation migration: no-command pipes now print help instead of consuming REPL batches; invoke explicit commands. One-shot commands omit decorative banners. Doctor/listen/bridge failure exits now correctly fail. Future schema versions/invalid stored endpoints now fail rather than being silently used. `identity remove` deletes only public metadata, so no forced confirmation prompt was added. The automatic health beacon is removed entirely; if its former embedded credential was valid, the owner must revoke/rotate it at its issuer (not possible through source edits).

### Intentionally not implemented and remaining work

- **P1:** obtain the intended SDK source/registry and compatible API/version; then pin it, install the real package and test identity create/load and two-peer send/listen against an authorized relay. No live seed or peer endpoint was supplied. No cryptography or delivery claim is made.
- **P2:** exercise hosted three-platform CI, terminal REPL and Ctrl-C on each OS. ChildGuard reaps directly owned children; subprocess-tree cleanup on forced parent termination is not guaranteed or verified. The SDK listener connection timer requires refreshing bridge files with setup after upgrading.
- **P2:** choose a new release version, build/review all platform binaries and regenerate pinned npm checksums from those exact release assets. Existing checksum trust anchors and downloaded binaries were not overwritten. No publication performed.
- **P2:** config read/modify/write is last-writer-wins despite unique atomic replacement; cross-process update locking and history rotation remain future improvements. History read memory is bounded by retained record count plus one record, while scanning still traverses the file.
- **P3 / not applicable:** project-level identity config, plugin architecture, persistent debug log infrastructure, credential vault, mandatory destructive confirmations, and extra quiet/progress frameworks are not required by the current focused client. Explicit commands already operate noninteractively; terminal-only REPL is retained for convenience. JSON is provided where useful for local metadata rather than adding untested network-output modes.

Implementation was kept in the existing Rust/clap/Node architecture. No dependency or framework was added, no Git commit/push/release was performed, and the inaccessible SDK dependency is clearly reported rather than replaced or mocked as a claimed working production backend.

## Integration with Hecate main

After the audit commit was pushed, `main` at `65b2cd0` was merged into `separate-branch` to resolve seven reported conflicts. The original findings above remain historical. The final package/native executable is now **Hecate 0.2.13**, npm package `@n11x/hecate-cli`, with `hecate` command examples and the Hecate repository/download location from main. Cargo.lock was updated only for the root package name/version; dependency versions were preserved.

Resolution retained the audit's installer pipeline/timeouts, telemetry removal, terminal-aware startup, SDK readiness checks, deadlines, exit handling, JSON output, tests and CI. It also retained main's Hecate branding, npm metadata, release binary name and existing 0.2.13 checksums. The `ghostnet-*` release asset names, `@n11x/ghostnet-sdk`, `GHOSTNET_*` environment variables and `.ghostnet-cli` data directory are intentional compatibility names from main. They were not renamed. The SDK E404 limitation remains unchanged.

The test harness now executes the `hecate` binary, expects its version output, and validates installer output named `hecate`/`hecate.exe`. The current canonical command reference is the README and `hecate --help`. No health credential or health reporting was reintroduced while resolving the launcher conflict.

Post-resolution verification passed: all 15 tests via `npm test`, formatting/clippy via `npm run check`, locked release build, all four JavaScript syntax checks, npm package dry run, and whitespace check. The resulting release binary reports `hecate 0.2.13`; its help lists the preserved commands with Hecate examples, and config/history JSON smoke checks passed.
