use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ghostnet-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hecate"));
        command
            .args(args)
            .current_dir(&self.0)
            .stdin(Stdio::null())
            .env("GHOSTNET_HOME", &self.0)
            .env("NO_COLOR", "1")
            .env_remove("GHOSTNET_SEED")
            .env_remove("GHOSTNET_NODE")
            .env_remove("GHOSTNET_NPM")
            .env_remove("GHOSTNET_TIMEOUT_SECS");
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn config(&self, content: &str) {
        std::fs::write(self.0.join("config.json"), content).unwrap();
    }
    fn fixture(&self, script: &str) {
        let bridge = self.0.join("bridge");
        let sdk = bridge.join("node_modules/@n11x/ghostnet-sdk");
        std::fs::create_dir_all(&sdk).unwrap();
        std::fs::write(sdk.join("package.json"), "{}").unwrap();
        std::fs::write(bridge.join("ghostnet-bridge.mjs"), script).unwrap();
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into()
}
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into()
}

#[test]
fn help_version_and_usage_exit_codes() {
    let s = Sandbox::new();
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["identity", "--help"],
        vec!["config", "show", "--help"],
        vec!["send", "--help"],
    ] {
        let o = s.run(&args);
        assert!(o.status.success(), "{}", stderr(&o));
        assert!(stdout(&o).contains("Usage:"));
    }
    for flag in ["--version", "-v"] {
        let o = s.run(&[flag]);
        assert!(o.status.success());
        assert_eq!(
            stdout(&o).trim(),
            concat!("hecate ", env!("CARGO_PKG_VERSION"))
        );
    }
    for args in [
        vec!["unknown"],
        vec!["send"],
        vec!["send", "peer"],
        vec!["listen", "--endpoint"],
        vec!["history", "--limit", "abc"],
        vec!["info", "--unknown"],
    ] {
        let o = s.run(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
        assert!(stderr(&o).contains("error:"));
        assert!(o.stdout.is_empty());
    }
}

#[test]
fn noninteractive_no_command_prints_help_without_prompt() {
    let s = Sandbox::new();
    let o = s.run(&[]);
    assert!(o.status.success());
    assert!(stdout(&o).contains("Usage:"));
    assert!(!stdout(&o).contains("Welcome,"));
    assert!(!stdout(&o).contains('\u{1b}'));
}

