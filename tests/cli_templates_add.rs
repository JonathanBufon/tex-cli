use std::os::unix::fs::PermissionsExt;
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

fn add_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.args(["templates", "add"]);
    cmd
}

#[test]
fn add_creates_template_from_source() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    let source = home.path().join("source.tex");
    let payload = b"\\documentclass{article}\nHello\n";
    std::fs::write(&source, payload).unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .assert()
        .success();

    let dest = templates.join("source.tex");
    assert!(dest.exists());
    assert_eq!(std::fs::read(&dest).unwrap(), payload);
    let mode = std::fs::metadata(&dest).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o644, "expected 0644, got {:o}", mode & 0o777);
}

#[test]
fn add_with_name_uses_custom_name() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    let source = home.path().join("x.tex");
    std::fs::write(&source, b"% x\n").unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .args(["--name", "y"])
        .assert()
        .success();

    assert!(templates.join("y.tex").exists());
    assert!(!templates.join("x.tex").exists());
}

#[test]
fn add_binary_source_exits_21() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    let source = home.path().join("bin.tex");
    // 0xFF is invalid as a UTF-8 continuation on its own.
    std::fs::write(&source, [0xFFu8, 0xFEu8, 0xFDu8]).unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .assert()
        .failure()
        .code(21)
        .stderr(predicate::str::contains("UTF-8"));

    assert!(!templates.join("bin.tex").exists());
}

#[test]
fn add_overwrite_without_force_non_tty_exits_15() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("x.tex"), b"original\n").unwrap();
    write_config(&home, &templates);

    let source = home.path().join("x.tex");
    std::fs::write(&source, b"new\n").unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .assert()
        .failure()
        .code(15);

    assert_eq!(
        std::fs::read(templates.join("x.tex")).unwrap(),
        b"original\n"
    );
}

#[test]
fn add_force_overwrites_existing() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("x.tex"), b"original\n").unwrap();
    write_config(&home, &templates);

    let source = home.path().join("src.tex");
    std::fs::write(&source, b"new\n").unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .args(["--name", "x", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("overwritten"));

    assert_eq!(std::fs::read(templates.join("x.tex")).unwrap(), b"new\n");
}

#[test]
fn add_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    let source = home.path().join("x.tex");
    std::fs::write(&source, b"% ok\n").unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .assert()
        .failure()
        .code(10);
}

#[test]
fn add_templates_dir_missing_exits_22() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("nope");
    write_config(&home, &templates);

    let source = home.path().join("x.tex");
    std::fs::write(&source, b"% ok\n").unwrap();

    add_cmd(&home)
        .arg(source.to_str().unwrap())
        .assert()
        .failure()
        .code(22);
}
