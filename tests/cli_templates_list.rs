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

fn list_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.args(["templates", "list"]);
    cmd
}

fn seed_templates(dir: &Path, names: &[&str]) {
    std::fs::create_dir_all(dir).unwrap();
    for n in names {
        std::fs::write(dir.join(n), format!("% template {n}\n")).unwrap();
    }
}

#[test]
fn list_humano_shows_three_templates() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    seed_templates(&templates, &["artigo.tex", "carta.tex", "relatorio.tex"]);
    write_config(&home, &templates);

    let assert = list_cmd(&home).assert().success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("NOME"));
    assert!(stdout.contains("TAMANHO"));
    assert!(stdout.contains("MODIFICADO"));
    assert!(stdout.contains("artigo"));
    assert!(stdout.contains("carta"));
    assert!(stdout.contains("relatorio"));
}

#[test]
fn list_json_produces_valid_array() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    seed_templates(&templates, &["artigo.tex", "carta.tex", "relatorio.tex"]);
    write_config(&home, &templates);

    let output = list_cmd(&home).args(["--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is valid JSON");
    let arr = parsed.as_array().expect("top-level is an array");
    assert_eq!(arr.len(), 3);
    for obj in arr {
        assert!(obj.get("name").is_some());
        assert!(obj.get("path").is_some());
        assert!(obj.get("size_bytes").is_some());
        assert!(obj.get("modified_at_epoch").is_some());
    }
    // Sorted alphabetically
    let names: Vec<&str> = arr.iter().map(|o| o["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["artigo", "carta", "relatorio"]);
}

#[test]
fn list_ignores_non_tex_files() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("artigo.tex"), "% ok").unwrap();
    std::fs::write(templates.join("refs.bib"), "% bib").unwrap();
    std::fs::write(templates.join("img.png"), [0u8, 1, 2]).unwrap();
    write_config(&home, &templates);

    let output = list_cmd(&home).args(["--format", "json"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["name"], "artigo");
}

#[test]
fn list_ignores_subdirs() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(templates.join("sub")).unwrap();
    std::fs::write(templates.join("artigo.tex"), "% root").unwrap();
    std::fs::write(templates.join("sub").join("nested.tex"), "% nested").unwrap();
    write_config(&home, &templates);

    let output = list_cmd(&home).args(["--format", "json"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["name"], "artigo");
}

#[test]
fn list_empty_dir_prints_message_and_exits_0() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    list_cmd(&home)
        .assert()
        .success()
        .stdout(predicate::str::contains("Nenhum template encontrado"));
}

#[test]
fn list_empty_dir_json_returns_empty_array() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    std::fs::create_dir_all(&templates).unwrap();
    write_config(&home, &templates);

    let output = list_cmd(&home).args(["--format", "json"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

#[test]
fn list_templates_dir_missing_exits_22() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("does-not-exist");
    write_config(&home, &templates);

    list_cmd(&home)
        .assert()
        .failure()
        .code(22)
        .stderr(predicate::str::contains("config set paths.templates_dir"));
}

#[test]
fn list_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    list_cmd(&home)
        .assert()
        .failure()
        .code(10)
        .stderr(predicate::str::contains("tex-cli init"));
}
