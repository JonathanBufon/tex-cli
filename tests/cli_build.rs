use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

const MINIMAL_TEMPLATE: &str = r#"\documentclass{article}
\begin{document}
{{ titulo }}
\end{document}
"#;

const BROKEN_TEMPLATE: &str = r#"\documentclass{article}
\begin{document}
{{ titulo }}
\undefinedcommand{oops}
"#;

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_config(home: &TempDir, templates_dir: &Path, output_dir: &Path) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::create_dir_all(templates_dir).unwrap();
    std::fs::create_dir_all(output_dir).unwrap();
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "{}"

[compiler]
engine = "tectonic"
keep_tex = false
keep_logs = false

[behavior]
ask_output_path_every_time = false
"#,
        templates_dir.display(),
        output_dir.display()
    );
    std::fs::write(&cfg, body).unwrap();
}

fn build_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.arg("build");
    cmd
}

fn seed_template(templates_dir: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(templates_dir).unwrap();
    let path = templates_dir.join(format!("{name}.tex"));
    std::fs::write(&path, body).unwrap();
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn build_produces_pdf_with_default_output_path() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"Meu Artigo"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success();

    let pdf = output.join("artigo.pdf");
    assert!(pdf.exists(), "PDF should exist at {pdf:?}");
}

#[test]
fn build_pdf_has_pdf_magic_bytes() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success();

    let bytes = std::fs::read(output.join("artigo.pdf")).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn build_pdf_has_mode_0644() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success();

    let mode = std::fs::metadata(output.join("artigo.pdf"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644);
}

#[test]
fn build_stdout_mentions_path_and_total_duration() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("PDF gerado em")
                .and(predicate::str::contains(
                    "Pipeline (render + compile) levou",
                ))
                .and(predicate::str::contains("s.")),
        );
}

#[test]
fn build_with_output_flag_uses_custom_path() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let custom = home.path().join("custom.pdf");
    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--output"])
        .arg(custom.to_str().unwrap())
        .assert()
        .success();

    assert!(custom.exists());
    assert!(!output.join("artigo.pdf").exists());
}

#[test]
fn build_missing_template_exits_20() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{}"#);

    build_cmd(&home)
        .args(["nao-existe", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(20);
}

#[test]
fn build_malformed_json_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "bad", "not json {");

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(31)
        .stderr(predicate::str::contains("JSON inválido"));
}

#[test]
fn build_json_top_level_array_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "arr", r#"["x"]"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(31);
}

#[test]
fn build_tera_error_exits_30() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    // JSON does not contain the {{ titulo }} variable.
    let data = seed_json(&home.path().join("d"), "empty", r#"{}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(30);
}

#[test]
fn build_broken_tex_after_render_exits_40_with_log_tail() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "broken", BROKEN_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["broken", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(40)
        .stderr(
            predicate::str::contains("Falha ao compilar")
                .and(predicate::str::contains("broken")),
        );

    assert!(
        !output.join("broken.pdf").exists(),
        "no PDF should be written on compile failure"
    );
}

#[test]
fn build_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    let data = seed_json(&home.path().join("d"), "d", r#"{}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(10);
}

#[test]
fn build_overwrite_without_force_non_tty_exits_15() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let pdf = output.join("artigo.pdf");
    std::fs::write(&pdf, b"original PDF").unwrap();

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(15);

    assert_eq!(std::fs::read(&pdf).unwrap(), b"original PDF");
}

#[test]
fn build_force_overwrites_existing_pdf() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let pdf = output.join("artigo.pdf");
    std::fs::write(&pdf, b"original").unwrap();

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sobrescrito"));

    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}
