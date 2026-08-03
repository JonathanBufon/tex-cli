//! Integration tests for spec 007 US4 (T038): zero-config first run.
//!
//! Verifies FR-007 — running the pipeline against a JSON with no
//! pre-existing config auto-creates the config at the canonical XDG
//! location, reports where it landed, and continues into the pipeline
//! (which then fails with the compile-step error since tectonic isn't
//! available locally; that failure mode is fine — the test asserts that
//! the config path was created, not that a PDF was produced).

use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn base_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("XDG_DATA_HOME", home.path().join(".local/share"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd
}

fn seed_json(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(format!("{name}.json"));
    std::fs::write(&p, body).unwrap();
    p
}

/// FR-007 core: build --json in a home with NO config file → the config
/// is auto-created at the canonical XDG location and a "note" is printed
/// to stderr naming the path.
#[test]
fn build_json_bootstraps_config_when_missing() {
    let home = TempDir::new().unwrap();
    let json = seed_json(
        &home.path().join("d"),
        "d",
        // Deliberately pick a document.type that has no built-in match so
        // the pipeline stops early with exit 82 — proving the bootstrap
        // step succeeded and the resolver ran.
        r#"{"document":{"type":"nonexistent"}}"#,
    );

    let cfg_path = home.path().join(".config/tex/config.toml");
    assert!(!cfg_path.exists(), "precondition: config absent");

    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        // 82 = NoTemplateMatch — pipeline reached resolution.
        .code(82)
        .stderr(predicate::str::contains("no config found"))
        .stderr(predicate::str::contains(cfg_path.to_str().unwrap()));

    assert!(cfg_path.exists(), "config should have been created");

    let body = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(body.contains("[paths]"));
    assert!(body.contains("[compiler]"));
    assert!(body.contains("tectonic"));
}

/// Second invocation must NOT re-print the "note: no config found" line —
/// bootstrap only fires on the first run.
#[test]
fn build_json_after_bootstrap_does_not_re_announce() {
    let home = TempDir::new().unwrap();
    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"type":"nonexistent"}}"#,
    );

    // First run — creates config.
    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .code(82);

    // Second run — same code, but the note MUST be absent.
    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .code(82)
        .stderr(predicate::str::contains("no config found").not());
}

/// Zero-config bootstrap only fires on the `--json` path per US4 scope.
/// Explicit `build <template> <json>` continues to error with the
/// pre-007 `ConfigMissing` (exit 10) — that path is not touched by US4.
#[test]
fn explicit_build_without_config_still_errors_10() {
    let home = TempDir::new().unwrap();
    let json = seed_json(&home.path().join("d"), "d", r#"{"titulo":"x"}"#);

    base_cmd(&home)
        .args(["build", "artigo", json.to_str().unwrap()])
        .assert()
        .code(10)
        .stderr(predicate::str::contains("No config"));
}
