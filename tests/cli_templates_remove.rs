use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_config(home: &TempDir, templates_dir: &Path) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let body = format!(
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
    std::fs::write(&cfg, body).unwrap();
}

fn remove_cmd(home: &TempDir, name: &str) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.args(["templates", "remove", name]);
    cmd
}

#[test]
fn remove_without_force_non_tty_exits_15_and_keeps_file() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("carta.tex"), b"payload\n").unwrap();
    write_config(&home, &templates);

    remove_cmd(&home, "carta").assert().failure().code(15);

    assert!(templates.join("carta.tex").exists());
}

#[test]
fn remove_with_force_deletes_file() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("carta.tex"), b"payload\n").unwrap();
    write_config(&home, &templates);

    remove_cmd(&home, "carta")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("removido"));

    assert!(!templates.join("carta.tex").exists());
}

#[test]
fn remove_nonexistent_exits_20() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    remove_cmd(&home, "nada")
        .arg("--force")
        .assert()
        .failure()
        .code(20)
        .stderr(predicate::str::contains("nada").and(predicate::str::contains("não existe")));
}

#[test]
fn remove_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    remove_cmd(&home, "carta")
        .arg("--force")
        .assert()
        .failure()
        .code(10);
}

#[test]
fn remove_templates_dir_missing_exits_22() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("nope");
    write_config(&home, &templates);

    remove_cmd(&home, "carta")
        .arg("--force")
        .assert()
        .failure()
        .code(22);
}
