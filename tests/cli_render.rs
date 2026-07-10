use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_config(home: &TempDir, templates_dir: &Path, output_dir: &Path) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "{}"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#,
        templates_dir.display(),
        output_dir.display()
    );
    std::fs::write(&cfg, body).unwrap();
}

fn render_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.arg("render");
    cmd
}

fn seed_template(templates_dir: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(templates_dir).unwrap();
    std::fs::write(templates_dir.join(format!("{name}.tex")), body).unwrap();
}

#[test]
fn render_writes_file_with_default_output_path() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "Olá, {{ nome }}.\n");
    write_config(&home, &templates, &output);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{"nome":"Ana"}"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Renderizado em"));

    let out = output.join("artigo.tex");
    assert!(out.exists());
    assert_eq!(std::fs::read(&out).unwrap(), b"Ol\xc3\xa1, Ana.\n");
    let mode = std::fs::metadata(&out).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o644);
}

#[test]
fn render_with_output_flag_uses_custom_path() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "{{ x }}\n");
    write_config(&home, &templates, &output);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{"x":"Y"}"#).unwrap();

    let custom = home.path().join("custom.tex");
    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--output"])
        .arg(custom.to_str().unwrap())
        .assert()
        .success();

    assert!(custom.exists());
    assert!(!output.join("artigo.tex").exists());
}

#[test]
fn render_missing_template_exits_20() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();
    write_config(&home, &templates, &output);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{}"#).unwrap();

    render_cmd(&home)
        .args(["nao-existe", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(20);
}

#[test]
fn render_malformed_json_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "{{ x }}\n");
    write_config(&home, &templates, &output);

    let data = home.path().join("bad.json");
    std::fs::write(&data, b"not json {").unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(31)
        .stderr(predicate::str::contains("JSON inválido"));
}

#[test]
fn render_json_top_level_array_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "{{ x }}\n");
    write_config(&home, &templates, &output);

    let data = home.path().join("arr.json");
    std::fs::write(&data, br#"["x"]"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(31)
        .stderr(predicate::str::contains("objeto"));
}

#[test]
fn render_tera_error_exits_30() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "Olá, {{ ausente }}.\n");
    write_config(&home, &templates, &output);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{}"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(30)
        .stderr(predicate::str::contains("artigo"));

    assert!(!output.join("artigo.tex").exists());
}

#[test]
fn render_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{}"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(10);
}

#[test]
fn render_templates_dir_missing_exits_22() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("nope");
    let output = home.path().join("o");
    write_config(&home, &templates, &output);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{}"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(22);
}

#[test]
fn render_overwrite_without_force_non_tty_exits_15() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "{{ x }}\n");
    write_config(&home, &templates, &output);

    let out = output.join("artigo.tex");
    std::fs::write(&out, b"original\n").unwrap();

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{"x":"Y"}"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(15);

    assert_eq!(std::fs::read(&out).unwrap(), b"original\n");
}

#[test]
fn render_force_overwrites_existing() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&output).unwrap();
    seed_template(&templates, "artigo", "{{ x }}\n");
    write_config(&home, &templates, &output);

    let out = output.join("artigo.tex");
    std::fs::write(&out, b"original\n").unwrap();

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{"x":"NOVO"}"#).unwrap();

    render_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sobrescrito"));

    assert_eq!(std::fs::read(&out).unwrap(), b"NOVO\n");
}
