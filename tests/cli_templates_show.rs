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

fn show_cmd(home: &TempDir, name: &str) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.args(["templates", "show", name]);
    cmd
}

#[test]
fn show_prints_raw_content() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    let content = b"\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n";
    std::fs::write(templates.join("artigo.tex"), content).unwrap();
    write_config(&home, &templates);

    let output = show_cmd(&home, "artigo").output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, content);
}

#[test]
fn show_accepts_name_with_or_without_extension() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    let content = b"% shared\n";
    std::fs::write(templates.join("artigo.tex"), content).unwrap();
    write_config(&home, &templates);

    let without = show_cmd(&home, "artigo").output().unwrap();
    let with = show_cmd(&home, "artigo.tex").output().unwrap();
    assert_eq!(without.stdout, with.stdout);
    assert_eq!(without.stdout, content);
}

#[test]
fn show_missing_template_exits_20() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    show_cmd(&home, "parecer")
        .assert()
        .failure()
        .code(20)
        .stderr(predicate::str::contains("parecer").and(predicate::str::contains("não existe")));
}

#[test]
fn show_case_sensitive() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("artigo.tex"), b"lower\n").unwrap();
    std::fs::write(templates.join("Artigo.tex"), b"upper\n").unwrap();
    write_config(&home, &templates);

    let lower = show_cmd(&home, "artigo").output().unwrap();
    let upper = show_cmd(&home, "Artigo").output().unwrap();
    assert_eq!(lower.stdout, b"lower\n");
    assert_eq!(upper.stdout, b"upper\n");
}

#[test]
fn show_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    show_cmd(&home, "anything")
        .assert()
        .failure()
        .code(10);
}

#[test]
fn show_templates_dir_missing_exits_22() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("nope");
    write_config(&home, &templates);

    show_cmd(&home, "anything")
        .assert()
        .failure()
        .code(22);
}
