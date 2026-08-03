//! Integration tests for spec 007 US1 (T008):
//! `tex-cli build --json <source>` — the JSON→template auto-resolution path.
//!
//! Success-path tests require `tectonic` on PATH (per project Docker convention).
//! Error-path tests short-circuit before compile and pass unconditionally.

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

fn write_config(home: &TempDir, templates_dir: &Path, output_dir: &Path) {
    let cfg = home.path().join(".config").join("tex").join("config.toml");
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
    std::fs::write(templates_dir.join(format!("{name}.tex")), body).unwrap();
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, body).unwrap();
    path
}

/// Case A of contracts/json-document-fields.md — built-in match by
/// `document.type`. Requires tectonic (produces a real PDF).
#[test]
fn build_json_resolves_builtin_by_document_type_and_produces_pdf() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"type":"artigo"},"titulo":"Auto Resolved"}"#,
    );

    build_cmd(&home)
        .args(["--json", data.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Template: artigo"));

    let pdf = output.join("artigo.pdf");
    assert!(pdf.exists(), "PDF should exist at {pdf:?}");
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}

/// Explicit `document.template` overrides `document.type`. Requires tectonic.
#[test]
fn build_json_honours_explicit_document_template() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    seed_template(&templates, "carta", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    // document.type says "carta" but document.template pins to "artigo".
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"type":"carta","template":"artigo"},"titulo":"Pin"}"#,
    );

    build_cmd(&home)
        .args(["--json", data.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Template: artigo"));

    assert!(output.join("artigo.pdf").exists());
}

/// Case E of contracts/json-document-fields.md — no document field →
/// `MissingTypeField` (exit code 80). Does NOT require tectonic.
#[test]
fn build_json_without_document_field_exits_80() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"nope"}"#);

    build_cmd(&home)
        .args(["--json", data.to_str().unwrap()])
        .assert()
        .code(80)
        .stderr(predicate::str::contains("document.type"));
}

/// `document.type = "unknown"` with no matching built-in →
/// `NoTemplateMatch` (exit code 82). Does NOT require tectonic.
#[test]
fn build_json_unknown_document_type_exits_82_with_candidates() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    seed_template(&templates, "carta", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"type":"invoice"},"titulo":"nope"}"#,
    );

    build_cmd(&home)
        .args(["--json", data.to_str().unwrap()])
        .assert()
        .code(82)
        .stderr(predicate::str::contains("invoice"))
        .stderr(predicate::str::contains("artigo"))
        .stderr(predicate::str::contains("carta"));
}

/// Explicit `document.template = "acme/invoice"` — namespaced, but no
/// installed templates exist yet (US1 scope) → `ExplicitTemplateMissing`
/// (exit code 81). Does NOT require tectonic.
#[test]
fn build_json_explicit_third_party_missing_exits_81() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"template":"acme/invoice"}}"#,
    );

    build_cmd(&home)
        .args(["--json", data.to_str().unwrap()])
        .assert()
        .code(81)
        .stderr(predicate::str::contains("acme/invoice"));
}

/// `--json` and positional `template_name` are mutually exclusive — clap
/// should reject the combination with exit code 2 (usage error). Does
/// NOT require tectonic.
#[test]
fn build_json_conflicts_with_positional_template_name() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"x"}"#);

    build_cmd(&home)
        .args(["artigo", "--json", data.to_str().unwrap()])
        .assert()
        .failure();
}
