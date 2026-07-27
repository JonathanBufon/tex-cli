//! T016 — Verify that `--format humano` is hard-rejected by clap
//! after the English-only translation of spec 006.
//!
//! Contract: pre-existing scripts passing `--format humano` must fail
//! with exit code 2 (clap parse error) rather than being silently
//! accepted as an alias. See specs/006-translate-to-english/research.md
//! § D-01 for the rationale.

use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_valid_config(home: &TempDir) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let body = r#"[paths]
templates_dir = "/tmp/templates"
output_dir = "/tmp/output"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#;
    std::fs::write(&cfg, body).unwrap();
}

fn base_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd
}

#[test]
fn config_show_format_humano_is_rejected_exit_2() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    base_cmd(&home)
        .args(["config", "show", "--format", "humano"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("human"));
}

#[test]
fn templates_list_format_humano_is_rejected_exit_2() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    base_cmd(&home)
        .args(["templates", "list", "--format", "humano"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("human"));
}

#[test]
fn config_show_format_human_is_accepted() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    base_cmd(&home)
        .args(["config", "show", "--format", "human"])
        .assert()
        .success();
}

#[test]
fn templates_list_default_format_matches_human() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = std::path::Path::new("/tmp/templates-humano-test");
    std::fs::create_dir_all(templates_dir).ok();

    // Update config to point at an existing dir so list doesn't exit 22.
    let cfg_body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "/tmp/output"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#,
        templates_dir.display()
    );
    std::fs::write(config_path(&home), cfg_body).unwrap();

    let default_out = base_cmd(&home)
        .args(["templates", "list"])
        .output()
        .unwrap();
    let explicit_out = base_cmd(&home)
        .args(["templates", "list", "--format", "human"])
        .output()
        .unwrap();

    assert!(default_out.status.success());
    assert!(explicit_out.status.success());
    assert_eq!(default_out.stdout, explicit_out.stdout);
}