#[test]
fn configuration_roundtrip_validation_and_repair_guidance() {
    let s = Sandbox::new();
    let o = s.run(&["config", "show", "--json"]);
    assert!(o.status.success());
    let value: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(value["version"], 1);
    assert!(s
        .run(&["config", "set-endpoint", "wss://relay.example/path"])
        .status
        .success());
    assert!(s
        .run(&["config", "set-endpoint", "wss://other.example:443"])
        .status
        .success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&s.run(&["config", "show", "--json"]).stdout)
            .unwrap()["endpoint"],
        "wss://other.example:443"
    );
    assert!(s.run(&["config", "clear-endpoint"]).status.success());
    for endpoint in [
        "http://relay.example",
        "wss://",
        "wss://?missing",
        "wss://user:password@host",
        "wss://host:0",
        "wss://host:65536",
        "wss://host\\bad",
        "wss://host/#fragment",
    ] {
        let o = s.run(&["config", "set-endpoint", endpoint]);
        assert_eq!(o.status.code(), Some(1), "{endpoint}");
    }
    for config in ["{", "{\"version\":99}", "{\"endpoint\":\"ws://bad\"}"] {
        s.config(config);
        let o = s.run(&["config", "show"]);
        assert_eq!(o.status.code(), Some(1));
        assert!(stderr(&o).contains("config"));
        assert!(!stderr(&o).contains("stack backtrace"));
    }
    s.config("{\"version\":0}");
    assert!(s.run(&["config", "clear-endpoint"]).status.success());
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(s.0.join("config.json")).unwrap()).unwrap();
    assert_eq!(config["version"], 1);
    assert!(!s
        .command(&["info"])
        .env("GHOSTNET_HOME", "relative")
        .output()
        .unwrap()
        .stdout
        .is_empty());
    assert_eq!(
        s.command(&["config", "show"])
            .env("GHOSTNET_HOME", "relative")
            .output()
            .unwrap()
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn local_labels_and_history_never_need_sdk() {
    let s = Sandbox::new();
    assert!(s.run(&["identity", "list"]).status.success());
    s.config(r#"{"identities":{"laptop":{"node_id":"public-id","created_at":1}}}"#);
    assert!(stdout(&s.run(&["identity", "list"])).contains("public-id"));
    assert!(s.run(&["identity", "remove", "laptop"]).status.success());
    assert_eq!(
        s.run(&["identity", "remove", "missing"]).status.code(),
        Some(1)
    );
    std::fs::write(s.0.join("history.ndjson"), "{\"timestamp\":1,\"direction\":\"sent\",\"peer\":\"public-id\",\"bytes\":5,\"outcome\":\"sent\"}\ncorrupt\n").unwrap();
    let o = s.run(&["history", "--json", "--limit", "1"]);
    assert!(o.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&o.stdout)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(stderr(&o).contains("malformed"));
    let o = s.run(&["history", "--json", "--limit", "0"]);
    assert_eq!(stdout(&o).trim(), "[]");
}

#[test]
fn missing_runtime_and_invalid_inputs_fail_without_success() {
    let s = Sandbox::new();
    for args in [
        vec!["identity", "create"],
        vec!["identity", "load"],
        vec!["send", "peer", "hi"],
        vec!["listen"],
        vec!["send", "", "hi"],
        vec!["send", "peer", ""],
    ] {
        let o = s.run(&args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert!(!stdout(&o).contains("Message sent."));
    }
    let o = s
        .command(&["doctor"])
        .env("GHOSTNET_NODE", s.0.join("missing-node"))
        .env("GHOSTNET_NPM", s.0.join("missing-npm"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("diagnostic checks failed"));
    let o = s
        .command(&["send", "peer", "hi", "--seed", " "])
        .output()
        .unwrap();
    assert!(stderr(&o).contains("identity is required"));
    let large = "é".repeat(32769);
    // Windows argv has a smaller limit: validate the byte boundary at unit level instead.
    if !cfg!(windows) {
        assert_eq!(s.run(&["send", "peer", &large]).status.code(), Some(1));
    }
}

#[test]
fn real_child_protocol_exit_status_timeout_and_endpoint_precedence() {
    let s = Sandbox::new();
    s.fixture("console.log(JSON.stringify({ok:true,sent:true}));process.exitCode=7;");
    let o = s
        .command(&["send", "peer", "hi"])
        .env("GHOSTNET_SEED", "test seed")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(!stdout(&o).contains("Message sent."));
    for script in [
        "console.log('{}')",
        "console.log('not-json')",
        "",
        "console.log('{\"ok\":true}')",
    ] {
        s.fixture(script);
        let o = s
            .command(&["send", "peer", "hi"])
            .env("GHOSTNET_SEED", "test seed")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(1), "{script}");
    }
    s.fixture("process.stdout.write('x'.repeat(1024*1024+1));setInterval(()=>{},1000)");
    let o = s
        .command(&["listen"])
        .env("GHOSTNET_SEED", "test seed")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("1 MiB limit"));
    s.fixture("setInterval(()=>{},1000)");
    let start = std::time::Instant::now();
    let o = s
        .command(&["send", "peer", "hi"])
        .env("GHOSTNET_SEED", "test seed")
        .env("GHOSTNET_TIMEOUT_SECS", "1")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("timed out"));
    assert!(start.elapsed().as_secs() < 10);
    s.fixture("import fs from 'node:fs';fs.writeFileSync('args.json',JSON.stringify({args:process.argv.slice(2),seed:process.env.GHOSTNET_SEED}));console.log(JSON.stringify({ok:true,sent:true}));");
    s.config(r#"{"endpoint":"wss://saved.example"}"#);
    for (extra, expected) in [
        (vec![], "wss://saved.example"),
        (
            vec!["--endpoint", "wss://explicit.example"],
            "wss://explicit.example",
        ),
    ] {
        let mut args = vec!["send", "peer", "hello", "--seed", "explicit seed"];
        args.extend(extra);
        let o = s
            .command(&args)
            .env("GHOSTNET_SEED", "environment seed")
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", stderr(&o));
        let invoked: serde_json::Value =
            serde_json::from_slice(&std::fs::read(s.0.join("bridge/args.json")).unwrap()).unwrap();
        assert_eq!(invoked["seed"], "explicit seed");
        assert_eq!(invoked["args"][4], expected);
        assert!(!invoked["args"].to_string().contains("seed"));
    }
    let history = std::fs::read_to_string(s.0.join("history.ndjson")).unwrap();
    assert!(!history.contains("hello"));
    assert!(!history.contains("seed"));
}

#[test]
fn listener_failures_and_incomplete_identities_fail() {
    let s = Sandbox::new();
    for script in [
        "console.log('{\"ok\":false,\"error\":\"test failure\"}')",
        "console.log('invalid')",
        "process.exit(9)",
    ] {
        s.fixture(script);
        let o = s
            .command(&["listen"])
            .env("GHOSTNET_SEED", "test seed")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(1), "{script}");
    }
    s.fixture("console.log('{\"ok\":true}')");
    let o = s.run(&["identity", "create"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!stdout(&o).contains("New identity created"));
}
